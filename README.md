# SecuritySmith

SecuritySmith is an open-source command-line tool for security consultants and penetration testers. It helps you manage the full pentesting lifecycle — from client and project setup to scope capture, findings, report generation, and retest tracking — using plain text files on your local machine.

> ⚠️ This is a **Work in Progress** project. Core features are being rebuilt as a CLI, and the codebase is not yet ready for production use.

## What SecuritySmith Is

A **CLI-first tool** for security consultants and penetration testers. Your work lives in:

- **TOML files** for configuration and structured data
- **Markdown files with YAML frontmatter** for findings, notes, requirements, reports, and Statements of Work

Everything is stored locally, is git-friendly, and can be edited with any text editor.

## What SecuritySmith Is Not

- A desktop application with a graphical user interface — it is a command-line tool
- A multi-user web application or SaaS — single user, local machine
- A cloud-hosted service — your data never leaves your machine by design
- A generic project management tool like Jira or Trello — we serve security consultants specifically
- A vulnerability scanner — we import and manage findings, we do not run scans
- A password manager or credential vault (Phase 1; a secure credential store may be added later)
- A social network, bug bounty marketplace, or collaboration platform
- An AI chatbot — use your preferred coding agent or editor AI inside the project folder instead

## Quick Start

```bash
# Clone and build the project
git clone <repo-url>
cd securitysmith
cargo build --release

# Create a workspace and start tracking an engagement
securitysmith init ~/clients/acme-2026
cd ~/clients/acme-2026
securitysmith client add
securitysmith project add --client acme
securitysmith engagement add --client acme --project webapp
securitysmith finding add --engagement webapp
securitysmith report --pdf
```

## Commands

| Command | Purpose |
|---------|---------|
| `securitysmith init [path]` | Create a new workspace |
| `securitysmith client add` | Add a client |
| `securitysmith project add` | Add a project under a client |
| `securitysmith engagement add` | Add an engagement under a project |
| `securitysmith finding add` | Add a finding |
| `securitysmith requirement add` | Capture a requirement |
| `securitysmith scope add` | Add an in-scope or out-of-scope asset |
| `securitysmith report` | Generate a report |
| `securitysmith sow` | Generate a Statement of Work |
| `securitysmith validate` | Check workspace health |

## Project Structure

```
~/clients/acme-2026/
├── securitysmith.toml
└── clients/
    └── acme/
        ├── client.toml
        └── projects/
            └── webapp/
                └── engagements/
                    └── initial/
                        ├── engagement.toml
                        ├── requirements.toml
                        ├── scope.toml
                        ├── findings/
                        ├── notes/
                        ├── report/
                        └── sow/
```

## Feature Roadmap & Status

| Feature | Status | Description |
|---------|--------|-------------|
| CLI workspace and entity management | 🚧 In Progress | `init`, `client`, `project`, `engagement` commands |
| Requirements and scope capture | 🚧 In Progress | TOML-based requirements and asset tracking |
| Findings as Markdown with frontmatter | 🚧 In Progress | Write findings in Markdown, query them like data |
| Report generation (Markdown, PDF) | 📋 Planned | Generate client reports from findings and templates |
| Statement of Work generation | 📋 Planned | Generate SOWs from requirements and scope |
| Template library | 📋 Planned | Reusable Markdown templates |
| Methodology checklists | 📋 Later | Pass/fail/not-tested tracking |
| Time tracking | 📋 Later | Log hours and summaries |
| Encrypted credential store | 📋 Later | Secure local credential storage |
| Full-text search | 📋 Later | Search across the workspace |
| CLI dashboard | 📋 Later | Summary views from frontmatter data |
| Calendar views | 📋 Later | Timeline views from dates |
| Invoice generation | 📋 Later | Generate invoices from time data |

## Philosophy

- **Privacy first.** No external APIs for data processing. No telemetry. No cloud sync.
- **Local data ownership.** You own your data, your reports, your findings. Always.
- **Plain text by default.** Files you can read, diff, version, and back up with standard tools.
- **CLI driven.** Fast, keyboard-first workflows for consultants who live in terminals and editors.
- **Stable at scale.** From one engagement to thousands — same model, same reliability.
