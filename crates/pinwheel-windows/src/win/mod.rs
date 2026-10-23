//! The Windows app: a hidden main window that owns the tray icon and gets
//! the hooks' messages, and the wheel window that is the drop target.
//! Everything runs on one thread; only conversions run on worker threads.

mod hooks;
mod log;
mod system;
mod tray;
mod wheel;

use std::cell::RefCell;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{
    IDropTarget, OleInitialize, OleUninitialize, RegisterDragDrop, RevokeDragDrop,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::{NIN_BALLOONUSERCLICK, NIN_SELECT};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, KillTimer, MessageBoxW, PostMessageW,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetTimer,
    SetWindowPos, ShowWindow, TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx, HWND_TOPMOST,
    MA_NOACTIVATE, MB_ICONINFORMATION, MB_OK, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
    MF_STRING, MF_UNCHECKED, MSG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    SW_SHOWNOACTIVATE, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WM_APP,
    WM_CONTEXTMENU, WM_DESTROY, WM_MOUSEACTIVATE, WM_NULL, WM_TIMER, WM_USER, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
};

use crate::geometry::{hit, place, Hit, SIZE};
use crate::job::{self, Outcome, Summary};
use crate::prefs::{self, Prefs, SaveTo};
use crate::render::{render, Fonts, Theme, View};
use crate::slices::{image_ring, Slice};

pub(crate) const WM_APP_TRAY: u32 = WM_APP + 1;
pub(crate) const WM_APP_TRIGGER: u32 = WM_APP + 2;
pub(crate) const WM_APP_BUTTON_UP: u32 = WM_APP + 3;
pub(crate) const WM_APP_ESCAPE: u32 = WM_APP + 4;
pub(crate) const WM_APP_DONE: u32 = WM_APP + 5;

/// After the mouse button comes up, wait this long for a drop before
/// closing the wheel.
const TIMER_HIDE: usize = 1;
const HIDE_DELAY_MS: u32 = 400;

/// The tray icon chosen with the keyboard (NIN_SELECT | NINF_KEY).
const NIN_KEYSELECT: u32 = WM_USER + 1;

const TIP: &str = "Pinwheel: drag files, press Shift";

const CMD_BESIDE: usize = 10;
const CMD_SUBFOLDER: usize = 11;
const CMD_FOLDER: usize = 12;
const CMD_ZIP: usize = 20;
const CMD_STARTUP: usize = 30;
const CMD_LAST: usize = 40;
const CMD_HELP: usize = 41;
const CMD_QUIT: usize = 99;

const HELP: &str = "1. Start dragging an image, several images or a folder.\n\
2. While still dragging, press Shift. The wheel appears at the pointer.\n\
3. Drop on a slice: JPG, PNG, WebP, ICO, TIFF, 50%, 1920 px or 500 KB.\n\n\
Results are saved next to the originals unless you choose otherwise. Originals are never changed. \
Move away from the wheel, or press Esc, to close it and carry on with your drag.\n\n\
Right-click the Pinwheel icon next to the clock for options.";

