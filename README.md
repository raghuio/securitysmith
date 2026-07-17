# SecuritySmith

A command-line tool for security consultants and penetration testers to manage client work using markdown files.

All data lives in the filesystem as Markdown and TOML files. No database, no UI, no network calls, no telemetry.

> **Warning:** This project is under active development and is not ready for production use. Feedbacks are welcome.

## Install

```sh
./scripts/build.sh --install
```

This builds the release binary and installs `sm` to `~/.cargo/bin/`. Make sure `~/.cargo/bin` is in your `PATH`.

## Quick start

```sh
sm new                         # create a workspace
sm new acme/web/initial        # create client, project, and engagement
sm finding acme/web/initial --title "Stored XSS"
sm ls acme/web/initial --findings
sm status                      # see active engagements across all workspaces
sm check                       # workspace health check
```

## Documentation

Full usage guides in [`docs/`](docs/README.md). Run `sm --help` for command reference.

## License

Private project.