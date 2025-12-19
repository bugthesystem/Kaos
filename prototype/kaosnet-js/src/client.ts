/**
 * KaosNet Client - Main entry point for the SDK
 */

import type {
  ClientOptions,
  Session,
  AuthResponse,
  DeviceAuthRequest,
  EmailAuthRequest,
  CustomAuthRequest,
  StorageObject,
  StorageWriteRequest,
  StorageReadRequest,
  StorageDeleteRequest,
  LeaderboardRecordList,
} from './types';
import { KaosSocket } from './socket';

const DEFAULT_OPTIONS: Required<ClientOptions> = {
  host: 'localhost',
  port: 7350,
  useSSL: false,
  timeout: 10000,
  autoReconnect: true,
  maxReconnectAttempts: 5,
  reconnectDelay: 1000,
  verbose: false,
};

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
export class KaosClient {
  private readonly options: Required<ClientOptions>;
  private readonly baseUrl: string;

  constructor(host?: string, port?: number, useSSL?: boolean);
  constructor(options: ClientOptions);
  constructor(hostOrOptions?: string | ClientOptions, port?: number, useSSL?: boolean) {
    if (typeof hostOrOptions === 'object') {
      this.options = { ...DEFAULT_OPTIONS, ...hostOrOptions };
    } else {
      this.options = {
        ...DEFAULT_OPTIONS,
        host: hostOrOptions ?? DEFAULT_OPTIONS.host,
        port: port ?? DEFAULT_OPTIONS.port,
        useSSL: useSSL ?? DEFAULT_OPTIONS.useSSL,
      };
    }

    const protocol = this.options.useSSL ? 'https' : 'http';
    this.baseUrl = `${protocol}://${this.options.host}:${this.options.port}`;
  }

