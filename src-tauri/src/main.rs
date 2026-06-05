// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKitGTK compositing bug on Linux that causes blank screens.
    // https://github.com/tauri-apps/tauri/issues/5143
    // SAFETY: This is a single-threaded process startup routine (no other threads exist yet).
    // The env var must be set before the WebKitGTK webview is initialized.
    // `std::env::set_var` is unsafe in Rust 2024 edition because it can race with other threads;
    // here it is safe because we are pre-main and no other threads are running.
    // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage
    #[cfg(target_os = "linux")]
    unsafe { // nosemgrep
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    securitysmith_lib::run()
}
