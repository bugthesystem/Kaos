/**
 * KaosNet Socket - Real-time WebSocket connection
 */

import type {
  ClientOptions,
  Session,
  SocketOptions,
  MatchmakerAddRequest,
  MatchmakerTicket,
  MatchmakerMatch,
  Match,
  Presence,
  MatchState,
  KaosNetEvents,
  ConnectionState,
} from './types';

type EventCallback = (...args: unknown[]) => void;

/**
 * WebSocket opcodes matching the server protocol.
 */
export enum OpCode {
  SessionStart = 0x01,
  SessionEnd = 0x02,
  Heartbeat = 0x03,
  SessionAck = 0x04,

  RoomCreate = 0x10,
  RoomJoin = 0x11,
  RoomLeave = 0x12,
  RoomData = 0x13,
  RoomState = 0x14,
  RoomList = 0x15,

  Rpc = 0x20,
  RpcResponse = 0x21,

  MatchmakerAdd = 0x30,
  MatchmakerRemove = 0x31,
  MatchmakerMatched = 0x32,

  Error = 0xff,
}

const DEFAULT_SOCKET_OPTIONS: Required<SocketOptions> = {
  enablePrediction: false,
  enableInterpolation: true,
  interpolationBuffer: 100,
};

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
export class KaosSocket {
  private ws: WebSocket | null = null;
  private readonly wsUrl: string;
  private readonly clientOptions: Required<ClientOptions>;
  private readonly socketOptions: Required<SocketOptions>;
  private session: Session | null = null;
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private rpcCallbacks = new Map<number, { resolve: (value: unknown) => void; reject: (error: Error) => void }>();
  private rpcIdCounter = 0;
  private currentMatchId: string | null = null;

  // Event handlers
  private eventListeners: Map<keyof KaosNetEvents, Set<EventCallback>> = new Map();

  // Convenience event handlers (Nakama-style)
  onConnect?: () => void;
  onDisconnect?: (reason?: string) => void;
  onError?: (error: Error) => void;
  onMatchmakerMatched?: (match: MatchmakerMatch) => void;
  onMatchPresenceJoin?: (matchId: string, presences: Presence[]) => void;
  onMatchPresenceLeave?: (matchId: string, presences: Presence[]) => void;
  onMatchState?: <T>(state: MatchState<T>) => void;

  private _state: ConnectionState = 'disconnected';

  constructor(wsUrl: string, clientOptions: Required<ClientOptions>, socketOptions?: SocketOptions) {
    this.wsUrl = wsUrl;
    this.clientOptions = clientOptions;
    this.socketOptions = { ...DEFAULT_SOCKET_OPTIONS, ...socketOptions };
  }

  get state(): ConnectionState {
    return this._state;
  }

  get isConnected(): boolean {
    return this._state === 'connected' && this.ws?.readyState === WebSocket.OPEN;
  }

  private log(...args: unknown[]): void {
    if (this.clientOptions.verbose) {
      console.log('[KaosSocket]', ...args);
    }
  }

  private emit<K extends keyof KaosNetEvents>(event: K, ...args: Parameters<KaosNetEvents[K]>): void {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      for (const listener of listeners) {
        try {
          listener(...args);
        } catch (e) {
          console.error(`Error in ${event} listener:`, e);
        }
      }
    }

