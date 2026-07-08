// gen-man — Generate man pages from clap definitions using clap_mangen.
//
// Usage:
//   cargo run --bin gen-man -- --output dist/sm.1
//   cargo run --bin gen-man                 # writes to ./sm.1

#[path = "../cli_def.rs"]
mod cli_def;

use clap::CommandFactory;
use clap_mangen::Man;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

fn main() {
    let output = std::env::args()
        .skip_while(|a| a != "--output")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sm.1"));

    let cmd = cli_def::Cli::command();
    let mut buffer = Cursor::new(Vec::new());
    Man::new(cmd)
        .render(&mut buffer)
        .expect("failed to render man page");

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&output, buffer.into_inner()).expect("failed to write man page");
    println!("Generated: {}", output.display());
}
