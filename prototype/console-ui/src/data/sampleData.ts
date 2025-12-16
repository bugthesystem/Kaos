// Sample data for Console UI demonstration

export interface Player {
  id: string;
  username: string;
  display_name: string | null;
  email: string | null;
  email_verified: boolean;
  devices: string[];
  social_links: { provider: string; provider_id: string; linked_at: number }[];
  created_at: number;
  updated_at: number;
  banned: boolean;
  ban_reason: string | null;
  avatar_url: string | null;
  level: number;
  xp: number;
  games_played: number;
  wins: number;
  losses: number;
  last_seen: number;
}

export interface Session {
  id: string;
  user_id: string;
  username: string;
  state: 'connecting' | 'connected' | 'authenticated';
  ip_address: string;
  user_agent: string;
  connected_at: number;
  last_activity: number;
}

export interface Room {
  id: string;
  name: string;
  game_mode: string;
  state: 'waiting' | 'playing' | 'finished';
  max_players: number;
  current_players: number;
  created_at: number;
  tick_rate: number;
  metadata: Record<string, any>;
}

export interface LeaderboardRecord {
  rank: number;
  owner_id: string;
  username: string;
  score: number;
  metadata: Record<string, any>;
  updated_at: number;
}

export interface Leaderboard {
  id: string;
  sort_order: 'ascending' | 'descending';
  operator: 'best' | 'set' | 'incr';
  reset_schedule: string | null;
  record_count: number;
  records: LeaderboardRecord[];
}

export interface ChatMessage {
  id: string;
  channel_id: string;
  sender_id: string;
  sender_username: string;
  content: string;
  created_at: number;
}

export interface ChatChannel {
  id: string;
  name: string;
  type: 'room' | 'group' | 'direct';
  member_count: number;
  created_at: number;
  messages: ChatMessage[];
}

export interface StorageObject {
  user_id: string;
  collection: string;
  key: string;
  value: Record<string, any>;
  version: string;
  created_at: number;
  updated_at: number;
}

export interface Tournament {
  id: string;
  name: string;
  description: string;
  category: string;
  state: 'upcoming' | 'active' | 'completed';
  start_time: number;
  end_time: number;
  max_participants: number;
  current_participants: number;
  prize_pool: string;
}

// Generate timestamps
const now = Date.now();
const hour = 3600000;
const day = 86400000;

// Sample Players
export const samplePlayers: Player[] = [
  {
    id: 'player_001',
    username: 'ProGamer42',
    display_name: 'Pro Gamer',
    email: 'progamer42@example.com',
    email_verified: true,
    devices: ['device_ios_abc123', 'device_web_xyz789'],
    social_links: [
      { provider: 'discord', provider_id: 'ProGamer#1234', linked_at: now - 30 * day },
      { provider: 'steam', provider_id: '76561198012345678', linked_at: now - 25 * day },
    ],
    created_at: now - 90 * day,
    updated_at: now - hour,
    banned: false,
    ban_reason: null,
    avatar_url: null,
    level: 42,
    xp: 12500,
    games_played: 523,
    wins: 312,
    losses: 211,
    last_seen: now - 5 * 60000,
  },
  {
    id: 'player_002',
    username: 'CasualPlayer',
    display_name: 'Just Chillin',
    email: 'casual@example.com',
    email_verified: true,
    devices: ['device_android_def456'],
    social_links: [],
    created_at: now - 45 * day,
    updated_at: now - 2 * hour,
    banned: false,
    ban_reason: null,
    avatar_url: null,
    level: 15,
    xp: 3200,
    games_played: 87,
    wins: 41,
    losses: 46,
    last_seen: now - 30 * 60000,
  },
  {
    id: 'player_003',
    username: 'SpeedRunner',
    display_name: 'Speed Demon',
    email: 'speed@example.com',
    email_verified: false,
    devices: ['device_pc_ghi789'],
    social_links: [
      { provider: 'twitch', provider_id: 'speedrunner_live', linked_at: now - 10 * day },
    ],
    created_at: now - 60 * day,
    updated_at: now - 3 * hour,
    banned: false,
    ban_reason: null,
    avatar_url: null,
    level: 38,
    xp: 9800,
    games_played: 234,
    wins: 189,
    losses: 45,
    last_seen: now - 2 * hour,
  },
  {
    id: 'player_004',
    username: 'NewbiePlayer',
    display_name: null,
    email: 'newbie@example.com',
    email_verified: false,
    devices: ['device_web_jkl012'],
    social_links: [],
    created_at: now - 2 * day,
    updated_at: now - day,
    banned: false,
    ban_reason: null,
    avatar_url: null,
    level: 3,
    xp: 450,
    games_played: 12,
    wins: 4,
    losses: 8,
    last_seen: now - day,
  },
  {
    id: 'player_005',
    username: 'ToxicTroll',
    display_name: 'Banned User',
    email: 'toxic@example.com',
    email_verified: true,
    devices: ['device_web_mno345'],
    social_links: [],
    created_at: now - 120 * day,
    updated_at: now - 5 * day,
    banned: true,
    ban_reason: 'Repeated harassment and toxic behavior',
    avatar_url: null,
    level: 28,
    xp: 7200,
    games_played: 156,
    wins: 67,
    losses: 89,
    last_seen: now - 5 * day,
  },
  {
    id: 'player_006',
    username: 'StrategyMaster',
    display_name: 'The Strategist',
    email: 'strategy@example.com',
    email_verified: true,
    devices: ['device_pc_pqr678', 'device_tablet_stu901'],
    social_links: [
      { provider: 'discord', provider_id: 'StratMaster#5678', linked_at: now - 50 * day },
    ],
    created_at: now - 180 * day,
    updated_at: now - 4 * hour,
    banned: false,
    ban_reason: null,
    avatar_url: null,
    level: 55,
    xp: 18900,
    games_played: 892,
    wins: 623,
    losses: 269,
    last_seen: now - hour,
  },
];

