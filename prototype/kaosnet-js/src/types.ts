/**
 * KaosNet SDK Types
 */

// ============================================================================
// Session & Authentication
// ============================================================================

export interface Session {
  token: string;
  refreshToken?: string;
  userId: string;
  username: string;
  expiresAt: number;
  createdAt: number;
}

export interface AuthResponse {
  session: Session;
  newAccount: boolean;
}

export interface Account {
  id: string;
  username?: string;
  displayName?: string;
  avatarUrl?: string;
  email?: string;
  devices: DeviceInfo[];
  metadata?: Record<string, unknown>;
  createdAt: number;
  updatedAt: number;
  disabled: boolean;
}

export interface DeviceInfo {
  deviceId: string;
  linkedAt: number;
}

export interface DeviceAuthRequest {
  deviceId: string;
  create?: boolean;
  username?: string;
}

export interface EmailAuthRequest {
  email: string;
  password: string;
  create?: boolean;
  username?: string;
}

export interface CustomAuthRequest {
  id: string;
  vars?: Record<string, string>;
  create?: boolean;
  username?: string;
}

// ============================================================================
// Matchmaking
// ============================================================================

export interface MatchmakerTicket {
  ticketId: string;
  presenceId: string;
}

export interface MatchmakerAddRequest {
  query?: string;
  minCount?: number;
  maxCount?: number;
  stringProperties?: Record<string, string>;
  numericProperties?: Record<string, number>;
}

export interface MatchmakerMatch {
  matchId: string;
  token: string;
  users: MatchmakerUser[];
  self: MatchmakerUser;
}

export interface MatchmakerUser {
  presenceId: string;
  username: string;
  stringProperties?: Record<string, string>;
  numericProperties?: Record<string, number>;
}

// ============================================================================
// Match / Room
// ============================================================================

export interface Match {
  matchId: string;
  authoritative: boolean;
  label?: string;
  size: number;
  presences: Presence[];
  self: Presence;
}

export interface Presence {
  userId: string;
  sessionId: string;
  username: string;
  node?: string;
}

export interface MatchState<T = unknown> {
  opcode: number;
  data: T;
  sender?: Presence;
}

// ============================================================================
// Storage
// ============================================================================

export interface StorageObject<T = unknown> {
  collection: string;
  key: string;
  userId: string;
  value: T;
  version: number;
  permissionRead: number;
  permissionWrite: number;
  createdAt: number;
  updatedAt: number;
}

export interface StorageWriteRequest<T = unknown> {
  collection: string;
  key: string;
  value: T;
  version?: number;
  permissionRead?: number;
  permissionWrite?: number;
}

export interface StorageReadRequest {
  collection: string;
  key: string;
  userId?: string;
}

export interface StorageDeleteRequest {
  collection: string;
  key: string;
  version?: number;
}

// ============================================================================
// Leaderboards
// ============================================================================

export interface LeaderboardRecord {
  leaderboardId: string;
  ownerId: string;
  username?: string;
  score: number;
  subscore: number;
  numScore: number;
  metadata?: Record<string, unknown>;
  createTime: number;
  updateTime: number;
  expiryTime?: number;
  rank: number;
}

export interface LeaderboardRecordList {
  records: LeaderboardRecord[];
  ownerRecords?: LeaderboardRecord[];
  nextCursor?: string;
  prevCursor?: string;
}

// ============================================================================
// RPC
// ============================================================================

export interface RpcResponse<T = unknown> {
  id: string;
  payload: T;
}

// ============================================================================
// Events
// ============================================================================

export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

export interface KaosNetEvents {
  connect: () => void;
  disconnect: (reason?: string) => void;
  error: (error: Error) => void;
  reconnect: (attempt: number) => void;
  matchmakerMatched: (match: MatchmakerMatch) => void;
  matchPresenceJoin: (matchId: string, presences: Presence[]) => void;
  matchPresenceLeave: (matchId: string, presences: Presence[]) => void;
  matchState: <T>(state: MatchState<T>) => void;
}

// ============================================================================
// Client Options
// ============================================================================

export interface ClientOptions {
  /** Server host (default: 'localhost') */
  host?: string;
  /** Server port (default: 7350) */
  port?: number;
  /** Use SSL/TLS (default: false) */
  useSSL?: boolean;
  /** Connection timeout in ms (default: 10000) */
  timeout?: number;
  /** Auto-reconnect on disconnect (default: true) */
  autoReconnect?: boolean;
  /** Max reconnect attempts (default: 5) */
  maxReconnectAttempts?: number;
  /** Reconnect delay in ms (default: 1000) */
  reconnectDelay?: number;
  /** Enable verbose logging (default: false) */
  verbose?: boolean;
}

export interface SocketOptions {
  /** Enable client-side prediction (default: false) */
  enablePrediction?: boolean;
  /** Enable state interpolation (default: true) */
  enableInterpolation?: boolean;
  /** Interpolation buffer size in ms (default: 100) */
  interpolationBuffer?: number;
}
