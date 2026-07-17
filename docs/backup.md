# Backup & Version Control

SecuritySmith stores everything as plain files. Back up and version control work with standard tools.

## Backup with tar

```sh
# Full backup
tar czf backup.tar.gz /path/to/workspace

# Restore
tar xzf backup.tar.gz -C /path/to/restore
```

## Version control with git

```sh
cd /path/to/workspace
git init
git add .
git commit -m "initial workspace"
```

Since everything is Markdown and TOML, `git diff` shows meaningful changes. Track finding updates, scope changes, and config edits over time.

## Note on encrypted credentials

The credential store (`.credentials.enc`) is a binary file. It works fine with `tar` and `git`, but `git diff` won't show readable changes. The file is encrypted — that's expected.