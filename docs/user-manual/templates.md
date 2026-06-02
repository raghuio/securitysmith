# Templates

## Finding Templates

SecuritySmith ships with 30 built-in OWASP finding templates:
- OWASP Top 10:2025 Web (10 categories)
- OWASP API Security Top 10:2023 (10 categories)
- OWASP Top 10 for LLMs:2025 (10 categories)

### Applying a Template

1. Open an engagement and click **New finding**.
2. Select a template from the **Apply Template** dropdown.
3. The form pre-fills: title, severity, overview, summary, impact, remediation, and references.
4. Customize the finding for your specific engagement and save.

### Creating Custom Templates

1. Go to **Templates** in the sidebar.
2. Click **New template**.
3. Choose category: `finding`, `requirements`, `checklist`, `email`, `status_report`, or `engagement_status`.
4. Fill in name, subcategory, and content (JSON for findings, markdown for others).
5. Save.

### Saving a Finding as a Template

After writing a great finding, click **Save as template** in the finding form to add it to your custom library.

### Built-in vs Custom

- Built-in templates are **read-only**. Duplicate them to create an editable copy.
- Custom templates can be edited, duplicated, or deleted at any time.
