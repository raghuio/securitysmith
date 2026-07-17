# Credentials

## `sm credential` — encrypted credential store

Credentials (URLs, usernames, passwords, API keys, VPN configs, SSH keys) are stored in an encrypted file (`.credentials.enc`) in the workspace. Encryption uses ChaCha20-Poly1305 with Argon2id key derivation from a master password.

The master password is required for every credential operation. It is never stored.

### Add a credential

```sh
sm credential acme/web_app/initial --add --label "Admin account" --type username_password
```

You'll be prompted for:
1. Master password
2. Credential value (the password, key, or URL)

Credential types: `url`, `username_password`, `api_key`, `bearer_token`, `vpn_config`, `ssh_key`, `custom`.

### List credentials for an engagement

```sh
sm credential acme/web_app/initial --list
```

Shows ID, label, type, and status. Credential values are not shown.

### Show a full credential (including value)

```sh
sm credential CRED-001 --show
```

You'll be prompted for the master password. The full credential including the value is displayed.

### Update credential status

```sh
sm credential CRED-001 --status working
```

Valid statuses: `not_verified`, `working`, `not_working`, `expired`.

### Remove a credential

```sh
sm credential CRED-001 --rm
```

### Integrity check

`sm check` verifies the credential store's magic header to confirm the file is intact.