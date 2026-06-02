# Core Workflows

## Client → Engagement → Credentials → Findings → Report Lifecycle

This is the standard workflow for every security assessment.

### 1. Create a Client

- Go to **Dashboard**.
- Click **Add your first client** (or use `Ctrl+K` → "New client").
- Fill in: name, contact email, notes, tags.

### 2. Create an Engagement

- From the client row, click **Add engagement**.
- Fill in: name, target area (Web, API, Network, etc.), assessment kind, dates, scope.
- Set **scheduling gates** if needed:
  - `credentials_ready` — do you have working test credentials?
  - `payment_cleared` — is the advance invoice paid?

### 3. Store Credentials

- Open the engagement and go to **Credentials**.
- Add URLs, usernames/passwords, API keys, VPN configs, SSH keys.
- Mark each as `verified` and `working` when confirmed.
- When all credentials are working, the engagement gate auto-opens.

### 4. Record Findings

- Open the engagement and go to **Findings**.
- Click **New finding** (or apply a template from the Templates library).
- Fill in the full finding structure:
  - Title, severity, CVSS, OWASP category, CWE
  - Overview, summary, affected endpoints, evidence
  - Impact items, remediation items, steps to reproduce
  - References

### 5. Generate a Report

- Go to **Reports**.
- Create a new report linked to the engagement.
- Select which findings to include.
- Write the executive summary and appendix.
- Click **Generate PDF**.
- The PDF uses your brand settings (logo, colors, company name) automatically.

### 6. Send the Deliverable

- Go to **Email** (or use the email composer from any page).
- Attach the generated PDF.
- Select the recipient from the client's contact list.
- Send via your configured SMTP server.

### 7. Track Remediation

- After delivery, change engagement status to `completed`.
- Findings move to `reported` status.
- Set fix deadlines based on severity.
- Create a **Retest Engagement** later to verify fixes.
