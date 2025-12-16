-- KaosNet Database Schema
-- This file is run on first database creation

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- Storage Objects (game data persistence)
-- ============================================================================

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

-- ============================================================================
-- Leaderboard Records
-- ============================================================================

CREATE TABLE IF NOT EXISTS leaderboard_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    leaderboard_id VARCHAR(128) NOT NULL,
    owner_id VARCHAR(128) NOT NULL,
    username VARCHAR(128),
    score BIGINT NOT NULL DEFAULT 0,
    subscore BIGINT NOT NULL DEFAULT 0,
    num_score INT NOT NULL DEFAULT 1,
    metadata JSONB,
    rank_value BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    UNIQUE (leaderboard_id, owner_id)
);

CREATE INDEX IF NOT EXISTS idx_leaderboard_id ON leaderboard_records(leaderboard_id);
CREATE INDEX IF NOT EXISTS idx_leaderboard_score ON leaderboard_records(leaderboard_id, score DESC);
CREATE INDEX IF NOT EXISTS idx_leaderboard_owner ON leaderboard_records(owner_id);

-- ============================================================================
-- Players (game users)
-- ============================================================================

CREATE TABLE IF NOT EXISTS players (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(128) UNIQUE NOT NULL,
    username VARCHAR(128) NOT NULL,
    display_name VARCHAR(128),
    email VARCHAR(256),
    email_verified BOOLEAN DEFAULT FALSE,
    avatar_url VARCHAR(512),
    lang_tag VARCHAR(16) DEFAULT 'en',
    location VARCHAR(128),
    timezone VARCHAR(64),
    metadata JSONB DEFAULT '{}',
    edge_count INT DEFAULT 0,
    banned BOOLEAN DEFAULT FALSE,
    ban_reason VARCHAR(512),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_players_username ON players(username);
CREATE INDEX IF NOT EXISTS idx_players_email ON players(email);
CREATE INDEX IF NOT EXISTS idx_players_banned ON players(banned);

-- ============================================================================
-- Friends (social)
-- ============================================================================

CREATE TABLE IF NOT EXISTS friends (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(128) NOT NULL,
    friend_id VARCHAR(128) NOT NULL,
    state INT NOT NULL DEFAULT 0,  -- 0=friends, 1=invite_sent, 2=invite_received, 3=blocked
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (user_id, friend_id)
);

CREATE INDEX IF NOT EXISTS idx_friends_user ON friends(user_id);
CREATE INDEX IF NOT EXISTS idx_friends_state ON friends(user_id, state);

-- ============================================================================
-- Groups (clans/guilds)
-- ============================================================================

CREATE TABLE IF NOT EXISTS groups (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(128) NOT NULL,
    description VARCHAR(1024),
    avatar_url VARCHAR(512),
    lang_tag VARCHAR(16) DEFAULT 'en',
    open BOOLEAN DEFAULT TRUE,
    edge_count INT DEFAULT 0,
    max_count INT DEFAULT 100,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_groups_name ON groups(name);
CREATE INDEX IF NOT EXISTS idx_groups_open ON groups(open);

CREATE TABLE IF NOT EXISTS group_members (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id VARCHAR(128) NOT NULL,
    state INT NOT NULL DEFAULT 2,  -- 0=superadmin, 1=admin, 2=member, 3=join_request
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (group_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_group_members_group ON group_members(group_id);
CREATE INDEX IF NOT EXISTS idx_group_members_user ON group_members(user_id);

-- ============================================================================
-- Notifications
-- ============================================================================

CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(128) NOT NULL,
    subject VARCHAR(256) NOT NULL,
    content JSONB NOT NULL DEFAULT '{}',
    code INT NOT NULL DEFAULT 0,
    sender_id VARCHAR(128),
    read BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_unread ON notifications(user_id, read) WHERE read = FALSE;

-- ============================================================================
-- Tournaments
-- ============================================================================

CREATE TABLE IF NOT EXISTS tournaments (
    id VARCHAR(128) PRIMARY KEY,
    name VARCHAR(256) NOT NULL,
    description VARCHAR(1024),
    category INT DEFAULT 0,
    sort_order VARCHAR(16) DEFAULT 'descending',
    operator VARCHAR(16) DEFAULT 'best',
    size INT DEFAULT 1000000,
    max_num_score INT DEFAULT 1000000,
    max_size INT DEFAULT 0,
    duration INT DEFAULT 0,
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    reset_schedule VARCHAR(64),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tournament_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tournament_id VARCHAR(128) NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    owner_id VARCHAR(128) NOT NULL,
    username VARCHAR(128),
    score BIGINT NOT NULL DEFAULT 0,
    subscore BIGINT NOT NULL DEFAULT 0,
    num_score INT NOT NULL DEFAULT 1,
    metadata JSONB,
    rank_value BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (tournament_id, owner_id)
);

CREATE INDEX IF NOT EXISTS idx_tournament_records_id ON tournament_records(tournament_id);
CREATE INDEX IF NOT EXISTS idx_tournament_records_score ON tournament_records(tournament_id, score DESC);

-- ============================================================================
-- Console Accounts (admin users)
-- ============================================================================

CREATE TABLE IF NOT EXISTS console_accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(128) UNIQUE NOT NULL,
    password_hash VARCHAR(256) NOT NULL,
    role VARCHAR(32) NOT NULL DEFAULT 'readonly',
    disabled BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_login TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_console_accounts_username ON console_accounts(username);

-- ============================================================================
-- API Keys (programmatic access)
-- ============================================================================

CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(128) NOT NULL,
    key_hash VARCHAR(256) NOT NULL,
    key_prefix VARCHAR(16) NOT NULL,
    scopes INT NOT NULL DEFAULT 0,
    created_by UUID REFERENCES console_accounts(id),
    disabled BOOLEAN DEFAULT FALSE,
    expires_at TIMESTAMPTZ,
    last_used TIMESTAMPTZ,
    request_count BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix);
