-- KaosNet Sample Data
-- Run this after init.sql to populate with demo data

-- ============================================================================
-- Sample Players
-- ============================================================================

INSERT INTO players (user_id, username, display_name, email, email_verified, metadata) VALUES
    ('user_001', 'ProGamer42', 'The Pro Gamer', 'pro@example.com', true, '{"level": 50, "xp": 125000, "achievements": ["first_blood", "champion"]}'),
    ('user_002', 'CasualPlayer', 'Just Chilling', 'casual@example.com', true, '{"level": 15, "xp": 8500}'),
    ('user_003', 'SpeedRunner', 'Speed Demon', 'speed@example.com', false, '{"level": 35, "xp": 67000, "speedrun_pb": 142.5}'),
    ('user_004', 'NoobMaster69', 'Noob No More', 'noob@example.com', true, '{"level": 5, "xp": 1200}'),
    ('user_005', 'EliteSniper', 'Elite Sniper', 'elite@example.com', true, '{"level": 42, "xp": 98000, "headshots": 5420}'),
    ('user_006', 'TankMain', 'Tank Main', 'tank@example.com', true, '{"level": 28, "xp": 45000, "damage_blocked": 1250000}'),
    ('user_007', 'HealBot', 'Healing Machine', 'heal@example.com', true, '{"level": 38, "xp": 72000, "heals": 890000}'),
    ('user_008', 'LoneSurvivor', 'Last Standing', 'lone@example.com', false, '{"level": 22, "xp": 31000}'),
    ('user_009', 'TeamCaptain', 'The Captain', 'captain@example.com', true, '{"level": 45, "xp": 110000, "wins": 324}'),
    ('user_010', 'NightOwl', 'Night Gamer', 'night@example.com', true, '{"level": 19, "xp": 22000}')
ON CONFLICT (user_id) DO NOTHING;

-- ============================================================================
-- Sample Storage Objects (player profiles)
-- ============================================================================

INSERT INTO storage_objects (user_id, collection, key, value) VALUES
    ('user_001', 'profiles', 'main', '{"name": "ProGamer42", "color": "#FF6B6B", "high_score": 15420, "games_played": 892}'),
    ('user_002', 'profiles', 'main', '{"name": "CasualPlayer", "color": "#4ECDC4", "high_score": 3200, "games_played": 45}'),
    ('user_003', 'profiles', 'main', '{"name": "SpeedRunner", "color": "#45B7D1", "high_score": 8900, "games_played": 234}'),
    ('user_004', 'profiles', 'main', '{"name": "NoobMaster69", "color": "#96CEB4", "high_score": 450, "games_played": 12}'),
    ('user_005', 'profiles', 'main', '{"name": "EliteSniper", "color": "#FFEAA7", "high_score": 12100, "games_played": 567}'),
    ('user_001', 'inventory', 'items', '{"coins": 15000, "gems": 250, "items": ["sword_epic", "shield_rare", "potion_x10"]}'),
    ('user_002', 'inventory', 'items', '{"coins": 2500, "gems": 25, "items": ["sword_common"]}'),
    ('user_005', 'inventory', 'items', '{"coins": 45000, "gems": 890, "items": ["sniper_legendary", "cloak_epic", "ammo_box"]}'),
    ('user_001', 'stats', 'combat', '{"kills": 4521, "deaths": 892, "assists": 2340, "kd_ratio": 5.07}'),
    ('user_005', 'stats', 'combat', '{"kills": 8942, "deaths": 1205, "assists": 890, "kd_ratio": 7.42}')
ON CONFLICT (user_id, collection, key) DO NOTHING;

-- ============================================================================
-- Sample Leaderboard Records
-- ============================================================================

INSERT INTO leaderboard_records (leaderboard_id, owner_id, username, score, metadata) VALUES
    ('kaos_io_highscores', 'user_001', 'ProGamer42', 15420, '{"kills": 142, "color": "#FF6B6B"}'),
    ('kaos_io_highscores', 'user_005', 'EliteSniper', 12100, '{"kills": 98, "color": "#FFEAA7"}'),
    ('kaos_io_highscores', 'user_003', 'SpeedRunner', 8900, '{"kills": 67, "color": "#45B7D1"}'),
    ('kaos_io_highscores', 'user_009', 'TeamCaptain', 7650, '{"kills": 54, "color": "#DDA0DD"}'),
    ('kaos_io_highscores', 'user_007', 'HealBot', 5200, '{"kills": 38, "color": "#98D8C8"}'),
    ('kaos_io_highscores', 'user_006', 'TankMain', 4800, '{"kills": 42, "color": "#F7DC6F"}'),
    ('kaos_io_highscores', 'user_002', 'CasualPlayer', 3200, '{"kills": 24, "color": "#4ECDC4"}'),
    ('kaos_io_highscores', 'user_010', 'NightOwl', 2100, '{"kills": 18, "color": "#BB8FCE"}'),
    ('kaos_io_highscores', 'user_008', 'LoneSurvivor', 1500, '{"kills": 12, "color": "#85C1E9"}'),
    ('kaos_io_highscores', 'user_004', 'NoobMaster69', 450, '{"kills": 3, "color": "#96CEB4"}'),
    ('weekly_scores', 'user_001', 'ProGamer42', 8200, '{"week": 50}'),
    ('weekly_scores', 'user_005', 'EliteSniper', 7100, '{"week": 50}'),
    ('weekly_scores', 'user_003', 'SpeedRunner', 5400, '{"week": 50}'),
    ('asteroids_highscores', 'user_003', 'SpeedRunner', 45200, '{"asteroids_destroyed": 892}'),
    ('asteroids_highscores', 'user_001', 'ProGamer42', 38900, '{"asteroids_destroyed": 756}'),
    ('asteroids_highscores', 'user_005', 'EliteSniper', 32100, '{"asteroids_destroyed": 612}')
