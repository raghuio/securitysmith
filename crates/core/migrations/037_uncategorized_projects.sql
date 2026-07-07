-- Migration 037: Auto-create Uncategorized projects for orphaned engagements
-- Per user decision 2026-06-05: every client with legacy engagements gets a
-- default "Uncategorized" project and orphaned engagements are linked to it.

-- Create an "Uncategorized" project for each client that has at least one
-- engagement with project_id IS NULL.
INSERT INTO projects (
    client_id,
    name,
    description,
    status,
    start_date,
    end_date,
    budgeted_hours,
    tech_stack,
    tentative_dates,
    tags,
    notes,
    is_active,
    created_at,
    updated_at
)
SELECT DISTINCT
    e.client_id,
    'Uncategorized',
    'Auto-created for legacy engagements without a project.',
    'active',
    NULL,
    NULL,
    NULL,
    '[]',
    NULL,
    '[]',
    'Auto-created during migration to support Client → Project → Engagement hierarchy.',
    1,
    strftime('%s', 'now'),
    strftime('%s', 'now')
FROM engagements e
WHERE e.project_id IS NULL
  AND e.is_active = 1;

-- Link orphaned engagements to their respective Uncategorized project.
UPDATE engagements
SET project_id = (
    SELECT p.id
    FROM projects p
    WHERE p.client_id = engagements.client_id
      AND p.name      = 'Uncategorized'
)
WHERE project_id IS NULL
  AND is_active = 1;
