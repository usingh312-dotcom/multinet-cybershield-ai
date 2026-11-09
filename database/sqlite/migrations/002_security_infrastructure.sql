-- MultiNet CyberShield AI Phase 2 security infrastructure schema (additive only)
-- Never rewrites 001_foundation.sql. Only additive statements are permitted here.
PRAGMA foreign_keys = ON;

-- Incident metadata columns (align storage with the frontend incident contracts).
ALTER TABLE incidents ADD COLUMN category TEXT;
ALTER TABLE incidents ADD COLUMN source TEXT;
ALTER TABLE incidents ADD COLUMN assigned_to TEXT;

-- Evidence attachments: opaque references only. Payloads are never stored inline;
-- integrity is recorded as a status managed by the authorized backend.
CREATE TABLE IF NOT EXISTS incident_evidence (
  id TEXT PRIMARY KEY,
  incident_id TEXT NOT NULL,
  evidence_type TEXT NOT NULL CHECK (evidence_type IN
    ('network_event','endpoint_event','dns_event','threat_intelligence','audit_log')),
  evidence_reference TEXT NOT NULL,
  collected_at TEXT,
  integrity_status TEXT NOT NULL DEFAULT 'unverified' CHECK (integrity_status IN
    ('unverified','verified','failed')),
  created_at TEXT NOT NULL,
  FOREIGN KEY(incident_id) REFERENCES incidents(id)
);
CREATE INDEX IF NOT EXISTS idx_incident_evidence_incident ON incident_evidence(incident_id);
CREATE INDEX IF NOT EXISTS idx_incident_evidence_type ON incident_evidence(evidence_type);

-- Persistent application preferences (non-secret only; credentials use the OS credential store).
CREATE TABLE IF NOT EXISTS app_preferences (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
