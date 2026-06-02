# Invoicing

## Quotes and Invoices

SecuritySmith can generate quotes for approval and invoices for payment.

### Creating an Invoice

1. Go to **Invoices**.
2. Click **New invoice**.
3. Select the client and optionally the engagement.
4. Set document type: `quote` or `invoice`.
5. Add line items: description, quantity, rate. The total auto-calculates.
6. Set tax rate and discount (percentage or fixed).
7. Choose currency: USD, EUR, GBP, INR, AUD, or custom.

### Status Tracking

- `draft` → `sent` → `paid` / `cancelled` / `overdue`
- When an invoice linked to an engagement is marked **paid**, the engagement's `payment_cleared` gate automatically sets to `true`.
- Changing a paid invoice back to another status resets the gate to `false`.

### PDF Export

Click **Generate PDF** to create a branded invoice/quote PDF using your company settings.

### Invoice Numbers

Auto-generated with configurable prefix, e.g., `INV-001`, `QUO-001`. Change the prefix in **Settings**.