    // Also call convenience handlers
    switch (event) {
      case 'connect':
        this.onConnect?.();
        break;
      case 'disconnect':
        this.onDisconnect?.(args[0] as string | undefined);
        break;
      case 'error':
        this.onError?.(args[0] as Error);
        break;
      case 'matchmakerMatched':
        this.onMatchmakerMatched?.(args[0] as MatchmakerMatch);
        break;
      case 'matchPresenceJoin':
        this.onMatchPresenceJoin?.(args[0] as string, args[1] as Presence[]);
        break;
      case 'matchPresenceLeave':
        this.onMatchPresenceLeave?.(args[0] as string, args[1] as Presence[]);
        break;
      case 'matchState':
        this.onMatchState?.(args[0] as MatchState);
        break;
    }
  }

  /**
   * Add an event listener.
   */
  on<K extends keyof KaosNetEvents>(event: K, callback: KaosNetEvents[K]): () => void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, new Set());
    }
    this.eventListeners.get(event)!.add(callback as EventCallback);
    return () => this.off(event, callback);
  }

  /**
   * Remove an event listener.
   */
  off<K extends keyof KaosNetEvents>(event: K, callback: KaosNetEvents[K]): void {
    this.eventListeners.get(event)?.delete(callback as EventCallback);
  }

  /**
   * Add a one-time event listener.
   */
  once<K extends keyof KaosNetEvents>(event: K, callback: KaosNetEvents[K]): () => void {
    const wrapper = ((...args: unknown[]) => {
      this.off(event, wrapper as KaosNetEvents[K]);
      (callback as EventCallback)(...args);
    }) as KaosNetEvents[K];
    return this.on(event, wrapper);
  }

  /**
   * Connect to the game server.
   */
  async connect(session: Session): Promise<void> {
    if (this._state === 'connected' || this._state === 'connecting') {
      throw new Error('Already connected or connecting');
    }

    this.session = session;
    this._state = 'connecting';

    return new Promise((resolve, reject) => {
      const url = `${this.wsUrl}?token=${encodeURIComponent(session.token)}`;
      this.log('Connecting to', this.wsUrl);

      try {
        this.ws = new WebSocket(url);
        this.ws.binaryType = 'arraybuffer';
      } catch (e) {
        this._state = 'disconnected';
        reject(e);
        return;
      }

      const timeout = setTimeout(() => {
        this.ws?.close();
        this._state = 'disconnected';
        reject(new Error('Connection timeout'));
      }, this.clientOptions.timeout);

      this.ws.onopen = () => {
        clearTimeout(timeout);
        this._state = 'connected';
        this.reconnectAttempts = 0;
        this.startHeartbeat();
        this.log('Connected');
        this.emit('connect');
        resolve();
      };

      this.ws.onerror = (event) => {
        clearTimeout(timeout);
        const error = new Error('WebSocket error');
        this.log('Error:', error);
        this.emit('error', error);
        if (this._state === 'connecting') {
          reject(error);
        }
      };

      this.ws.onclose = (event) => {
        clearTimeout(timeout);
        this.stopHeartbeat();
        const wasConnected = this._state === 'connected';
        const wasConnecting = this._state === 'connecting';
        this._state = 'disconnected';
        this.log('Disconnected:', event.reason || 'Connection closed');
        this.emit('disconnect', event.reason);

        if (wasConnected && this.clientOptions.autoReconnect) {
          this.attemptReconnect();
        }

        if (wasConnecting) {
          reject(new Error(event.reason || 'Connection failed'));
        }
      };

      this.ws.onmessage = (event) => {
        this.handleMessage(event.data);
      };
    });
  }

  /**
   * Disconnect from the server.
   */
  disconnect(): void {
    this.clientOptions.autoReconnect = false;
    this.clearReconnectTimer();
    this.stopHeartbeat();
    this.ws?.close(1000, 'Client disconnect');
    this.ws = null;
    this._state = 'disconnected';
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.clientOptions.maxReconnectAttempts) {
      this.log('Max reconnect attempts reached');
      return;
    }

    this._state = 'reconnecting';
    this.reconnectAttempts++;
    const delay = this.clientOptions.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    this.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
    this.emit('reconnect', this.reconnectAttempts);

    this.reconnectTimer = setTimeout(async () => {
      if (this.session) {
        try {
          await this.connect(this.session);
        } catch (e) {
          // Will trigger another reconnect via onclose
        }
      }
    }, delay);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private startHeartbeat(): void {
    this.heartbeatTimer = setInterval(() => {
      this.sendOpcode(OpCode.Heartbeat);
    }, 30000);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private sendOpcode(op: OpCode, payload?: Uint8Array): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('Not connected');
    }

    const payloadLen = payload?.length ?? 0;
    const buffer = new ArrayBuffer(4 + payloadLen);
    const view = new DataView(buffer);

    view.setUint8(0, op);
    view.setUint8(1, 0); // flags
    view.setUint16(2, payloadLen, true); // little-endian

    if (payload) {
      new Uint8Array(buffer, 4).set(payload);
    }

    this.ws.send(buffer);
  }

  private sendJson(op: OpCode, data: unknown): void {
    const json = JSON.stringify(data);
    const encoder = new TextEncoder();
    const payload = encoder.encode(json);
    this.sendOpcode(op, payload);
  }

  private handleMessage(data: ArrayBuffer | string): void {
    // Handle both binary and text messages
    if (typeof data === 'string') {
      // JSON message (legacy/game state format)
      try {
        const parsed = JSON.parse(data);
        this.handleJsonMessage(parsed);
      } catch (e) {
        this.log('Failed to parse JSON message:', e);
      }
      return;
    }

    // Binary protocol
    const view = new DataView(data);
    if (data.byteLength < 4) {
      this.log('Invalid message: too short');
      return;
    }

    const op = view.getUint8(0);
    // const flags = view.getUint8(1);
    const len = view.getUint16(2, true);

    if (data.byteLength < 4 + len) {
      this.log('Invalid message: payload truncated');
      return;
    }

    const payload = new Uint8Array(data, 4, len);
    this.handleBinaryMessage(op, payload);
  }

  private handleJsonMessage(data: unknown): void {
    // Game state broadcast (works both with and without match ID for simpler games)
    this.emit('matchState', {
      opcode: 0,
      data,
      sender: undefined,
    });
  }

  private handleBinaryMessage(op: number, payload: Uint8Array): void {
    const decoder = new TextDecoder();

    switch (op) {
      case OpCode.SessionAck:
        this.log('Session acknowledged');
        break;

      case OpCode.Heartbeat:
        // Server heartbeat response
        break;

      case OpCode.RoomState:
        if (this.currentMatchId) {
          try {
            const json = JSON.parse(decoder.decode(payload));
            this.emit('matchState', {
              opcode: json.opcode ?? 0,
              data: json.data ?? json,
              sender: json.sender,
            });
          } catch {
            this.emit('matchState', {
              opcode: 0,
              data: payload,
              sender: undefined,
            });
          }
        }
        break;

      case OpCode.RpcResponse: {
        const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
        const id = view.getUint32(0, true);
        const responseData = decoder.decode(payload.slice(4));
        const callback = this.rpcCallbacks.get(id);
        if (callback) {
          this.rpcCallbacks.delete(id);
          try {
            callback.resolve(JSON.parse(responseData));
          } catch {
            callback.resolve(responseData);
          }
        }
        break;
      }

      case OpCode.MatchmakerMatched: {
        try {
          const match = JSON.parse(decoder.decode(payload)) as MatchmakerMatch;
          this.emit('matchmakerMatched', match);
        } catch (e) {
          this.log('Failed to parse matchmaker match:', e);
        }
        break;
      }

      case OpCode.Error: {
        const error = decoder.decode(payload);
        this.log('Server error:', error);
        this.emit('error', new Error(error));
        break;
      }

      default:
        this.log('Unknown opcode:', op);
    }
  }

  // ============================================================================
  // Matchmaking
  // ============================================================================

  /**
   * Add to matchmaking queue.
   */
  async addMatchmaker(request: MatchmakerAddRequest): Promise<MatchmakerTicket> {
    return this.rpc<MatchmakerAddRequest, MatchmakerTicket>('matchmaker.add', request);
  }

  /**
   * Remove from matchmaking queue.
   */
  async removeMatchmaker(ticketId: string): Promise<void> {
    await this.rpc('matchmaker.remove', { ticket_id: ticketId });
  }

  // ============================================================================
  // Match / Room
  // ============================================================================

  /**
   * Create a new match.
   */
  async createMatch(name?: string): Promise<Match> {
    const match = await this.rpc<{ name?: string }, Match>('match.create', { name });
    this.currentMatchId = match.matchId;
    return match;
  }

  /**
   * Join an existing match.
   */
  async joinMatch(matchId: string, metadata?: Record<string, string>): Promise<Match> {
    const match = await this.rpc<{ match_id: string; metadata?: Record<string, string> }, Match>(
      'match.join',
      { match_id: matchId, metadata }
    );
    this.currentMatchId = matchId;
    return match;
  }

  /**
   * Leave the current match.
   */
  async leaveMatch(): Promise<void> {
    if (!this.currentMatchId) return;
    await this.rpc('match.leave', { match_id: this.currentMatchId });
    this.currentMatchId = null;
  }

  /**
   * Send match state to other players.
   */
  sendMatchState<T = unknown>(opcode: number, data: T, presences?: Presence[]): void {
    if (!this.currentMatchId) {
      throw new Error('Not in a match');
    }

    this.sendJson(OpCode.RoomData, {
      match_id: this.currentMatchId,
      opcode,
      data,
      presences: presences?.map(p => p.sessionId),
    });
  }

  /**
   * Send raw data to the server (for simple games).
   * This is a convenience method for games that don't use the full protocol.
   */
  send<T = unknown>(data: T): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('Not connected');
    }
    this.ws.send(JSON.stringify(data));
  }

  // ============================================================================
  // RPC
  // ============================================================================

  /**
   * Call a server RPC function.
   */
  async rpc<T = unknown, R = unknown>(id: string, payload?: T): Promise<R> {
    return new Promise((resolve, reject) => {
      const rpcId = ++this.rpcIdCounter;
      this.rpcCallbacks.set(rpcId, { resolve: resolve as (v: unknown) => void, reject });

      // Build RPC payload
      const encoder = new TextEncoder();
      const idBytes = encoder.encode(id);
      const payloadBytes = payload ? encoder.encode(JSON.stringify(payload)) : new Uint8Array(0);

      const buffer = new Uint8Array(4 + 1 + idBytes.length + payloadBytes.length);
      const view = new DataView(buffer.buffer);
      view.setUint32(0, rpcId, true);
      buffer[4] = idBytes.length;
      buffer.set(idBytes, 5);
      buffer.set(payloadBytes, 5 + idBytes.length);

      try {
        this.sendOpcode(OpCode.Rpc, buffer);
      } catch (e) {
        this.rpcCallbacks.delete(rpcId);
        reject(e);
      }

      // Timeout
      setTimeout(() => {
        if (this.rpcCallbacks.has(rpcId)) {
          this.rpcCallbacks.delete(rpcId);
          reject(new Error(`RPC timeout: ${id}`));
        }
      }, this.clientOptions.timeout);
    });
  }
}
