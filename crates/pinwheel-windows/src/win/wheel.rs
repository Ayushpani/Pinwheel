//! The wheel's window: a layered, topmost, never-focused popup that is also
//! the drop target.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use tiny_skia::Pixmap;
use windows::core::{implement, Ref, Result};
use windows::Win32::Foundation::{HWND, POINT, POINTL, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, ReleaseStgMedium, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY,
    DROPEFFECT_NONE,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};

use super::{with_app, App};

/// Copies a premultiplied RGBA bitmap onto the layered window at `pos`.
pub fn present(hwnd: HWND, pixmap: &Pixmap, pos: (i32, i32)) {
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // Negative: rows run top to bottom, like the pixmap.
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        if let Ok(dib) = CreateDIBSection(Some(mem), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
            for (d, s) in dst.chunks_exact_mut(4).zip(pixmap.data().chunks_exact(4)) {
                d.copy_from_slice(&[s[2], s[1], s[0], s[3]]);
            }
            let old = SelectObject(mem, dib.into());
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                Some(screen),
                Some(&POINT { x: pos.0, y: pos.1 }),
                Some(&SIZE { cx: w, cy: h }),
                Some(mem),
                Some(&POINT { x: 0, y: 0 }),
                Default::default(),
                Some(&blend),
                ULW_ALPHA,
            );
            SelectObject(mem, old);
            let _ = DeleteObject(dib.into());
        }
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
    }
}

/// The file and folder paths in a drag, if it carries any.
fn paths(data: &IDataObject) -> Vec<PathBuf> {
    let format = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let mut out = Vec::new();
    unsafe {
        let Ok(mut medium) = data.GetData(&format) else {
            return out;
        };
        let drop = HDROP(medium.u.hGlobal.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        for i in 0..count {
            let len = DragQueryFileW(drop, i, None) as usize;
            let mut buf = vec![0u16; len + 1];
            DragQueryFileW(drop, i, Some(&mut buf));
            out.push(PathBuf::from(OsString::from_wide(&buf[..len])));
        }
        ReleaseStgMedium(&mut medium);
    }
    out
}

/// Accepts a copy only, so the source never deletes the original, even with
/// Shift held (which normally means "move").
fn set_effect(effect: *mut DROPEFFECT, accept: bool) {
    if effect.is_null() {
        return;
    }
    unsafe {
        *effect = if accept && (*effect & DROPEFFECT_COPY) == DROPEFFECT_COPY {
            DROPEFFECT_COPY
        } else {
            DROPEFFECT_NONE
        };
    }
}

#[implement(IDropTarget)]
pub struct DropTarget;

impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(
        &self,
        data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        let files = data.ok().map(paths).unwrap_or_default();
        let accept = with_app(|app| {
            app.enter(files);
            app.hover((pt.x, pt.y))
        });
        set_effect(effect, accept.unwrap_or(false));
        Ok(())
    }

    fn DragOver(
        &self,
        _keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        let accept = with_app(|app| app.hover((pt.x, pt.y)));
        set_effect(effect, accept.unwrap_or(false));
        Ok(())
    }

    fn DragLeave(&self) -> Result<()> {
        with_app(App::hide);
        Ok(())
    }

    fn Drop(
        &self,
        _data: Ref<'_, IDataObject>,
        _keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> Result<()> {
        let accepted = with_app(|app| app.drop_at((pt.x, pt.y)));
        set_effect(effect, accepted.unwrap_or(false));
        Ok(())
    }
}

pub fn target() -> IDropTarget {
    DropTarget.into()
}
