use tauri::{AppHandle, Manager};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::RemoteDesktop::{
        NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
    },
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{WM_NCDESTROY, WM_POWERBROADCAST, WM_SIZE},
    },
};
struct Context {
    app: AppHandle,
    owner: String,
}
unsafe extern "system" fn hook(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass: usize,
    data: usize,
) -> LRESULT {
    let context = unsafe { &*(data as *const Context) };
    if (message == 0x02b1 && wparam.0 == 7)
        || (message == WM_POWERBROADCAST && wparam.0 == 4)
        || (message == WM_SIZE && wparam.0 == 1)
    {
        crate::vault::revoke(&context.app, &context.owner);
        let app = context.app.clone();
        let owner = context.owner.clone();
        tauri::async_runtime::spawn(async move {
            crate::vault::lock(&app, &owner);
        });
    }
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if message == WM_NCDESTROY {
        unsafe {
            let _ = WTSUnRegisterSessionNotification(hwnd);
            let _ = RemoveWindowSubclass(hwnd, Some(hook), subclass);
            drop(Box::from_raw(data as *mut Context));
        }
    }
    result
}
pub fn install(app: &AppHandle, owner: &str) {
    let Some(window) = app.get_window(owner) else {
        return;
    };
    let Ok(hwnd) = window.hwnd() else { return };
    let context = Box::new(Context {
        app: app.clone(),
        owner: owner.into(),
    });
    let pointer = Box::into_raw(context);
    unsafe {
        if !SetWindowSubclass(hwnd, Some(hook), 0xAE20usize, pointer as usize).as_bool() {
            drop(Box::from_raw(pointer));
            return;
        }
        let _ = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
    }
}
