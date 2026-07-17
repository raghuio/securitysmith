# Custom Documents

## `sm document` — manage custom documents

Custom documents are Markdown files for deliverables that aren't reports or SOWs: Rules of Engagement (RoE), NDAs, proposals, and custom documents.

### Create a document

```sh
sm document acme/web_app/initial --title "Rules of Engagement" --type roe
```

Document types: `roe`, `nda`, `proposal`, `custom`.

This creates the document file and opens it in `$EDITOR`.

### Show a document

```sh
sm document DOC-001
```

### Finalize (set read-only)

```sh
sm document DOC-001 --finalize
```

A finalized document cannot be edited. Use this when a document is signed or delivered.

### Unlock (revert to draft)

```sh
sm document DOC-001 --unlock
```

### Export a document

```sh
sm document DOC-001 --export html
sm document DOC-001 --export pdf --to ~/documents/roe.pdf
sm document DOC-001 --export json
```

Formats: `markdown`, `html`, `pdf`, `json`. PDF requires `--to <path>`.

### List documents in an engagement

```sh
sm ls acme/web_app/initial --documents
```

Shows document ID, type, and status.