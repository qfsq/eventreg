-- EventRegister schema. Safe to run repeatedly: tables, the enum, and indexes
-- are created only when missing so `./deploy.sh` can upgrade an existing install.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

DO $$
BEGIN
    CREATE TYPE event_category AS ENUM (
        'chore',
        'work',
        'emotion',
        'thinking',
        'writing',
        'inventing'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE,
    provider VARCHAR(50) NOT NULL,
    provider_id VARCHAR(255) NOT NULL,
    display_name VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_provider_user UNIQUE (provider, provider_id)
);

CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category event_category NOT NULL,
    title VARCHAR(255) NOT NULL CHECK (length(btrim(title)) > 0),
    content TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    logged_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT events_title_length CHECK (length(title) <= 255)
);

CREATE TABLE IF NOT EXISTS user_summaries (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category event_category NOT NULL,
    total_count BIGINT NOT NULL DEFAULT 0 CHECK (total_count >= 0),
    last_logged_at TIMESTAMPTZ,
    streak_days INT NOT NULL DEFAULT 0 CHECK (streak_days >= 0),
    aggregate_metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, category)
);

-- Upgrade path for databases created by the original scaffold.
ALTER TABLE user_summaries
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE INDEX IF NOT EXISTS idx_events_user_logged
    ON events (user_id, logged_at DESC);

CREATE INDEX IF NOT EXISTS idx_events_user_category_logged
    ON events (user_id, category, logged_at DESC);
