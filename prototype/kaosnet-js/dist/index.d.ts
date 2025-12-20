/**
 * KaosNet SDK Types
 */
interface Session {
    token: string;
    refreshToken?: string;
    userId: string;
    username: string;
    expiresAt: number;
    createdAt: number;
}
interface AuthResponse {
    session: Session;
    newAccount: boolean;
}
interface Account {
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
interface DeviceInfo {
    deviceId: string;
    linkedAt: number;
}
interface DeviceAuthRequest {
    deviceId: string;
    create?: boolean;
    username?: string;
}
interface EmailAuthRequest {
    email: string;
    password: string;
    create?: boolean;
    username?: string;
}
interface CustomAuthRequest {
    id: string;
    vars?: Record<string, string>;
    create?: boolean;
    username?: string;
}
interface MatchmakerTicket {
    ticketId: string;
    presenceId: string;
}
interface MatchmakerAddRequest {
    query?: string;
    minCount?: number;
    maxCount?: number;
    stringProperties?: Record<string, string>;
    numericProperties?: Record<string, number>;
}
interface MatchmakerMatch {
    matchId: string;
    token: string;
    users: MatchmakerUser[];
    self: MatchmakerUser;
}
interface MatchmakerUser {
    presenceId: string;
    username: string;
    stringProperties?: Record<string, string>;
    numericProperties?: Record<string, number>;
}
interface Match {
    matchId: string;
    authoritative: boolean;
    label?: string;
    size: number;
    presences: Presence[];
    self: Presence;
}
interface Presence {
    userId: string;
    sessionId: string;
    username: string;
    node?: string;
}
interface MatchState<T = unknown> {
    opcode: number;
    data: T;
    sender?: Presence;
}
interface StorageObject<T = unknown> {
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
interface StorageWriteRequest<T = unknown> {
    collection: string;
    key: string;
    value: T;
    version?: number;
    permissionRead?: number;
    permissionWrite?: number;
}
interface StorageReadRequest {
    collection: string;
    key: string;
    userId?: string;
}
interface StorageDeleteRequest {
    collection: string;
    key: string;
    version?: number;
}
interface LeaderboardRecord {
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
interface LeaderboardRecordList {
    records: LeaderboardRecord[];
    ownerRecords?: LeaderboardRecord[];
    nextCursor?: string;
    prevCursor?: string;
}
interface RpcResponse<T = unknown> {
    id: string;
    payload: T;
}
type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';
interface KaosNetEvents {
    connect: () => void;
    disconnect: (reason?: string) => void;
    error: (error: Error) => void;
    reconnect: (attempt: number) => void;
    matchmakerMatched: (match: MatchmakerMatch) => void;
    matchPresenceJoin: (matchId: string, presences: Presence[]) => void;
    matchPresenceLeave: (matchId: string, presences: Presence[]) => void;
    matchState: <T>(state: MatchState<T>) => void;
}
interface ClientOptions {
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
interface SocketOptions {
    /** Enable client-side prediction (default: false) */
    enablePrediction?: boolean;
    /** Enable state interpolation (default: true) */
    enableInterpolation?: boolean;
    /** Interpolation buffer size in ms (default: 100) */
    interpolationBuffer?: number;
}

/**
 * KaosNet Socket - Real-time WebSocket connection
 */

/**
 * WebSocket opcodes matching the server protocol.
 */
declare enum OpCode {
    SessionStart = 1,
    SessionEnd = 2,
    Heartbeat = 3,
    SessionAck = 4,
    RoomCreate = 16,
    RoomJoin = 17,
    RoomLeave = 18,
    RoomData = 19,
    RoomState = 20,
    RoomList = 21,
    Rpc = 32,
    RpcResponse = 33,
    MatchmakerAdd = 48,
    MatchmakerRemove = 49,
    MatchmakerMatched = 50,
    Error = 255
}
/**
 * KaosNet Socket for real-time game communication.
 *
 * @example
 * ```typescript
 * const socket = client.createSocket();
 * await socket.connect(session);
 *
 * // Join matchmaking
 * const ticket = await socket.addMatchmaker({
 *   query: '+mode:ranked +region:us',
 *   minCount: 2,
 *   maxCount: 4,
 * });
 *
 * // Handle match found
 * socket.onMatchmakerMatched = (match) => {
 *   socket.joinMatch(match.matchId);
 * };
 *
 * // Send/receive match state
 * socket.onMatchState = (state) => {
 *   console.log('Received state:', state);
 * };
 * socket.sendMatchState(1, { x: 100, y: 200 });
 * ```
 */
declare class KaosSocket {
    private ws;
    private readonly wsUrl;
    private readonly clientOptions;
    private readonly socketOptions;
    private session;
    private reconnectAttempts;
    private reconnectTimer;
    private heartbeatTimer;
    private rpcCallbacks;
    private rpcIdCounter;
    private currentMatchId;
    private eventListeners;
    onConnect?: () => void;
    onDisconnect?: (reason?: string) => void;
    onError?: (error: Error) => void;
    onMatchmakerMatched?: (match: MatchmakerMatch) => void;
    onMatchPresenceJoin?: (matchId: string, presences: Presence[]) => void;
    onMatchPresenceLeave?: (matchId: string, presences: Presence[]) => void;
    onMatchState?: <T>(state: MatchState<T>) => void;
    private _state;
    constructor(wsUrl: string, clientOptions: Required<ClientOptions>, socketOptions?: SocketOptions);
    get state(): ConnectionState;
    get isConnected(): boolean;
    private log;
    private emit;
    /**
     * Add an event listener.
     */
    on<K extends keyof KaosNetEvents>(event: K, callback: KaosNetEvents[K]): () => void;
    /**
     * Remove an event listener.
     */
    off<K extends keyof KaosNetEvents>(event: K, callback: KaosNetEvents[K]): void;
    /**
     * Add a one-time event listener.
     */
    once<K extends keyof KaosNetEvents>(event: K, callback: KaosNetEvents[K]): () => void;
    /**
     * Connect to the game server.
     */
    connect(session: Session): Promise<void>;
    /**
     * Disconnect from the server.
     */
    disconnect(): void;
    private attemptReconnect;
    private clearReconnectTimer;
    private startHeartbeat;
    private stopHeartbeat;
    private sendOpcode;
    private sendJson;
    private handleMessage;
    private handleJsonMessage;
    private handleBinaryMessage;
    /**
     * Add to matchmaking queue.
     */
    addMatchmaker(request: MatchmakerAddRequest): Promise<MatchmakerTicket>;
    /**
     * Remove from matchmaking queue.
     */
    removeMatchmaker(ticketId: string): Promise<void>;
    /**
     * Create a new match.
     */
    createMatch(name?: string): Promise<Match>;
    /**
     * Join an existing match.
     */
    joinMatch(matchId: string, metadata?: Record<string, string>): Promise<Match>;
    /**
     * Leave the current match.
     */
    leaveMatch(): Promise<void>;
    /**
     * Send match state to other players.
     */
    sendMatchState<T = unknown>(opcode: number, data: T, presences?: Presence[]): void;
    /**
     * Send raw data to the server (for simple games).
     * This is a convenience method for games that don't use the full protocol.
     */
    send<T = unknown>(data: T): void;
    /**
     * Call a server RPC function.
     */
    rpc<T = unknown, R = unknown>(id: string, payload?: T): Promise<R>;
}

/**
 * KaosNet Client - Main entry point for the SDK
 */

/**
 * KaosNet Client for connecting to a KaosNet game server.
 *
 * @example
 * ```typescript
 * const client = new KaosClient('localhost', 7350);
 * const session = await client.authenticateDevice({ deviceId: 'unique-device-id' });
 * const socket = client.createSocket();
 * await socket.connect(session);
 * ```
 */
declare class KaosClient {
    private readonly options;
    private readonly baseUrl;
    constructor(host?: string, port?: number, useSSL?: boolean);
    constructor(options: ClientOptions);
    private log;
    private request;
    /**
     * Authenticate with a device ID (anonymous auth).
     * Creates a new account if one doesn't exist.
     */
    authenticateDevice(request: DeviceAuthRequest): Promise<Session>;
    authenticateDevice(deviceId: string, create?: boolean, username?: string): Promise<Session>;
    /**
     * Authenticate with email and password.
     * Creates a new account if `create` is true.
     */
    authenticateEmail(request: EmailAuthRequest): Promise<Session>;
    authenticateEmail(email: string, password: string, create?: boolean, username?: string): Promise<Session>;
    /**
     * Authenticate with a custom method (e.g., Steam, custom backend).
     */
    authenticateCustom(request: CustomAuthRequest): Promise<Session>;
    authenticateCustom(id: string, create?: boolean, username?: string, vars?: Record<string, string>): Promise<Session>;
    /**
     * Refresh an expired session token.
     */
    refreshSession(session: Session): Promise<Session>;
    /**
     * Check if a session token is expired.
     */
    isSessionExpired(session: Session): boolean;
    /**
     * Get the authenticated user's account info.
     */
    getAccount(session: Session): Promise<Account>;
    /**
     * Link a device ID to the authenticated account.
     * Allows logging in with multiple devices.
     */
    linkDevice(session: Session, deviceId: string): Promise<void>;
    /**
     * Link email/password to the authenticated account.
     * Allows the user to log in with email after initial device auth.
     */
    linkEmail(session: Session, email: string, password: string): Promise<void>;
    /**
     * Unlink a device from the authenticated account.
     */
    unlinkDevice(session: Session, deviceId: string): Promise<void>;
    /**
     * Create a new WebSocket connection for real-time communication.
     */
    createSocket(): KaosSocket;
    /**
     * Read storage objects.
     */
    readStorageObjects<T = unknown>(session: Session, requests: StorageReadRequest[]): Promise<StorageObject<T>[]>;
    /**
     * Write storage objects.
     */
    writeStorageObjects<T = unknown>(session: Session, objects: StorageWriteRequest<T>[]): Promise<StorageObject<T>[]>;
    /**
     * Delete storage objects.
     */
    deleteStorageObjects(session: Session, requests: StorageDeleteRequest[]): Promise<void>;
    /**
     * List storage objects in a collection.
     */
    listStorageObjects<T = unknown>(session: Session, collection: string, userId?: string, limit?: number, cursor?: string): Promise<{
        objects: StorageObject<T>[];
        cursor?: string;
    }>;
    /**
     * List leaderboard records.
     */
    listLeaderboardRecords(session: Session, leaderboardId: string, ownerIds?: string[], limit?: number, cursor?: string): Promise<LeaderboardRecordList>;
    /**
     * Write a leaderboard record.
     */
    writeLeaderboardRecord(session: Session, leaderboardId: string, score: number, subscore?: number, metadata?: Record<string, unknown>): Promise<void>;
    /**
     * Add to matchmaker queue with properties.
     * Like Nakama's matchmaker with string and numeric properties.
     *
     * @example
     * ```typescript
     * const ticket = await client.addMatchmaker(session, {
     *   queue: 'ranked',
     *   query: '+region:us +mode:ranked',
     *   minCount: 2,
     *   maxCount: 4,
     *   stringProperties: { region: 'us', mode: 'ranked' },
     *   numericProperties: { skill: 1500 },
     * });
     * ```
     */
    addMatchmaker(session: Session, request: {
        queue: string;
        query?: string;
        minCount?: number;
        maxCount?: number;
        stringProperties?: Record<string, string>;
        numericProperties?: Record<string, number>;
    }): Promise<{
        ticket: {
            id: string;
            queue: string;
            players: Array<{
                user_id: string;
                username: string;
                skill: number;
            }>;
            properties: Record<string, unknown>;
            created_at: number;
        };
    }>;
    /**
     * Remove from matchmaker queue.
     */
    removeMatchmaker(session: Session): Promise<{
        message: string;
        ticket_id: string;
    }>;
    /**
     * Get matchmaker ticket for a user.
     */
    getMatchmakerTicket(session: Session, userId?: string): Promise<{
        id: string;
        queue: string;
        players: Array<{
            user_id: string;
            username: string;
            skill: number;
        }>;
        properties: Record<string, unknown>;
        created_at: number;
    } | null>;
    /**
     * List matchmaker queues with stats.
     */
    listMatchmakerQueues(session: Session): Promise<{
        queues: Array<{
            queue: string;
            tickets: number;
            players: number;
            longest_wait_secs: number;
        }>;
    }>;
    /**
     * Call a server RPC function via HTTP.
     */
    rpc<T = unknown, R = unknown>(session: Session, id: string, payload?: T): Promise<R>;
}

export { type Account, type AuthResponse, KaosClient as Client, type ClientOptions, type ConnectionState, type CustomAuthRequest, type DeviceAuthRequest, type DeviceInfo, type EmailAuthRequest, KaosClient, type KaosNetEvents, KaosSocket, type LeaderboardRecord, type LeaderboardRecordList, type Match, type MatchState, type MatchmakerAddRequest, type MatchmakerMatch, type MatchmakerTicket, type MatchmakerUser, OpCode, type Presence, type RpcResponse, type Session, KaosSocket as Socket, type SocketOptions, type StorageDeleteRequest, type StorageObject, type StorageReadRequest, type StorageWriteRequest };
