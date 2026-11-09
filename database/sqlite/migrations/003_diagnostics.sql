-- Day 11: Diagnostics persistence for real traffic sampling and speed tests.
-- Additive only: no existing table is altered.

CREATE TABLE IF NOT EXISTS traffic_samples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    interface_id TEXT NOT NULL,
    interface_index INTEGER NOT NULL,
    rx_bytes INTEGER NOT NULL,
    tx_bytes INTEGER NOT NULL,
    rx_rate_bps REAL,
    tx_rate_bps REAL,
    counter_reset INTEGER NOT NULL DEFAULT 0,
    sampled_at TEXT NOT NULL,
    source TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_traffic_samples_sampled_at
    ON traffic_samples (sampled_at);
CREATE INDEX IF NOT EXISTS idx_traffic_samples_interface_time
    ON traffic_samples (interface_id, sampled_at);

CREATE TABLE IF NOT EXISTS speed_test_results (
    id TEXT PRIMARY KEY,
    server_url TEXT NOT NULL,
    server_name TEXT NOT NULL,
    server_observed_client_ip TEXT NOT NULL,
    interface_id TEXT,
    interface_local_ip TEXT,
    download_mbps REAL NOT NULL,
    upload_mbps REAL NOT NULL,
    ping_ms REAL NOT NULL,
    jitter_ms REAL NOT NULL,
    loss_pct REAL NOT NULL,
    download_bytes INTEGER NOT NULL,
    upload_bytes INTEGER NOT NULL,
    ping_samples INTEGER NOT NULL,
    ping_received INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_speed_test_results_completed_at
    ON speed_test_results (completed_at);
