# Scope & Notes

## `sm scope` — edit or export scope

Open scope.md in `$EDITOR`:

```sh
sm scope acme/web_app/initial
```

On first use, a `scope.md` file is created from a template. A default `templates/scope.md` is created in the workspace if one doesn't exist. Edit that template file to change the default structure — no recompilation needed.

The default scope template has these sections: In Scope, Out of Scope, Rules of Engagement, Notes.

### Export scope

```sh
sm scope acme/web_app/initial --export html
sm scope acme/web_app/initial --export pdf --to ~/reports/scope.pdf
sm scope acme/web_app/initial --export markdown
```

Formats: `markdown`, `html`, `pdf`, `json`. PDF requires `--to <path>`.

## `sm note` — create or export notes

Create a quick note:

```sh
sm note acme/web_app/initial "Remember to test the API rate limits"
```

Notes are timestamped and stored under `notes/` in the engagement directory.

### Export all notes in an engagement

```sh
sm note acme/web_app/initial --export html
sm note acme/web_app/initial --export pdf --to ~/reports/notes.pdf
sm note acme/web_app/initial --export markdown
```

Formats: `markdown`, `html`, `pdf`, `json`. PDF requires `--to <path>`.