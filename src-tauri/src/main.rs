// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKitGTK compositing bug on Linux that causes blank screens.
    // https://github.com/tauri-apps/tauri/issues/5143
    // SAFETY: This is a single-threaded process startup routine.
    // The env var must be set before the WebKitGTK webview is initialized.
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    securitysmith_lib::run()
}
