-- KaosNet Storage Schema for PostgreSQL

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS storage_objects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(128) NOT NULL,
    collection VARCHAR(128) NOT NULL,
    key VARCHAR(128) NOT NULL,
    value JSONB NOT NULL DEFAULT '{}',
    version BIGINT NOT NULL DEFAULT 1,
    permission INT DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (user_id, collection, key)
);

CREATE INDEX IF NOT EXISTS idx_storage_user ON storage_objects(user_id);
CREATE INDEX IF NOT EXISTS idx_storage_collection ON storage_objects(user_id, collection);
CREATE INDEX IF NOT EXISTS idx_storage_value ON storage_objects USING GIN(value);
