//! Window plumbing shared by MelCat's windows: class registration, the
//! instance handle and per-window state.

use std::cell::RefCell;
use std::rc::Rc;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, ERROR_CLASS_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, LoadCursorW, LoadIconW, RegisterClassExW, SetWindowLongPtrW, GWLP_USERDATA, IDC_ARROW,
    WNDCLASSEXW, WNDPROC,
};

/// Every MelCat window class name starts with this prefix.
pub(crate) const CLASS_PREFIX: &str = "MelCat.";

/// This module's instance handle.
pub(crate) fn instance() -> HINSTANCE {
    // SAFETY: GetModuleHandleW(None) returns the handle of the running executable.
    unsafe { GetModuleHandleW(None) }.map(HINSTANCE::from).unwrap_or_default()
}

/// Registers a window class (once; registering again is not an error). The
/// class gets the app icon (resource 1) and the arrow cursor.
pub(crate) fn register_class(class: PCWSTR, proc: WNDPROC, background: Option<HBRUSH>) -> windows::core::Result<()> {
    // SAFETY: the class name is a static string, the procedure a valid extern "system" fn.
    unsafe {
        let class_info = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: proc,
            hInstance: instance(),
            hIcon: LoadIconW(Some(instance()), PCWSTR(std::ptr::without_provenance(1))).unwrap_or_default(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: background.unwrap_or_default(),
            lpszClassName: class,
            ..Default::default()
        };
        if RegisterClassExW(&class_info) == 0 {
            let error = GetLastError();
            if error != ERROR_CLASS_ALREADY_EXISTS {
                return Err(windows::core::Error::from_hresult(error.to_hresult()));
            }
        }
    }
    Ok(())
}

/// Stores `state` in the window (one strong reference, released by [`detach`]).
pub(crate) fn attach<T: 'static>(hwnd: HWND, state: &Rc<RefCell<T>>) {
    let raw = Rc::into_raw(Rc::clone(state));
    // SAFETY: GWLP_USERDATA is free for the window's owner; `detach` reclaims the reference.
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
    }
}

/// The state stored by [`attach`], if any.
///
/// Each window class must always use the same `T` for its windows.
pub(crate) fn state<T: 'static>(hwnd: HWND) -> Option<Rc<RefCell<T>>> {
    // SAFETY: the pointer was produced by `Rc::into_raw::<RefCell<T>>` in `attach`
    // and stays valid until `detach`, which clears it first.
    unsafe {
        let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<T>;
        if raw.is_null() {
            return None;
        }
        Rc::increment_strong_count(raw);
        Some(Rc::from_raw(raw))
    }
}

/// Releases the state stored by [`attach`]. Call from `WM_NCDESTROY`.
pub(crate) fn detach<T: 'static>(hwnd: HWND) {
    // SAFETY: see `state`; the slot is cleared before the reference is released.
    unsafe {
        let raw = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *const RefCell<T>;
        if !raw.is_null() {
            drop(Rc::from_raw(raw));
        }
    }
}

/// Screen coordinates packed in a `WPARAM`/`LPARAM` (signed, like `GET_X_LPARAM`).
pub(crate) fn lparam_point(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam.0 & 0xFFFF) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_coordinates_are_signed() {
        assert_eq!(lparam_point(LPARAM(0x0020_0010)), (16, 32));
        assert_eq!(lparam_point(LPARAM(0xFFFE_FFFF)), (-1, -2));
    }

    #[test]
    fn the_prefix_names_melcat() {
        assert!(CLASS_PREFIX.starts_with("MelCat"));
    }
}
