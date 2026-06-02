# Email Integration

## SMTP Setup

1. Go to **Settings → Email**.
2. Enter:
   - SMTP host (e.g., `smtp.gmail.com`)
   - Port (usually `587`)
   - Username and password
   - TLS toggle (recommended)
   - From address
3. Click **Test Connection** to verify.

## Sending Emails

- Use the **Email Composer** from any page.
- Select a template from the Templates library to pre-fill subject and body.
- Attach PDFs (reports, invoices, SOWs) directly from the app.
- Choose recipients from the client's contact list.

## Follow-up Reminders

Configure reminder intervals in **Settings → Email**:
- **Feedback reminder**: default 7 days after engagement completion
- **Retest reminder**: default 90 days after engagement completion

Reminders appear as in-app notifications. You decide whether to send them — nothing is sent automatically.

## Privacy

All emails are sent through **your** SMTP server. No email content is sent to external APIs or telemetry servers.
