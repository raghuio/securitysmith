# SecuritySmith

SecuritySmith is a open-source desktop application for security consultants and penetration testers to manage consulting engagements end to end — from initial client contact and scope definition to findings collection, report generation, and retest tracking. All data stays encrypted on your local machine. No servers. No hosting.

> ⚠️ This is a **Work in Progress** project. Core features are being built incrementally and the codebase is not yet ready for production use.

## What SecuritySmith Is

A **opinionated desktop application** for security consultants and penetration testers to track their security journey end to end — from initial client contact and scope definition to findings collection, report generation, and retest tracking. All data stays encrypted on your local machine. AI assistance via local Ollama(initially).

## What SecuritySmith Is Not

This project is deliberately **not** trying to be everything to everyone. We are focused and opinionated. SecuritySmith is **not**:

- A multi-user web application or SaaS — single user, single device, period
- A cloud-hosted service — your data never leaves your machine by design
- A generic project management tool like Jira or Trello — we serve security consultants specifically
- A vulnerability scanner — we import and manage findings, we do not run scans
- A password manager or credential vault
- A social network, bug bounty marketplace, or collaboration platform — this is your private workspace
- A "chat with AI" — AI is an assistant for your real work, not the product

## Feature Roadmap & Status

| Feature | Status | Description |
|---------|--------|-------------|
| Encrypted local vault (SQLCipher) | 🚧 In Progress | All data encrypted at rest with your master password |
| User profile & brand settings | 🚧 In Progress | Configure your identity, company, themes, and AI preferences |
| Client & engagement management | 📋 Planned | Track clients and active/past security engagements |
| Requirements & scope drafting | 📋 Planned | Capture requirements, generate SOW documents with AI help |
| Finding import from tools | 📋 Planned | Import from Nessus, Burp, ZAP, Nmap, and more |
| Report generation (PDF, DOCX, HTML, Markdown) | 📋 Planned | Generate polished client reports from templates |
| AI assistant via Ollama | 📋 Planned | Local AI helps draft scope, write findings, suggest remediation |
| Bug bounty tracking & stats | 📋 Planned | Track your bug bounty submissions, payouts, and success rates |
| Audit trail / changelog | 🚧 In Progress | Every change logged — immutable history of your work |
| Hostable mode | 📋 Later | Optional self-hosted mode for teams (not now, not Phase 0) |

> **We welcome feedback.** If you are a security consultant or pentester and these features matter to you, please open an issue and tell us about your workflow. We build what you need.

## Philosophy

- **Privacy first.** No external APIs for data processing. No telemetry. No cloud sync.
- **Local data ownership.** You own your data, your reports, your findings. Always.
- **AI assisted, not AI driven.** The consultant is in charge. AI as assistance, not a replacement.
- **Stable at scale.** From 1 client to 10,000 — same performance, same reliability.