ON CONFLICT (leaderboard_id, owner_id) DO NOTHING;

-- ============================================================================
-- Sample Groups
-- ============================================================================

INSERT INTO groups (id, name, description, open, max_count) VALUES
    ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'Elite Gamers', 'For serious competitive players only', false, 50),
    ('b2c3d4e5-f6a7-8901-bcde-f12345678901', 'Casual Crew', 'Just having fun!', true, 100),
    ('c3d4e5f6-a7b8-9012-cdef-123456789012', 'Speed Run Masters', 'Breaking records together', true, 25)
ON CONFLICT DO NOTHING;

INSERT INTO group_members (group_id, user_id, state) VALUES
    ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'user_001', 0),  -- superadmin
    ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'user_005', 1),  -- admin
    ('a1b2c3d4-e5f6-7890-abcd-ef1234567890', 'user_009', 2),  -- member
    ('b2c3d4e5-f6a7-8901-bcde-f12345678901', 'user_002', 0),  -- superadmin
    ('b2c3d4e5-f6a7-8901-bcde-f12345678901', 'user_004', 2),  -- member
    ('b2c3d4e5-f6a7-8901-bcde-f12345678901', 'user_010', 2),  -- member
    ('c3d4e5f6-a7b8-9012-cdef-123456789012', 'user_003', 0),  -- superadmin
    ('c3d4e5f6-a7b8-9012-cdef-123456789012', 'user_001', 2)   -- member
ON CONFLICT (group_id, user_id) DO NOTHING;

-- ============================================================================
-- Sample Friends
-- ============================================================================

INSERT INTO friends (user_id, friend_id, state) VALUES
    ('user_001', 'user_005', 0),  -- friends
    ('user_005', 'user_001', 0),  -- friends (bidirectional)
    ('user_001', 'user_009', 0),  -- friends
    ('user_009', 'user_001', 0),
    ('user_002', 'user_004', 0),  -- friends
    ('user_004', 'user_002', 0),
    ('user_003', 'user_001', 1),  -- invite sent
    ('user_001', 'user_003', 2),  -- invite received
    ('user_001', 'user_004', 3)   -- blocked
ON CONFLICT (user_id, friend_id) DO NOTHING;

-- ============================================================================
-- Sample Notifications
-- ============================================================================

INSERT INTO notifications (user_id, subject, content, code, sender_id) VALUES
    ('user_001', 'Friend Request', '{"message": "SpeedRunner wants to be your friend!"}', 1, 'user_003'),
    ('user_001', 'Achievement Unlocked', '{"achievement": "champion", "description": "Win 100 games"}', 2, null),
    ('user_002', 'Welcome!', '{"message": "Welcome to KaosNet! Start playing to earn rewards."}', 0, null),
    ('user_004', 'Group Invite', '{"group": "Casual Crew", "inviter": "CasualPlayer"}', 3, 'user_002'),
    ('user_005', 'Leaderboard Update', '{"message": "You are now #2 on the weekly leaderboard!"}', 4, null)
ON CONFLICT DO NOTHING;

-- ============================================================================
-- Sample Tournaments
-- ============================================================================

INSERT INTO tournaments (id, name, description, sort_order, operator, duration, start_time, end_time) VALUES
    ('weekly_tournament', 'Weekly Championship', 'Compete for weekly glory!', 'descending', 'best', 604800, NOW(), NOW() + INTERVAL '7 days'),
    ('speed_challenge', 'Speed Challenge', 'Fastest players win!', 'ascending', 'best', 86400, NOW(), NOW() + INTERVAL '1 day')
ON CONFLICT (id) DO NOTHING;

INSERT INTO tournament_records (tournament_id, owner_id, username, score, metadata) VALUES
    ('weekly_tournament', 'user_001', 'ProGamer42', 8200, '{"matches_played": 15}'),
    ('weekly_tournament', 'user_005', 'EliteSniper', 7100, '{"matches_played": 12}'),
    ('weekly_tournament', 'user_003', 'SpeedRunner', 5400, '{"matches_played": 18}'),
    ('speed_challenge', 'user_003', 'SpeedRunner', 142, '{"time_seconds": 142}'),
    ('speed_challenge', 'user_001', 'ProGamer42', 187, '{"time_seconds": 187}')
ON CONFLICT (tournament_id, owner_id) DO NOTHING;

-- Update ranks (would normally be done by application)
-- This is a simplified version - real implementation would use window functions
