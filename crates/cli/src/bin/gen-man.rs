// gen-man — Generate man pages from clap definitions using clap_mangen.
//
// Generates one page per command:
//   sm.1          — main page (overview, lists all commands)
//   sm-new.1      — sm new
//   sm-ls.1       — sm ls
//   sm-show.1     — sm show
//   ...etc.
//
// Usage:
//   cargo run --bin gen-man -- --output dist    # writes all pages to dist/
//   cargo run --bin gen-man                     # writes to ./man/

#[path = "../cli_def.rs"]
mod cli_def;

use clap::CommandFactory;
use clap_mangen::Man;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

fn main() {
    let output_dir = std::env::args()
        .skip_while(|a| a != "--output")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("man"));

    fs::create_dir_all(&output_dir).expect("failed to create output directory");

    let cmd = cli_def::Cli::command();

    // Render the main sm(1) page
    render_page(&cmd, &output_dir.join("sm.1"), "SM");

    // Render one page per subcommand
    for sub in cmd.get_subcommands().filter(|s| !s.is_hide_set()) {
        let name = sub.get_name();
        let filename = format!("sm-{}.1", name);
        let title = format!("SM-{}", name.to_uppercase());
        render_page(sub, &output_dir.join(&filename), &title);
    }

    println!("Generated man pages in: {}", output_dir.display());
}

fn render_page(cmd: &clap::Command, path: &Path, title: &str) {
    let mut buffer = Cursor::new(Vec::new());
    Man::new(cmd.clone())
        .title(title)
        .render(&mut buffer)
        .expect("failed to render man page");

    fs::write(path, buffer.into_inner()).expect("failed to write man page");
    println!("  {}", path.display());
}
