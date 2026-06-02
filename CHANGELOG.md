# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Encrypted local vault using SQLCipher with Argon2id-derived master key
- Recovery phrase (BIP-39) for vault recovery and master-password rotation
- Theme system: light / dark / custom accent with CSS variable tokens (`src/theme/`)
- Client management: create, read, update, delete, list, dashboard stats
- Engagement management: full CRUD, status transitions, archive, gating
- Credential manager with AES-GCM encryption at rest
- Finding management: severity, CVSS, duplicate, archive, counts
- Template library: OWASP Web, OWASP API, OWASP LLM, custom templates
- Report builder with PDF generation via `printpdf`
- Document builder with placeholder rendering
- Invoice builder with line items and PDF generation
- Immutable activity log / audit trail (`audit_log` table)
- Calendar events and reminder system with dismissed-reminder tracking
- News feed with RSS aggregation, refresh, and client-alert keyword matching
- AI chat assistant via local Ollama with tool-call approval flow
- Email composer with SMTP connection test and send
- Evidence attachments: upload, gallery, thumbnails, rename, reorder, storage tracking
- Client contacts (multiple contacts per client)
- Retest remediation: create retest engagements, compare findings, bulk status update, overdue tracking
- Scope & asset management: CRUD, bulk import, text export
- Time tracking: entries, weekly summary, budget status, invoice-from-time
- Analytics dashboard: severity distribution, top categories, findings over time, remediation rate, revenue, engagement timeline, time by activity, budget vs actual
- Methodology checklists with engagement assignment and coverage tracking
- Global full-text search with index rebuild command
- Notification center with dismiss
- Compliance mapping: frameworks, controls, finding mappings, engagement coverage
- Data portability: encrypted/unencrypted JSON export, preview import, execute import

### Changed

- (nothing released yet)

### Deprecated

- (nothing released yet)

### Removed

- (nothing released yet)

### Fixed

- (nothing released yet)

### Security

- (nothing released yet)
