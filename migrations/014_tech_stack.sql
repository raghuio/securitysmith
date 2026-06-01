-- Add tech_stack column to clients for news keyword matching (PROP-014)
ALTER TABLE clients ADD COLUMN tech_stack TEXT NOT NULL DEFAULT '[]';
