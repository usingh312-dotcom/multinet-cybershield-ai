-- MultiNet CyberShield AI Phase 1 foundation schema (SQLite)
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS network_interfaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  mac_address TEXT,
  is_up INTEGER NOT NULL DEFAULT 0,
  metadata_json TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_network_interfaces_name ON network_interfaces(name);

CREATE TABLE IF NOT EXISTS connection_metrics (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  interface_id TEXT NOT NULL,
  latency_ms REAL,
  jitter_ms REAL,
  loss_pct REAL,
  rx_bytes INTEGER NOT NULL DEFAULT 0,
  tx_bytes INTEGER NOT NULL DEFAULT 0,
  measured_at TEXT NOT NULL,
  source TEXT NOT NULL,
  FOREIGN KEY(interface_id) REFERENCES network_interfaces(id)
);
CREATE INDEX IF NOT EXISTS idx_connection_metrics_iface_time ON connection_metrics(interface_id, measured_at);

CREATE TABLE IF NOT EXISTS bonding_sessions (
  id TEXT PRIMARY KEY,
  status TEXT NOT NULL,
  policy TEXT,
  started_at TEXT,
  stopped_at TEXT,
  failure_reason TEXT
);

CREATE TABLE IF NOT EXISTS tunnel_paths (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  interface_id TEXT,
  remote_endpoint TEXT,
  state TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES bonding_sessions(id)
);
CREATE INDEX IF NOT EXISTS idx_tunnel_paths_session ON tunnel_paths(session_id);

CREATE TABLE IF NOT EXISTS security_events (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  severity TEXT NOT NULL,
  category TEXT,
  summary TEXT NOT NULL,
  raw_ref TEXT,
  occurred_at TEXT NOT NULL,
  ingested_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_security_events_time ON security_events(occurred_at);
CREATE INDEX IF NOT EXISTS idx_security_events_severity ON security_events(severity);

CREATE TABLE IF NOT EXISTS endpoint_processes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pid INTEGER,
  name TEXT NOT NULL,
  path TEXT,
  user_name TEXT,
  sha256 TEXT,
  observed_at TEXT NOT NULL,
  source TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_endpoint_processes_time ON endpoint_processes(observed_at);

CREATE TABLE IF NOT EXISTS dns_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  query_name TEXT NOT NULL,
  query_type TEXT,
  response_code TEXT,
  resolver TEXT,
  verdict TEXT,
  observed_at TEXT NOT NULL,
  source TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_dns_events_time ON dns_events(observed_at);
CREATE INDEX IF NOT EXISTS idx_dns_events_name ON dns_events(query_name);

CREATE TABLE IF NOT EXISTS threat_indicators (
  id TEXT PRIMARY KEY,
  indicator_type TEXT NOT NULL,
  indicator_value TEXT NOT NULL,
  confidence TEXT,
  source TEXT NOT NULL,
  first_seen_at TEXT,
  last_seen_at TEXT,
  metadata_json TEXT
);
CREATE INDEX IF NOT EXISTS idx_threat_indicators_value ON threat_indicators(indicator_type, indicator_value);

CREATE TABLE IF NOT EXISTS incidents (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  severity TEXT NOT NULL,
  status TEXT NOT NULL,
  summary TEXT,
  opened_at TEXT NOT NULL,
  closed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_incidents_status ON incidents(status);

CREATE TABLE IF NOT EXISTS incident_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  incident_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  detail_json TEXT,
  occurred_at TEXT NOT NULL,
  FOREIGN KEY(incident_id) REFERENCES incidents(id)
);
CREATE INDEX IF NOT EXISTS idx_incident_events_incident ON incident_events(incident_id, occurred_at);

CREATE TABLE IF NOT EXISTS approval_requests (
  id TEXT PRIMARY KEY,
  action TEXT NOT NULL,
  target TEXT NOT NULL,
  reason TEXT NOT NULL,
  status TEXT NOT NULL,
  requested_by TEXT NOT NULL,
  decided_by TEXT,
  requested_at TEXT NOT NULL,
  decided_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_approval_requests_status ON approval_requests(status);

CREATE TABLE IF NOT EXISTS firewall_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  direction TEXT,
  action TEXT NOT NULL,
  scope TEXT,
  enabled INTEGER NOT NULL DEFAULT 0,
  managed_by TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  subject_id TEXT,
  actor TEXT,
  detail TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_logs_time ON audit_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_kind ON audit_logs(kind);

CREATE TABLE IF NOT EXISTS ai_conversations (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  role TEXT NOT NULL,
  content_ref TEXT,
  metadata_json TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ai_conversations_time ON ai_conversations(created_at);
