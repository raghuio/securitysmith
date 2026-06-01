ALTER TABLE findings ADD COLUMN reported_at TEXT;
ALTER TABLE findings ADD COLUMN fix_deadline TEXT;
ALTER TABLE findings ADD COLUMN client_response TEXT DEFAULT 'no_response'
    CHECK (client_response IN ('acknowledged', 'in_progress', 'fixed', 'accepted_risk', 'disputed', 'deferred', 'no_response'));
ALTER TABLE findings ADD COLUMN retested_at TEXT;
ALTER TABLE findings ADD COLUMN retest_result TEXT DEFAULT 'not_tested'
    CHECK (retest_result IN ('not_tested', 'pass', 'fail', 'partial'));
ALTER TABLE findings ADD COLUMN retest_notes TEXT;
ALTER TABLE findings ADD COLUMN original_finding_id INTEGER REFERENCES findings(id);

ALTER TABLE engagements ADD COLUMN original_engagement_id INTEGER REFERENCES engagements(id);
