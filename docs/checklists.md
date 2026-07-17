# Methodology Checklists

## `sm checklist` — track testing methodology

Checklists help you systematically follow a testing methodology (OWASP WSTG, ASVS, PTES) and demonstrate coverage in your reports.

### Assign a checklist to an engagement

```sh
sm checklist acme/web_app/initial --assign owasp-wstg
```

This creates a `checklist.toml` in the engagement directory with all items from the built-in checklist.

### List checklist items with coverage

```sh
sm checklist acme/web_app/initial --list
```

Shows each item's ID, status, and linked finding (if any). A coverage percentage is displayed at the top.

Item statuses: `not_started`, `in_progress`, `tested`, `not_applicable`, `finding_created`, `deferred`.

Coverage = count of (`tested` + `finding_created` + `not_applicable`) / total items.

### Update item status

```sh
sm checklist acme/web_app/initial --item WSTG-INPV-01 --status tested
```

### Link a finding to a checklist item

```sh
sm checklist acme/web_app/initial --item WSTG-INPV-01 --finding ACME-WEB-001
```

This links the checklist item to the finding so you can trace which test case discovered which finding.