pub(crate) struct App {
    main: HWND,
    wheel: HWND,
    fonts: Fonts,
    theme: Theme,
    ring: Vec<Slice>,
    prefs: Prefs,
    prefs_path: PathBuf,
    visible: bool,
    pos: (i32, i32),
    scale: f32,
    files: Vec<PathBuf>,
    summary: Summary,
    hovered: Hit,
    busy: usize,
    last: Option<PathBuf>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` on the app. Returns `None` if the app is not set up or is
/// already in use further up the stack.
pub(crate) fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

impl App {
    fn show(&mut self) {
        if self.visible {
            return;
        }
        let mut pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut pt);
        }
        let (work, scale) = system::monitor_at(pt);
        self.theme = system::theme();
        self.scale = scale;
        let size = (SIZE * scale).round() as i32;
        self.pos = place(
            (pt.x, pt.y),
            size,
            (work.left, work.top, work.right, work.bottom),
        );
        self.files.clear();
        self.summary = Summary {
            title: "Pinwheel".into(),
            detail: "Move onto a slice".into(),
            usable: true,
        };
        self.hovered = Hit::Hub;
        log::write(&format!("wheel shown at {:?}, scale {}", self.pos, scale));
        self.redraw();
        unsafe {
            let _ = ShowWindow(self.wheel, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                self.wheel,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        self.visible = true;
    }

    fn redraw(&self) {
        let labels: Vec<&str> = self.ring.iter().map(|s| s.label).collect();
        let detail = match self.hovered {
            Hit::Slice(i) if self.summary.usable => self.ring[i].hint,
            _ => self.summary.detail.as_str(),
        };
        let view = View {
            labels: &labels,
            hovered: self.hovered,
            hub_title: &self.summary.title,
            hub_detail: detail,
            disabled: !self.summary.usable,
        };
        let pixmap = render(&view, &self.theme, &self.fonts, self.scale);
        wheel::present(self.wheel, &pixmap, self.pos);
    }

    fn hit_at(&self, pt: (i32, i32)) -> Hit {
        let dx = (pt.0 - self.pos.0) as f32 / self.scale - SIZE / 2.0;
        let dy = (pt.1 - self.pos.1) as f32 / self.scale - SIZE / 2.0;
        hit(dx, dy)
    }

    pub(crate) fn enter(&mut self, files: Vec<PathBuf>) {
        if !self.visible {
            return;
        }
        self.summary = job::summarise(&files);
        log::write(&format!(
            "drag entered: {} item(s), usable: {}",
            files.len(),
            self.summary.usable
        ));
        self.files = files;
        self.redraw();
    }

    /// Updates the hovered slice. Returns whether a drop here would be taken.
    pub(crate) fn hover(&mut self, pt: (i32, i32)) -> bool {
        if !self.visible {
            return false;
        }
        let h = self.hit_at(pt);
        if h != self.hovered {
            self.hovered = h;
            self.redraw();
        }
        matches!(h, Hit::Slice(_)) && self.summary.usable && !self.files.is_empty()
    }

    pub(crate) fn hide(&mut self) {
        if self.visible {
            unsafe {
                let _ = ShowWindow(self.wheel, SW_HIDE);
            }
            self.visible = false;
            self.hovered = Hit::Outside;
        }
    }

    /// Runs the slice under `pt` on the dragged files. Returns whether the
    /// drop was taken.
    pub(crate) fn drop_at(&mut self, pt: (i32, i32)) -> bool {
        let taken = self.visible && self.hover(pt);
        let h = self.hit_at(pt);
        let files = std::mem::take(&mut self.files);
        self.hide();
        if let (true, Hit::Slice(i)) = (taken, h) {
            log::write(&format!("dropped on {}", self.ring[i].label));
            self.start(files, self.ring[i].clone());
        }
        taken
    }

    fn start(&mut self, files: Vec<PathBuf>, slice: Slice) {
        self.busy += 1;
        let count = files.len();
        tray::set_tip(
            self.main,
            &format!(
                "Pinwheel: working on {count} item{}…",
                if count == 1 { "" } else { "s" }
            ),
        );
        let prefs = self.prefs.clone();
        let main = self.main.0 as isize;
        std::thread::spawn(move || {
            let outcome = job::run(&files, &slice, &prefs);
            let ptr = Box::into_raw(Box::new(outcome));
            let hwnd = HWND(main as *mut _);
            if unsafe { PostMessageW(Some(hwnd), WM_APP_DONE, WPARAM(0), LPARAM(ptr as isize)) }
                .is_err()
            {
                // The app is closing; nobody will read it.
                drop(unsafe { Box::from_raw(ptr) });
            }
        });
    }

    fn done(&mut self, outcome: Outcome) {
        log::write(&format!(
            "{}: {}",
            outcome.title,
            outcome.message.replace('\n', " / ")
        ));
        self.busy = self.busy.saturating_sub(1);
        if self.busy == 0 {
            tray::set_tip(self.main, TIP);
        }
        if outcome.reveal.is_some() {
            self.last = outcome.reveal.clone();
        }
        tray::notify(self.main, &outcome.title, &outcome.message, !outcome.ok);
    }

    fn save_prefs(&self) {
        if let Err(e) = self.prefs.save(&self.prefs_path) {
            tray::notify(self.main, "Settings not saved", &e.to_string(), true);
        }
    }
}

fn reveal(path: &Path) {
    let _ = std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn();
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn message(text: &str) {
    log::write(text);
    let text = wide(text);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("Pinwheel"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

fn show_menu(hwnd: HWND, x: i32, y: i32) {
    let Some((prefs, last)) = with_app(|a| (a.prefs.clone(), a.last.clone())) else {
        return;
    };
    let startup = system::starts_with_windows();
    let check = |on: bool| if on { MF_CHECKED } else { MF_UNCHECKED };
    let folder_label = wide(&match &prefs.save_to {
        SaveTo::Folder(p) => format!("In {}", p.display()),
        _ => "In a folder I choose…".to_string(),
    });
    let cmd = unsafe {
        let (Ok(menu), Ok(save)) = (CreatePopupMenu(), CreatePopupMenu()) else {
            return;
        };
        let _ = AppendMenuW(
            save,
            MF_STRING | check(prefs.save_to == SaveTo::Beside),
            CMD_BESIDE,
            w!("Next to the original"),
        );
        let _ = AppendMenuW(
            save,
            MF_STRING | check(prefs.save_to == SaveTo::Subfolder),
            CMD_SUBFOLDER,
            w!("In a \"Pinwheel\" subfolder"),
        );
        let _ = AppendMenuW(
            save,
            MF_STRING | check(matches!(prefs.save_to, SaveTo::Folder(_))),
            CMD_FOLDER,
            PCWSTR(folder_label.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            w!("Drag files, press Shift, drop on a slice"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_POPUP, save.0 as usize, w!("Save results"));
        let _ = AppendMenuW(
            menu,
            MF_STRING | check(prefs.zip_several),
            CMD_ZIP,
            w!("Also make a ZIP for several files"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | check(startup),
            CMD_STARTUP,
            w!("Start with Windows"),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        if last.is_some() {
            let _ = AppendMenuW(menu, MF_STRING, CMD_LAST, w!("Show last result"));
        }
        let _ = AppendMenuW(menu, MF_STRING, CMD_HELP, w!("How to use Pinwheel"));
        let _ = AppendMenuW(menu, MF_STRING, CMD_QUIT, w!("Quit Pinwheel"));
        // Without this the menu does not close when clicking elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            x,
            y,
            None,
            hwnd,
            None,
        )
        .0 as usize;
        let _ = DestroyMenu(menu);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        cmd
    };
    match cmd {
        CMD_BESIDE | CMD_SUBFOLDER => {
            with_app(|a| {
                a.prefs.save_to = if cmd == CMD_BESIDE {
                    SaveTo::Beside
                } else {
                    SaveTo::Subfolder
                };
                a.save_prefs();
            });
        }
        CMD_FOLDER => {
            if let Some(dir) = system::pick_folder(hwnd) {
                with_app(|a| {
                    a.prefs.save_to = SaveTo::Folder(dir);
                    a.save_prefs();
                });
            }
        }
        CMD_ZIP => {
            with_app(|a| {
                a.prefs.zip_several = !a.prefs.zip_several;
                a.save_prefs();
            });
        }
        CMD_STARTUP => system::set_start_with_windows(!startup),
        CMD_LAST => {
            if let Some(p) = last {
                reveal(&p);
            }
        }
        CMD_HELP => message(HELP),
        CMD_QUIT => unsafe {
            let _ = DestroyWindow(hwnd);
        },
        _ => {}
    }
}

fn taskbar_created() -> u32 {
    static MSG: OnceLock<u32> = OnceLock::new();
    *MSG.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

unsafe extern "system" fn main_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_APP_TRIGGER => {
            log::write("trigger: Shift during a drag");
            with_app(App::show);
        }
        WM_APP_BUTTON_UP => unsafe {
            SetTimer(Some(hwnd), TIMER_HIDE, HIDE_DELAY_MS, None);
        },
        WM_TIMER if wparam.0 == TIMER_HIDE => {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_HIDE);
            }
            with_app(App::hide);
        }
        WM_APP_ESCAPE => {
            with_app(App::hide);
        }
        WM_APP_DONE => {
            let outcome = unsafe { Box::from_raw(lparam.0 as *mut Outcome) };
            with_app(|a| a.done(*outcome));
        }
        WM_APP_TRAY => {
            let event = (lparam.0 & 0xFFFF) as u32;
            let x = (wparam.0 & 0xFFFF) as i16 as i32;
            let y = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
            match event {
                WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT => show_menu(hwnd, x, y),
                NIN_BALLOONUSERCLICK => {
                    if let Some(Some(p)) = with_app(|a| a.last.clone()) {
                        reveal(&p);
                    }
                }
                _ => {}
            }
        }
        WM_DESTROY => unsafe { PostQuitMessage(0) },
        m if m == taskbar_created() => tray::add(hwnd, TIP),
        _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
    LRESULT(0)
}

unsafe extern "system" fn wheel_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Clicking the wheel never takes focus from the app being dragged from.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Starts Pinwheel and runs until Quit.
pub fn run() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    // One Pinwheel at a time.
    let Ok(_instance) = (unsafe { CreateMutexW(None, true, w!("Local\\Pinwheel.SingleInstance")) })
    else {
        return;
    };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        message("Pinwheel is already running. Look for its icon next to the clock.");
        return;
    }
    log::start();
    if unsafe { OleInitialize(None) }.is_err() {
        message(
            "Pinwheel could not start drag and drop on this PC. Restart Windows and try again.",
        );
        return;
    }
    let Some(fonts) = system::fonts() else {
        message("Pinwheel needs the Segoe UI or Arial font from C:\\Windows\\Fonts, and could not find it.");
        return;
    };

    let instance = unsafe { GetModuleHandleW(None) }.unwrap_or_default();
    let main_class = WNDCLASSW {
        lpfnWndProc: Some(main_proc),
        hInstance: instance.into(),
        lpszClassName: w!("PinwheelMain"),
        ..Default::default()
    };
    let wheel_class = WNDCLASSW {
        lpfnWndProc: Some(wheel_proc),
        hInstance: instance.into(),
        lpszClassName: w!("PinwheelWheel"),
        ..Default::default()
    };
    let windows = unsafe {
        RegisterClassW(&main_class);
        RegisterClassW(&wheel_class);
        let main = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("PinwheelMain"),
            w!("Pinwheel"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        );
        let wheel = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("PinwheelWheel"),
            w!("Pinwheel"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        );
        main.and_then(|m| wheel.map(|w| (m, w)))
    };
    let Ok((main, wheel_hwnd)) = windows else {
        message("Pinwheel could not create its windows.");
        return;
    };

    let target: IDropTarget = wheel::target();
    if unsafe { RegisterDragDrop(wheel_hwnd, &target) }.is_err() {
        message("Pinwheel could not accept drops. Restart Windows and try again.");
        return;
    }
    let hooks = match hooks::install(main) {
        Ok(h) => h,
        Err(_) => {
            message(
                "Pinwheel could not watch for the Shift key. Some security software blocks this.",
            );
            return;
        }
    };

    log::write("ready: tray icon, drop target and hooks in place");
    let prefs_path = prefs::default_path();
    let mut prefs = Prefs::load(&prefs_path);
    tray::add(main, TIP);
    if !prefs.welcomed {
        tray::notify(
            main,
            "Pinwheel is ready",
            "Drag an image or a folder, press Shift while dragging, and drop it on a slice. Right-click this icon for options.",
            false,
        );
        prefs.welcomed = true;
        let _ = prefs.save(&prefs_path);
    }

    APP.with(|cell| {
        *cell.borrow_mut() = Some(App {
            main,
            wheel: wheel_hwnd,
            fonts,
            theme: system::theme(),
            ring: image_ring(),
            prefs,
            prefs_path,
            visible: false,
            pos: (0, 0),
            scale: 1.0,
            files: Vec::new(),
            summary: Summary {
                title: String::new(),
                detail: String::new(),
                usable: false,
            },
            hovered: Hit::Outside,
            busy: 0,
            last: None,
        })
    });

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        log::write("quit");
        tray::remove(main);
        let _ = UnhookWindowsHookEx(hooks.0);
        let _ = UnhookWindowsHookEx(hooks.1);
        let _ = RevokeDragDrop(wheel_hwnd);
        drop(target);
        OleUninitialize();
    }
}
