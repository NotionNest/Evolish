CREATE TABLE history_policy (
    singleton_id INTEGER PRIMARY KEY NOT NULL CHECK (singleton_id = 1),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    retention_days INTEGER CHECK (retention_days IS NULL OR retention_days > 0)
);

CREATE TABLE translation_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    original_text TEXT NOT NULL,
    normalized_text TEXT NOT NULL,
    source_language TEXT,
    source_language_confidence REAL,
    target_language TEXT NOT NULL,
    intent TEXT NOT NULL,
    entry_point TEXT NOT NULL,
    mode_id TEXT NOT NULL,
    mode_version INTEGER NOT NULL CHECK (mode_version > 0),
    mode_snapshot_json TEXT NOT NULL,
    privacy_mode TEXT NOT NULL,
    workspace_snapshot_version INTEGER NOT NULL CHECK (workspace_snapshot_version > 0),
    workspace_snapshot_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE translation_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES translation_sessions(id),
    provider_profile_id TEXT NOT NULL,
    provider_snapshot_json TEXT NOT NULL,
    request_snapshot_version INTEGER NOT NULL CHECK (request_snapshot_version > 0),
    request_snapshot_json TEXT NOT NULL,
    result_version INTEGER NOT NULL CHECK (result_version > 0),
    status TEXT NOT NULL,
    error_code TEXT,
    started_at TEXT,
    completed_at TEXT,
    UNIQUE (session_id, provider_profile_id, result_version)
);

CREATE TABLE translation_results (
    attempt_id TEXT PRIMARY KEY NOT NULL REFERENCES translation_attempts(id),
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    result_kind TEXT NOT NULL,
    primary_translation TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX translation_workspace_profiles_default_mode_id_idx
    ON translation_workspace_profiles(default_mode_id);

CREATE INDEX translation_sessions_created_at_idx
    ON translation_sessions(created_at DESC);

CREATE INDEX translation_attempts_session_id_idx
    ON translation_attempts(session_id);