// Sample Sessions
export const sampleSessions: Session[] = [
  {
    id: 'sess_001',
    user_id: 'player_001',
    username: 'ProGamer42',
    state: 'authenticated',
    ip_address: '192.168.1.100',
    user_agent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)',
    connected_at: now - 2 * hour,
    last_activity: now - 5 * 60000,
  },
  {
    id: 'sess_002',
    user_id: 'player_003',
    username: 'SpeedRunner',
    state: 'authenticated',
    ip_address: '10.0.0.55',
    user_agent: 'KaosNet/1.0 (Windows NT 10.0)',
    connected_at: now - hour,
    last_activity: now - 2 * 60000,
  },
  {
    id: 'sess_003',
    user_id: 'player_006',
    username: 'StrategyMaster',
    state: 'connected',
    ip_address: '172.16.0.42',
    user_agent: 'Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X)',
    connected_at: now - 30 * 60000,
    last_activity: now - 60000,
  },
  {
    id: 'sess_004',
    user_id: '',
    username: '',
    state: 'connecting',
    ip_address: '192.168.1.201',
    user_agent: 'Mozilla/5.0 (Linux; Android 14)',
    connected_at: now - 30000,
    last_activity: now - 30000,
  },
];

// Sample Rooms
export const sampleRooms: Room[] = [
  {
    id: 'room_001',
    name: 'Battle Royale #1',
    game_mode: 'battle_royale',
    state: 'playing',
    max_players: 100,
    current_players: 87,
    created_at: now - 15 * 60000,
    tick_rate: 20,
    metadata: { map: 'island', difficulty: 'normal' },
  },
  {
    id: 'room_002',
    name: 'Team Deathmatch',
    game_mode: 'tdm',
    state: 'waiting',
    max_players: 16,
    current_players: 8,
    created_at: now - 5 * 60000,
    tick_rate: 60,
    metadata: { map: 'arena', teams: 2 },
  },
  {
    id: 'room_003',
    name: 'Ranked Match #42',
    game_mode: 'ranked',
    state: 'playing',
    max_players: 10,
    current_players: 10,
    created_at: now - 25 * 60000,
    tick_rate: 30,
    metadata: { rank_tier: 'gold', season: 3 },
  },
  {
    id: 'room_004',
    name: 'Private Lobby',
    game_mode: 'custom',
    state: 'waiting',
    max_players: 20,
    current_players: 3,
    created_at: now - 2 * 60000,
    tick_rate: 20,
    metadata: { password_protected: true, owner: 'player_001' },
  },
];

// Sample Leaderboards
export const sampleLeaderboards: Leaderboard[] = [
  {
    id: 'weekly_scores',
    sort_order: 'descending',
    operator: 'best',
    reset_schedule: '0 0 * * 0',
    record_count: 156,
    records: [
      { rank: 1, owner_id: 'player_006', username: 'StrategyMaster', score: 25400, metadata: { wins: 42 }, updated_at: now - hour },
      { rank: 2, owner_id: 'player_001', username: 'ProGamer42', score: 23100, metadata: { wins: 38 }, updated_at: now - 2 * hour },
      { rank: 3, owner_id: 'player_003', username: 'SpeedRunner', score: 21800, metadata: { wins: 35 }, updated_at: now - 3 * hour },
      { rank: 4, owner_id: 'player_002', username: 'CasualPlayer', score: 8500, metadata: { wins: 12 }, updated_at: now - 5 * hour },
      { rank: 5, owner_id: 'player_004', username: 'NewbiePlayer', score: 1200, metadata: { wins: 2 }, updated_at: now - day },
    ],
  },
  {
    id: 'all_time_kills',
    sort_order: 'descending',
    operator: 'incr',
    reset_schedule: null,
    record_count: 892,
    records: [
      { rank: 1, owner_id: 'player_006', username: 'StrategyMaster', score: 15234, metadata: {}, updated_at: now - hour },
      { rank: 2, owner_id: 'player_001', username: 'ProGamer42', score: 12456, metadata: {}, updated_at: now - 2 * hour },
      { rank: 3, owner_id: 'player_003', username: 'SpeedRunner', score: 8923, metadata: {}, updated_at: now - 3 * hour },
    ],
  },
  {
    id: 'speedrun_times',
    sort_order: 'ascending',
    operator: 'best',
    reset_schedule: null,
    record_count: 45,
    records: [
      { rank: 1, owner_id: 'player_003', username: 'SpeedRunner', score: 12345, metadata: { category: 'any%' }, updated_at: now - day },
      { rank: 2, owner_id: 'player_001', username: 'ProGamer42', score: 15678, metadata: { category: 'any%' }, updated_at: now - 2 * day },
    ],
  },
];

