//! The notification-area icon: its menu, tooltip and result notifications.

use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_INFO,
    NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFYICONDATAW,
    NOTIFYICONDATAW_0, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, LoadImageW, HICON, IMAGE_ICON, LR_DEFAULTCOLOR, SM_CXSMICON, SM_CYSMICON,
};

use super::WM_APP_TRAY;

const ID: u32 = 1;

/// Copies `s` into a fixed UTF-16 field, cut to fit, always terminated.
fn fill<const N: usize>(field: &mut [u16; N], s: &str) {
    let mut units: Vec<u16> = s.encode_utf16().collect();
    if units.len() > N - 1 {
        units.truncate(N - 2);
        units.push('…' as u16);
    }
    field[..units.len()].copy_from_slice(&units);
    field[units.len()] = 0;
}

fn base(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID,
        ..Default::default()
    }
}

/// The app icon from the .exe's resources, at the small-icon size.
pub fn icon() -> HICON {
    unsafe {
        let module =
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap_or_default();
        LoadImageW(
            Some(module.into()),
            PCWSTR(1 as *const u16),
            IMAGE_ICON,
            GetSystemMetrics(SM_CXSMICON),
            GetSystemMetrics(SM_CYSMICON),
            LR_DEFAULTCOLOR,
        )
        .map(|h| HICON(h.0))
        .unwrap_or_default()
    }
}

pub fn add(hwnd: HWND, tip: &str) {
    let mut data = base(hwnd);
    data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = WM_APP_TRAY;
    data.hIcon = icon();
    fill(&mut data.szTip, tip);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &data);
        data.Anonymous = NOTIFYICONDATAW_0 {
            uVersion: NOTIFYICON_VERSION_4,
        };
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
    }
}

pub fn set_tip(hwnd: HWND, tip: &str) {
    let mut data = base(hwnd);
    data.uFlags = NIF_TIP | NIF_SHOWTIP;
    fill(&mut data.szTip, tip);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

/// A Windows notification from the tray icon.
pub fn notify(hwnd: HWND, title: &str, message: &str, warning: bool) {
    let mut data = base(hwnd);
    data.uFlags = NIF_INFO;
    data.dwInfoFlags = if warning { NIIF_WARNING } else { NIIF_INFO };
    fill(&mut data.szInfoTitle, title);
    fill(&mut data.szInfo, message);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

pub fn remove(hwnd: HWND) {
    let data = base(hwnd);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}
