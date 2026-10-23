//! Watches the mouse and keyboard, system-wide, for the trigger: Shift
//! pressed while the left button is held and has moved, which is a drag in
//! progress. The hooks only post messages; they never block or swallow input.

use std::cell::Cell;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_LSHIFT, VK_RSHIFT, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetSystemMetrics, PostMessageW, SetWindowsHookExW, HC_ACTION, HHOOK,
    KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, SM_CXDRAG, SM_CYDRAG, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN,
    WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::{WM_APP_BUTTON_UP, WM_APP_ESCAPE, WM_APP_TRIGGER};

thread_local! {
    static TARGET: Cell<isize> = const { Cell::new(0) };
    static DOWN: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
    static DRAGGING: Cell<bool> = const { Cell::new(false) };
    static SHIFT: Cell<bool> = const { Cell::new(false) };
}

fn post(msg: u32) {
    let hwnd = HWND(TARGET.get() as *mut _);
    let _ = unsafe { PostMessageW(Some(hwnd), msg, WPARAM(0), LPARAM(0)) };
}

unsafe extern "system" fn mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let (x, y) = (info.pt.x, info.pt.y);
        match wparam.0 as u32 {
            WM_LBUTTONDOWN => {
                DOWN.set(Some((x, y)));
                DRAGGING.set(false);
            }
            WM_MOUSEMOVE => {
                if let Some((dx, dy)) = DOWN.get() {
                    let (tx, ty) =
                        unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
                    if (x - dx).abs() > tx || (y - dy).abs() > ty {
                        DRAGGING.set(true);
                    }
                }
            }
            WM_LBUTTONUP => {
                DOWN.set(None);
                if DRAGGING.replace(false) {
                    post(WM_APP_BUTTON_UP);
                }
            }
            _ => {}
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn keyboard(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode as u16;
        let is_shift = vk == VK_LSHIFT.0 || vk == VK_RSHIFT.0 || vk == VK_SHIFT.0;
        match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN if is_shift => {
                // Only a fresh press counts: Shift held before the drag
                // began (selecting text, say) never opens the wheel.
                if !SHIFT.replace(true) && DRAGGING.get() {
                    post(WM_APP_TRIGGER);
                }
            }
            WM_KEYUP | WM_SYSKEYUP if is_shift => SHIFT.set(false),
            WM_KEYDOWN if vk == VK_ESCAPE.0 => post(WM_APP_ESCAPE),
            _ => {}
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Installs both hooks on this thread, posting to `target`. The thread must
/// run a message loop.
pub fn install(target: HWND) -> windows::core::Result<(HHOOK, HHOOK)> {
    TARGET.set(target.0 as isize);
    unsafe {
        let m = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), None, 0)?;
        let k = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), None, 0)?;
        Ok((m, k))
    }
}
