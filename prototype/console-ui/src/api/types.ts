// API Response Types

// =============================================================================
// Role-Based Access Control (RBAC)
// =============================================================================

export type Role = 'admin' | 'developer' | 'viewer';

export type Permission =
  // Status & Monitoring
  | 'view:status'
  | 'view:metrics'
  | 'view:config'
  // Sessions
  | 'view:sessions'
  | 'kick:session'
  // Rooms
  | 'view:rooms'
  | 'terminate:room'
  // Accounts
  | 'view:accounts'
  | 'create:account'
  | 'update:account'
  | 'delete:account'
  | 'disable:account'
  // API Keys
  | 'view:apikeys'
  | 'create:apikey'
  | 'delete:apikey'
  // Lua/Scripting
  | 'view:scripts'
  | 'reload:scripts'
  | 'execute:rpc'
  // Storage
  | 'view:storage'
  | 'write:storage'
  | 'delete:storage'
  // Leaderboards
  | 'view:leaderboards'
  | 'delete:leaderboard'
  | 'delete:leaderboard_record'
  // Matchmaker
  | 'view:matchmaker'
  | 'cancel:matchmaker_ticket'
  // Notifications
  | 'view:notifications'
  | 'send:notification'
  // Chat
  | 'view:chat'
  | 'delete:chat_message';

// Permission mappings per role
// Admin: Full access to everything
// Developer: Can view everything, can execute RPCs, manage some resources
// Viewer: Read-only access to non-sensitive data
export const ROLE_PERMISSIONS: Record<Role, Permission[]> = {
  admin: [
    'view:status', 'view:metrics', 'view:config',
    'view:sessions', 'kick:session',
    'view:rooms', 'terminate:room',
    'view:accounts', 'create:account', 'update:account', 'delete:account', 'disable:account',
    'view:apikeys', 'create:apikey', 'delete:apikey',
    'view:scripts', 'reload:scripts', 'execute:rpc',
    'view:storage', 'write:storage', 'delete:storage',
    'view:leaderboards', 'delete:leaderboard', 'delete:leaderboard_record',
    'view:matchmaker', 'cancel:matchmaker_ticket',
    'view:notifications', 'send:notification',
    'view:chat', 'delete:chat_message',
  ],
  developer: [
    'view:status', 'view:metrics', 'view:config',
    'view:sessions',
    'view:rooms',
    'view:accounts',
    'view:apikeys',
    'view:scripts', 'execute:rpc',
    'view:storage',
    'view:leaderboards',
    'view:matchmaker',
    'view:notifications',
    'view:chat',
  ],
  viewer: [
    'view:status', 'view:metrics',
    'view:sessions',
    'view:rooms',
    'view:leaderboards',
  ],
};

// =============================================================================
// Server Types
// =============================================================================

export interface ServerStatus {
  version: string;
  uptime_secs: number;
  sessions: SessionStats;
  rooms: RoomStats;
}

export interface SessionStats {
  total: number;
  connecting: number;
  connected: number;
  authenticated: number;
}

export interface RoomStats {
  total: number;
  players: number;
}

export interface SessionInfo {
  id: number;
  address: string;
  state: string;
  user_id: string | null;
  username: string | null;
  room_id: string | null;
  connected_at: number;
  last_heartbeat: number;
}

export interface RoomInfo {
  id: string;
  label: string | null;
  module: string | null;
  state: string;
  tick_rate: number;
  player_count: number;
  max_players: number;
  created_at: number;
}

export interface RoomPlayerInfo {
  session_id: number;
  user_id: string | null;
  username: string | null;
  address: string;
}

export interface AccountInfo {
  id: string;
  username: string;
  role: string;
  disabled?: boolean;
}

export interface ApiKeyInfo {
  id: string;
  name: string;
  key_prefix: string;
  scopes: string[];
  created_at: number;
  expires_at: number | null;
  last_used: number | null;
  request_count: number;
  disabled: boolean;
}

export interface LuaScriptInfo {
  name: string;
  path: string;
  size: number;
  loaded: boolean;
  content?: string;
  error?: string;
  hooks?: string[];
  rpcs?: string[];
}

export interface RpcInfo {
  name: string;
  module: string;
}

export interface PaginatedList<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  token: string;
  expires_at: number;
  user: AccountInfo;
}

export interface CreateAccountRequest {
  username: string;
  password: string;
  role: string;
}

export interface CreateApiKeyRequest {
  name: string;
  scopes: string[];
  expires_in_days?: number;
}

export interface CreateApiKeyResponse {
  id: string;
  key: string;
  name: string;
  scopes: string[];
  expires_at: number | null;
}

// Metrics types - matches backend MetricsData struct
export interface MetricsData {
  uptime_seconds: number;
  sessions_active: number;
  sessions_total: number;
  sessions_by_state: Record<string, number>;
  rooms_active: number;
  rooms_total: number;
  websocket_connections: number;
  bytes_received_total: number;
  bytes_sent_total: number;
  udp_packets_received_total: number;
  udp_packets_sent_total: number;
  chat_messages_total: number;
  leaderboard_submissions_total: number;
  matchmaker_queue_size: number;
  matchmaker_matches_total: number;
  notifications_total: number;
}

// Audit Log types
export interface AuditLogInfo {
  id: string;
  timestamp: number;
  actor_id: string;
  actor_name: string;
  actor_type: 'user' | 'api_key';
  action: string;
  resource_type: string;
  resource_id: string;
  details: Record<string, unknown> | null;
  ip_address: string | null;
  success: boolean;
}