  private log(...args: unknown[]): void {
    if (this.options.verbose) {
      console.log('[KaosClient]', ...args);
    }
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
    session?: Session
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    if (session) {
      headers['Authorization'] = `Bearer ${session.token}`;
    }

    this.log(`${method} ${path}`);

    const response = await fetch(url, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
      signal: AbortSignal.timeout(this.options.timeout),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: response.statusText }));
      throw new Error(error.error || error.message || `HTTP ${response.status}`);
    }

    return response.json();
  }

  // ============================================================================
  // Authentication
  // ============================================================================

  /**
   * Authenticate with a device ID (anonymous auth).
   * Creates a new account if one doesn't exist.
   */
  async authenticateDevice(request: DeviceAuthRequest): Promise<Session>;
  async authenticateDevice(deviceId: string, create?: boolean, username?: string): Promise<Session>;
  async authenticateDevice(
    requestOrDeviceId: DeviceAuthRequest | string,
    create?: boolean,
    username?: string
  ): Promise<Session> {
    const req: DeviceAuthRequest =
      typeof requestOrDeviceId === 'string'
        ? { deviceId: requestOrDeviceId, create, username }
        : requestOrDeviceId;

    const response = await this.request<AuthResponse>('POST', '/api/auth/device', {
      device_id: req.deviceId,
      create: req.create ?? true,
      username: req.username,
    });

    return response.session;
  }

  /**
   * Authenticate with email and password.
   * Creates a new account if `create` is true.
   */
  async authenticateEmail(request: EmailAuthRequest): Promise<Session>;
  async authenticateEmail(email: string, password: string, create?: boolean, username?: string): Promise<Session>;
  async authenticateEmail(
    requestOrEmail: EmailAuthRequest | string,
    password?: string,
    create?: boolean,
    username?: string
  ): Promise<Session> {
    const req: EmailAuthRequest =
      typeof requestOrEmail === 'string'
        ? { email: requestOrEmail, password: password!, create, username }
        : requestOrEmail;

    const response = await this.request<AuthResponse>('POST', '/api/auth/email', {
      email: req.email,
      password: req.password,
      create: req.create ?? true,
      username: req.username,
    });

    return response.session;
  }

  /**
   * Authenticate with a custom method (e.g., Steam, custom backend).
   */
  async authenticateCustom(request: CustomAuthRequest): Promise<Session>;
  async authenticateCustom(id: string, create?: boolean, username?: string, vars?: Record<string, string>): Promise<Session>;
  async authenticateCustom(
    requestOrId: CustomAuthRequest | string,
    create?: boolean,
    username?: string,
    vars?: Record<string, string>
  ): Promise<Session> {
    const req: CustomAuthRequest =
      typeof requestOrId === 'string'
        ? { id: requestOrId, create, username, vars }
        : requestOrId;

    const response = await this.request<AuthResponse>('POST', '/api/auth/custom', {
      id: req.id,
      create: req.create ?? true,
      username: req.username,
      vars: req.vars,
    });

    return response.session;
  }

  /**
   * Refresh an expired session token.
   */
  async refreshSession(session: Session): Promise<Session> {
    if (!session.refreshToken) {
      throw new Error('Session does not have a refresh token');
    }

    const response = await this.request<AuthResponse>('POST', '/api/auth/refresh', {
      refresh_token: session.refreshToken,
    });

    return response.session;
  }

  /**
   * Check if a session token is expired.
   */
  isSessionExpired(session: Session): boolean {
    return Date.now() / 1000 >= session.expiresAt;
  }

  // ============================================================================
  // Socket
  // ============================================================================

  /**
   * Create a new WebSocket connection for real-time communication.
   */
  createSocket(): KaosSocket {
    const wsProtocol = this.options.useSSL ? 'wss' : 'ws';
    // Game server WebSocket is typically on port + 1
    const wsUrl = `${wsProtocol}://${this.options.host}:${this.options.port + 1}`;
    return new KaosSocket(wsUrl, this.options);
  }

  // ============================================================================
  // Storage (REST API)
  // ============================================================================

  /**
   * Read storage objects.
   */
  async readStorageObjects<T = unknown>(
    session: Session,
    requests: StorageReadRequest[]
  ): Promise<StorageObject<T>[]> {
    const response = await this.request<{ objects: StorageObject<T>[] }>(
      'POST',
      '/api/storage/read',
      { object_ids: requests.map(r => ({
        collection: r.collection,
        key: r.key,
        user_id: r.userId,
      }))},
      session
    );
    return response.objects;
  }

  /**
   * Write storage objects.
   */
  async writeStorageObjects<T = unknown>(
    session: Session,
    objects: StorageWriteRequest<T>[]
  ): Promise<StorageObject<T>[]> {
    const response = await this.request<{ objects: StorageObject<T>[] }>(
      'POST',
      '/api/storage/write',
      { objects: objects.map(o => ({
        collection: o.collection,
        key: o.key,
        value: o.value,
        version: o.version,
        permission_read: o.permissionRead,
        permission_write: o.permissionWrite,
      }))},
      session
    );
    return response.objects;
  }

  /**
   * Delete storage objects.
   */
  async deleteStorageObjects(
    session: Session,
    requests: StorageDeleteRequest[]
  ): Promise<void> {
    await this.request(
      'POST',
      '/api/storage/delete',
      { object_ids: requests.map(r => ({
        collection: r.collection,
        key: r.key,
        version: r.version,
      }))},
      session
    );
  }

  /**
   * List storage objects in a collection.
   */
  async listStorageObjects<T = unknown>(
    session: Session,
    collection: string,
    userId?: string,
    limit?: number,
    cursor?: string
  ): Promise<{ objects: StorageObject<T>[]; cursor?: string }> {
    const params = new URLSearchParams({ collection });
    if (userId) params.set('user_id', userId);
    if (limit) params.set('limit', String(limit));
    if (cursor) params.set('cursor', cursor);

    return this.request(`GET`, `/api/storage?${params}`, undefined, session);
  }

  // ============================================================================
  // Leaderboards (REST API)
  // ============================================================================

  /**
   * List leaderboard records.
   */
  async listLeaderboardRecords(
    session: Session,
    leaderboardId: string,
    ownerIds?: string[],
    limit?: number,
    cursor?: string
  ): Promise<LeaderboardRecordList> {
    const params = new URLSearchParams();
    if (ownerIds?.length) params.set('owner_ids', ownerIds.join(','));
    if (limit) params.set('limit', String(limit));
    if (cursor) params.set('cursor', cursor);

    return this.request(
      'GET',
      `/api/leaderboards/${leaderboardId}/records?${params}`,
      undefined,
      session
    );
  }

  /**
   * Write a leaderboard record.
   */
  async writeLeaderboardRecord(
    session: Session,
    leaderboardId: string,
    score: number,
    subscore?: number,
    metadata?: Record<string, unknown>
  ): Promise<void> {
    await this.request(
      'POST',
      `/api/leaderboards/${leaderboardId}/records`,
      { score, subscore, metadata },
      session
    );
  }

  // ============================================================================
  // Matchmaker (REST API)
  // ============================================================================

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
  async addMatchmaker(
    session: Session,
    request: {
      queue: string;
      query?: string;
      minCount?: number;
      maxCount?: number;
      stringProperties?: Record<string, string>;
      numericProperties?: Record<string, number>;
    }
  ): Promise<{
    ticket: {
      id: string;
      queue: string;
      players: Array<{ user_id: string; username: string; skill: number }>;
      properties: Record<string, unknown>;
      created_at: number;
    };
  }> {
    return this.request(
      'POST',
      '/api/matchmaker/add',
      {
        queue: request.queue,
        query: request.query,
        min_count: request.minCount ?? 2,
        max_count: request.maxCount ?? 8,
        string_properties: request.stringProperties ?? {},
        numeric_properties: request.numericProperties ?? {},
      },
      session
    );
  }

  /**
   * Remove from matchmaker queue.
   */
  async removeMatchmaker(session: Session): Promise<{ message: string; ticket_id: string }> {
    return this.request('DELETE', '/api/matchmaker/remove', undefined, session);
  }

  /**
   * Get matchmaker ticket for a user.
   */
  async getMatchmakerTicket(
    session: Session,
    userId?: string
  ): Promise<{
    id: string;
    queue: string;
    players: Array<{ user_id: string; username: string; skill: number }>;
    properties: Record<string, unknown>;
    created_at: number;
  } | null> {
    const uid = userId ?? session.userId;
    try {
      return await this.request('GET', `/api/matchmaker/tickets/${uid}`, undefined, session);
    } catch {
      return null;
    }
  }

  /**
   * List matchmaker queues with stats.
   */
  async listMatchmakerQueues(session: Session): Promise<{
    queues: Array<{
      queue: string;
      tickets: number;
      players: number;
      longest_wait_secs: number;
    }>;
  }> {
    return this.request('GET', '/api/matchmaker/queues', undefined, session);
  }

  // ============================================================================
  // RPC (REST API)
  // ============================================================================

  /**
   * Call a server RPC function via HTTP.
   */
  async rpc<T = unknown, R = unknown>(
    session: Session,
    id: string,
    payload?: T
  ): Promise<R> {
    const response = await this.request<{ payload: R }>(
      'POST',
      `/api/rpc/${id}`,
      { payload },
      session
    );
    return response.payload;
  }
}
