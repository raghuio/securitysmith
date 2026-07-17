# Requirements

## `sm req` — manage requirements

### Create a requirement

```sh
sm req acme/web_app/initial --title "Test all auth flows"
```

This creates the requirement file and opens it in `$EDITOR`. Skip the template:

```sh
sm req acme/web_app/initial --title "Test all auth flows" --no-template
```

### Show a requirement

```sh
sm req REQ-001
```

### Update requirement status

```sh
sm req REQ-001 --status verified
```

Valid statuses: `open`, `in_progress`, `verified`, `rejected`, `deferred`.

### Export a requirement

```sh
sm req REQ-001 --export html
sm req REQ-001 --export pdf --to ~/reports/req.pdf
sm req REQ-001 --export json
```

Formats: `markdown`, `html`, `pdf`, `json`. PDF requires `--to <path>`.