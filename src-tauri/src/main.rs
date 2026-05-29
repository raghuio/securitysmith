// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKitGTK compositing bug on Linux that causes blank screens.
    // https://github.com/tauri-apps/tauri/issues/5143
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    securitysmith_lib::run()
}
