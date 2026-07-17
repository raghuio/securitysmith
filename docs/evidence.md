# Evidence

## `sm evidence` — manage evidence files

Evidence files (screenshots, captures, logs, PoC scripts) are stored under `evidence/` inside the engagement directory.

### Add an evidence file

```sh
sm evidence acme/web_app/initial --add ~/screenshots/xss_proof.png
```

The file is copied into the engagement's `evidence/` directory with a sanitized filename. A SHA-256 hash is computed and recorded in the evidence index.

**Evidence is stored in plaintext.** Use OS-level disk encryption (LUKS, FileVault) or tools like `gocryptfs` for sensitive files. `sm check` scans text evidence files for common secret patterns and warns you.

### List evidence files

```sh
sm evidence acme/web_app/initial --list
```

Shows filename, size, SHA-256 hash, and date added.

### Show or open an evidence file

```sh
sm evidence acme/web_app/initial --show xss_proof.png
```

Opens the file in `$EDITOR`.

### Remove evidence

Evidence files are removed through `sm rm`:

```sh
sm rm acme/web_app/initial/evidence/xss_proof.png --yes
```

### Hash verification

`sm check` verifies that evidence file hashes match what was recorded. If a file was modified after being added, the check reports a mismatch.