-- Migration 036: Alter engagements table
-- Adds project_id and type_label_id per PROP-033.
-- Keeps existing engagement_type column for backwards compat.

ALTER TABLE engagements ADD COLUMN project_id INTEGER;
ALTER TABLE engagements ADD COLUMN type_label_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_engagements_project_id ON engagements(project_id);
CREATE INDEX IF NOT EXISTS idx_engagements_type_label ON engagements(type_label_id);
