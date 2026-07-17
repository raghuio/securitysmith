#![no_main]

use libfuzzer_sys::fuzz_target;
use securitysmith_workspace::typst_engine::markdown_to_typst;

// Fuzz the Markdown-to-Typst converter.
// Feed arbitrary bytes as Markdown input. The converter must not panic.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = markdown_to_typst(s);
    }
});