// Sample Chat Channels
export const sampleChatChannels: ChatChannel[] = [
  {
    id: 'channel_global',
    name: 'Global Chat',
    type: 'group',
    member_count: 1234,
    created_at: now - 365 * day,
    messages: [
      { id: 'msg_001', channel_id: 'channel_global', sender_id: 'player_001', sender_username: 'ProGamer42', content: 'Anyone up for a match?', created_at: now - 5 * 60000 },
      { id: 'msg_002', channel_id: 'channel_global', sender_id: 'player_003', sender_username: 'SpeedRunner', content: 'Sure, count me in!', created_at: now - 4 * 60000 },
      { id: 'msg_003', channel_id: 'channel_global', sender_id: 'player_006', sender_username: 'StrategyMaster', content: 'GG everyone, great games today', created_at: now - 2 * 60000 },
    ],
  },
  {
    id: 'room_001_chat',
    name: 'Battle Royale #1',
    type: 'room',
    member_count: 87,
    created_at: now - 15 * 60000,
    messages: [
      { id: 'msg_004', channel_id: 'room_001_chat', sender_id: 'player_001', sender_username: 'ProGamer42', content: 'Final circle incoming!', created_at: now - 60000 },
    ],
  },
];

// Sample Storage Objects
export const sampleStorageObjects: StorageObject[] = [
  {
    user_id: 'player_001',
    collection: 'profiles',
    key: 'main',
    value: { color: '#ff5500', high_score: 15234, settings: { sound: true, music: false } },
    version: 'v3',
    created_at: now - 90 * day,
    updated_at: now - hour,
  },
  {
    user_id: 'player_001',
    collection: 'inventory',
    key: 'weapons',
    value: { items: ['sword_legendary', 'bow_epic', 'staff_rare'] },
    version: 'v12',
    created_at: now - 60 * day,
    updated_at: now - 2 * hour,
  },
  {
    user_id: 'player_003',
    collection: 'profiles',
    key: 'main',
    value: { color: '#00ff88', high_score: 21800, settings: { sound: true, music: true } },
    version: 'v2',
    created_at: now - 60 * day,
    updated_at: now - 3 * hour,
  },
  {
    user_id: 'player_006',
    collection: 'achievements',
    key: 'unlocked',
    value: { achievements: ['first_win', 'streak_10', 'champion', 'legendary'] },
    version: 'v8',
    created_at: now - 180 * day,
    updated_at: now - 4 * hour,
  },
];

// Sample Tournaments
export const sampleTournaments: Tournament[] = [
  {
    id: 'tournament_001',
    name: 'Winter Championship 2024',
    description: 'The ultimate winter showdown with massive prizes!',
    category: 'competitive',
    state: 'active',
    start_time: now - 2 * day,
    end_time: now + 5 * day,
    max_participants: 256,
    current_participants: 248,
    prize_pool: '$10,000',
  },
  {
    id: 'tournament_002',
    name: 'Weekly Showdown',
    description: 'Every week, battle for glory and rewards',
    category: 'weekly',
    state: 'active',
    start_time: now - 3 * day,
    end_time: now + 4 * day,
    max_participants: 128,
    current_participants: 95,
    prize_pool: '$500',
  },
  {
    id: 'tournament_003',
    name: 'Newcomer Cup',
    description: 'For players level 1-20 only',
    category: 'beginner',
    state: 'upcoming',
    start_time: now + 2 * day,
    end_time: now + 4 * day,
    max_participants: 64,
    current_participants: 23,
    prize_pool: '$100',
  },
  {
    id: 'tournament_004',
    name: 'Fall Championship 2024',
    description: 'The autumn championship has concluded',
    category: 'competitive',
    state: 'completed',
    start_time: now - 30 * day,
    end_time: now - 23 * day,
    max_participants: 512,
    current_participants: 512,
    prize_pool: '$25,000',
  },
];

// Utility functions
export function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString();
}

export function formatRelativeTime(ts: number): string {
  const diff = Date.now() - ts;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  return `${days}d ago`;
}

export function formatDuration(ms: number): string {
  const hours = Math.floor(ms / 3600000);
  const minutes = Math.floor((ms % 3600000) / 60000);
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}
