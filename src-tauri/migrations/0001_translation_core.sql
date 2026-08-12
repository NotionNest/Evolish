PRAGMA foreign_keys = ON;

CREATE TABLE translation_modes (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('built_in', 'custom')),
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    instruction TEXT NOT NULL CHECK (length(trim(instruction)) > 0),
    version INTEGER NOT NULL CHECK (version > 0),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    sort_order INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE provider_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    adapter_kind TEXT NOT NULL,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    endpoint TEXT,
    model TEXT NOT NULL CHECK (length(trim(model)) > 0),
    parameters_json TEXT NOT NULL,
    secret_ref TEXT,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    sort_order INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE translation_workspace_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    window_role TEXT NOT NULL UNIQUE CHECK (window_role IN ('main', 'mini')),
    primary_provider_profile_id TEXT REFERENCES provider_profiles(id),
    enabled_provider_order_json TEXT NOT NULL,
    primary_target_language TEXT NOT NULL,
    secondary_target_language TEXT NOT NULL CHECK (secondary_target_language <> primary_target_language),
    default_mode_id TEXT NOT NULL REFERENCES translation_modes(id),
    version INTEGER NOT NULL CHECK (version > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TRIGGER workspace_profile_primary_must_be_enabled_and_ordered_on_insert
BEFORE INSERT ON translation_workspace_profiles
WHEN NEW.primary_provider_profile_id IS NOT NULL
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM provider_profiles
        WHERE id = NEW.primary_provider_profile_id AND enabled = 1
    ) THEN RAISE(ABORT, 'primary provider must be enabled') END;
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM json_each(NEW.enabled_provider_order_json)
        WHERE value = NEW.primary_provider_profile_id
    ) THEN RAISE(ABORT, 'primary provider must be in enabled order') END;
END;

CREATE TRIGGER workspace_profile_primary_must_be_enabled_and_ordered_on_update
BEFORE UPDATE OF primary_provider_profile_id, enabled_provider_order_json ON translation_workspace_profiles
WHEN NEW.primary_provider_profile_id IS NOT NULL
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM provider_profiles
        WHERE id = NEW.primary_provider_profile_id AND enabled = 1
    ) THEN RAISE(ABORT, 'primary provider must be enabled') END;
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM json_each(NEW.enabled_provider_order_json)
        WHERE value = NEW.primary_provider_profile_id
    ) THEN RAISE(ABORT, 'primary provider must be in enabled order') END;
END;

INSERT INTO translation_modes (id, kind, name, instruction, version, enabled, sort_order, created_at, updated_at) VALUES
    ('018f0a10-0000-7000-8000-000000000001', 'built_in', 'Standard', 'Translate accurately and clearly.', 1, 1, 0, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z'),
    ('018f0a10-0000-7000-8000-000000000002', 'built_in', 'Literal', 'Preserve the source wording and structure where natural.', 1, 1, 1, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z'),
    ('018f0a10-0000-7000-8000-000000000003', 'built_in', 'Natural', 'Prefer idiomatic target-language expression.', 1, 1, 2, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z'),
    ('018f0a10-0000-7000-8000-000000000004', 'built_in', 'Academic', 'Use precise formal terminology and an academic register.', 1, 1, 3, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z'),
    ('018f0a10-0000-7000-8000-000000000005', 'built_in', 'Concise', 'Translate concisely without omitting meaning.', 1, 1, 4, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z');

INSERT INTO translation_workspace_profiles (id, window_role, primary_provider_profile_id, enabled_provider_order_json, primary_target_language, secondary_target_language, default_mode_id, version, created_at, updated_at)
VALUES ('workspace-main', 'main', NULL, '[]', 'en', 'zh-CN', '018f0a10-0000-7000-8000-000000000001', 1, '2026-08-12T00:00:00Z', '2026-08-12T00:00:00Z');
