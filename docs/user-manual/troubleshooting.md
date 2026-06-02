# Troubleshooting

## Forgot Master Password

If you forgot your password but have your **12-word recovery phrase**:
1. On the unlock screen, click **Use recovery phrase**.
2. Enter all 12 words in order.
3. The vault unlocks. Go to **Settings → Security → Change Master Password** to set a new password. The recovery phrase is regenerated as part of the change.

If you lost both password and recovery phrase, your data is **unrecoverable**.

## Vault Recovery

If the vault file becomes corrupted:
1. Close the app.
2. Check `~/.local/share/securitysmith/` for `vault.db` and `vault.db-wal`.
3. If WAL exists, the database may recover on next open.
4. If still corrupted, restore from a `.ssexport` backup file via **Import**.

## Resetting the App

To start completely fresh:
```bash
rm -rf ~/.local/share/securitysmith/
```
This deletes the vault, attachments, and all settings. **Cannot be undone.**

## Ollama Not Connecting

- Verify Ollama is running: `curl http://localhost:11434/api/tags`
- Check the URL in **Settings → AI**.
- Ensure no firewall is blocking localhost.

## SMTP Test Fails

- Double-check host, port, username, and password.
- For Gmail, use an **App Password**, not your main password.
- Enable TLS unless your server explicitly requires plain text.

## Build Issues

If `verify-all.sh` fails:
1. Run `cd src-tauri && cargo fmt` to fix Rust formatting.
2. Run `npx prettier --write 'src/**/*.{ts,tsx}'` to fix frontend formatting.
3. Run `npm audit fix` to resolve frontend dependency issues.
4. Run `cd src-tauri && cargo clippy` to see lint warnings.
