-- KaosNet PostgreSQL Schema
-- Initialize database schema for game server

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Storage: Objects table
CREATE TABLE IF NOT EXISTS storage_objects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(128) NOT NULL,
    collection VARCHAR(128) NOT NULL,
    key VARCHAR(128) NOT NULL,
    value JSONB NOT NULL DEFAULT '{}',
    version VARCHAR(64),
    permission INT DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (user_id, collection, key)
);

CREATE INDEX idx_storage_user ON storage_objects(user_id);
CREATE INDEX idx_storage_collection ON storage_objects(user_id, collection);
CREATE INDEX idx_storage_value ON storage_objects USING GIN(value);

-- Leaderboards: Config
CREATE TABLE IF NOT EXISTS leaderboard_config (
    id VARCHAR(128) PRIMARY KEY,
    name VARCHAR(256) NOT NULL,
    sort_order VARCHAR(16) DEFAULT 'descending',
    operator VARCHAR(16) DEFAULT 'best',
    reset_schedule VARCHAR(16) DEFAULT 'never',
    max_entries INT DEFAULT 10000,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Leaderboards: Records
CREATE TABLE IF NOT EXISTS leaderboard_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    leaderboard_id VARCHAR(128) NOT NULL REFERENCES leaderboard_config(id),
    user_id VARCHAR(128) NOT NULL,
    username VARCHAR(128) NOT NULL,
    score BIGINT NOT NULL,
    num_submissions INT DEFAULT 1,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (leaderboard_id, user_id)
);

CREATE INDEX idx_leaderboard_score ON leaderboard_records(leaderboard_id, score DESC);

-- Social: Friends
CREATE TABLE IF NOT EXISTS friends (
    user_id VARCHAR(128) NOT NULL,
    friend_id VARCHAR(128) NOT NULL,
    state INT NOT NULL DEFAULT 0, -- 0=pending, 1=accepted, 2=blocked
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, friend_id)
);

CREATE INDEX idx_friends_state ON friends(user_id, state);

-- Social: Groups
CREATE TABLE IF NOT EXISTS groups (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(256) NOT NULL,
    description TEXT,
    creator_id VARCHAR(128) NOT NULL,
    is_open BOOLEAN DEFAULT true,
    max_members INT DEFAULT 100,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_groups_name ON groups(name);

-- Social: Group Members
CREATE TABLE IF NOT EXISTS group_members (
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id VARCHAR(128) NOT NULL,
    role INT DEFAULT 0, -- 0=member, 1=mod, 2=admin, 3=owner
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (group_id, user_id)
);

-- Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(128) NOT NULL,
    code INT NOT NULL,
    subject VARCHAR(256),
    content TEXT,
    sender_id VARCHAR(128),
    data JSONB,
    is_read BOOLEAN DEFAULT false,
    is_persistent BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

CREATE INDEX idx_notifications_user ON notifications(user_id, created_at DESC);
CREATE INDEX idx_notifications_unread ON notifications(user_id, is_read) WHERE NOT is_read;

-- Matchmaker: Tickets (transient, typically in-memory)
-- This table is optional, mainly for persistence across restarts
CREATE TABLE IF NOT EXISTS matchmaker_tickets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    queue VARCHAR(128) NOT NULL,
    user_ids VARCHAR(128)[] NOT NULL,
    skill DOUBLE PRECISION NOT NULL,
    properties JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_matchmaker_queue ON matchmaker_tickets(queue, skill);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at triggers
CREATE TRIGGER storage_objects_updated_at
    BEFORE UPDATE ON storage_objects
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER leaderboard_records_updated_at
    BEFORE UPDATE ON leaderboard_records
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER friends_updated_at
    BEFORE UPDATE ON friends
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER groups_updated_at
    BEFORE UPDATE ON groups
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ==================== Tournaments ====================

-- Tournament configuration
CREATE TABLE IF NOT EXISTS tournaments (
    id VARCHAR(128) PRIMARY KEY,
    name VARCHAR(256) NOT NULL,
    description TEXT,
    category VARCHAR(128) DEFAULT 'default',
    sort_order VARCHAR(16) DEFAULT 'descending',
    operator VARCHAR(16) DEFAULT 'best',
    max_participants INT DEFAULT 0,
    max_submissions INT DEFAULT 0,
    entry_fee BIGINT DEFAULT 0,
    metadata JSONB,
    reset_schedule VARCHAR(16) DEFAULT 'never',
    duration_secs INT NOT NULL,
    join_window_secs INT DEFAULT 0,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    state VARCHAR(16) DEFAULT 'upcoming',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_tournaments_category ON tournaments(category);
CREATE INDEX idx_tournaments_state ON tournaments(state);
CREATE INDEX idx_tournaments_start ON tournaments(start_time);

-- Tournament records (participants)
CREATE TABLE IF NOT EXISTS tournament_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tournament_id VARCHAR(128) NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    user_id VARCHAR(128) NOT NULL,
    username VARCHAR(128) NOT NULL,
    score BIGINT DEFAULT 0,
    num_submissions INT DEFAULT 0,
    metadata JSONB,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (tournament_id, user_id)
);

CREATE INDEX idx_tournament_records_score ON tournament_records(tournament_id, score DESC);
CREATE INDEX idx_tournament_records_user ON tournament_records(user_id);

-- ==================== Chat ====================

-- Chat channels
CREATE TABLE IF NOT EXISTS chat_channels (
    id VARCHAR(256) PRIMARY KEY,
    name VARCHAR(256) NOT NULL,
    channel_type VARCHAR(16) NOT NULL, -- 'room', 'group', 'dm'
    room_id VARCHAR(128),
    group_id UUID,
    dm_user1 VARCHAR(128),
    dm_user2 VARCHAR(128),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_chat_channels_room ON chat_channels(room_id) WHERE room_id IS NOT NULL;
CREATE INDEX idx_chat_channels_group ON chat_channels(group_id) WHERE group_id IS NOT NULL;
CREATE INDEX idx_chat_channels_dm ON chat_channels(dm_user1, dm_user2) WHERE channel_type = 'dm';

-- Chat messages
CREATE TABLE IF NOT EXISTS chat_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    channel_id VARCHAR(256) NOT NULL REFERENCES chat_channels(id) ON DELETE CASCADE,
    sender_id VARCHAR(128) NOT NULL,
    sender_username VARCHAR(128) NOT NULL,
    content TEXT NOT NULL,
    code INT DEFAULT 0,
    data JSONB,
    is_persistent BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_chat_messages_channel ON chat_messages(channel_id, created_at DESC);
CREATE INDEX idx_chat_messages_sender ON chat_messages(sender_id);

-- Chat channel members
CREATE TABLE IF NOT EXISTS chat_members (
    channel_id VARCHAR(256) NOT NULL REFERENCES chat_channels(id) ON DELETE CASCADE,
    user_id VARCHAR(128) NOT NULL,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_chat_members_user ON chat_members(user_id);

-- Triggers for new tables
CREATE TRIGGER tournament_records_updated_at
    BEFORE UPDATE ON tournament_records
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER chat_messages_updated_at
    BEFORE UPDATE ON chat_messages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
