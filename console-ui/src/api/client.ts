import type {
  ServerStatus,
  SessionInfo,
  RoomInfo,
  AccountInfo,
  ApiKeyInfo,
  LuaScriptInfo,
  RpcInfo,
  PaginatedList,
  LoginRequest,
  LoginResponse,
  CreateAccountRequest,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
} from './types';

const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:7350';

class ApiClient {
  private token: string | null = null;

  setToken(token: string | null) {
    this.token = token;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    const response = await fetch(`${API_BASE}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    return response.json();
  }

  // Auth
  async login(data: LoginRequest): Promise<LoginResponse> {
    return this.request('POST', '/api/auth/login', data);
  }

  async logout(): Promise<void> {
    await this.request('POST', '/api/auth/logout');
  }

  async me(): Promise<AccountInfo> {
    return this.request('GET', '/api/auth/me');
  }

  // Status
  async getStatus(): Promise<ServerStatus> {
    return this.request('GET', '/api/status');
  }

  async getConfig(): Promise<Record<string, unknown>> {
    return this.request('GET', '/api/config');
  }

  // Sessions
  async listSessions(page = 1, pageSize = 20): Promise<PaginatedList<SessionInfo>> {
    return this.request('GET', `/api/sessions?page=${page}&page_size=${pageSize}`);
  }

  async getSession(id: number): Promise<SessionInfo> {
    return this.request('GET', `/api/sessions/${id}`);
  }

  async kickSession(id: number): Promise<void> {
    await this.request('POST', `/api/sessions/${id}/kick`);
  }

  // Rooms
  async listRooms(page = 1, pageSize = 20): Promise<PaginatedList<RoomInfo>> {
    return this.request('GET', `/api/rooms?page=${page}&page_size=${pageSize}`);
  }

  async getRoom(id: string): Promise<RoomInfo> {
    return this.request('GET', `/api/rooms/${id}`);
  }

  async getRoomState(id: string): Promise<{ room_id: string; state: string; state_size: number; players: number[] }> {
    return this.request('GET', `/api/rooms/${id}/state`);
  }

  async getRoomPlayers(id: string): Promise<{ room_id: string; players: number[]; player_count: number }> {
    return this.request('GET', `/api/rooms/${id}/players`);
  }

  async terminateRoom(id: string): Promise<void> {
    await this.request('POST', `/api/rooms/${id}/terminate`);
  }

  // Accounts
  async listAccounts(page = 1, pageSize = 20): Promise<PaginatedList<AccountInfo>> {
    return this.request('GET', `/api/accounts?page=${page}&page_size=${pageSize}`);
  }

  async createAccount(data: CreateAccountRequest): Promise<AccountInfo> {
    return this.request('POST', '/api/accounts', data);
  }

  async getAccount(id: string): Promise<AccountInfo> {
    return this.request('GET', `/api/accounts/${id}`);
  }

  async updateAccount(id: string, data: Partial<{ username: string; role: string; disabled: boolean }>): Promise<AccountInfo> {
    return this.request('PUT', `/api/accounts/${id}`, data);
  }

  async deleteAccount(id: string): Promise<void> {
    await this.request('DELETE', `/api/accounts/${id}`);
  }

  async changePassword(id: string, password: string): Promise<void> {
    await this.request('POST', `/api/accounts/${id}/password`, { password });
  }

  // API Keys
  async listApiKeys(page = 1, pageSize = 20): Promise<PaginatedList<ApiKeyInfo>> {
    return this.request('GET', `/api/keys?page=${page}&page_size=${pageSize}`);
  }

  async createApiKey(data: CreateApiKeyRequest): Promise<CreateApiKeyResponse> {
    return this.request('POST', '/api/keys', data);
  }

  async getApiKey(id: string): Promise<ApiKeyInfo> {
    return this.request('GET', `/api/keys/${id}`);
  }

  async deleteApiKey(id: string): Promise<void> {
    await this.request('DELETE', `/api/keys/${id}`);
  }

  async getApiKeyUsage(id: string): Promise<{ id: string; name: string; request_count: number; last_used: number | null }> {
    return this.request('GET', `/api/keys/${id}/usage`);
  }

  // Lua
  async listScripts(page = 1, pageSize = 20): Promise<PaginatedList<LuaScriptInfo>> {
    return this.request('GET', `/api/lua/scripts?page=${page}&page_size=${pageSize}`);
  }

  async getScript(name: string): Promise<LuaScriptInfo> {
    return this.request('GET', `/api/lua/scripts/${name}`);
  }

  async listRpcs(page = 1, pageSize = 20): Promise<PaginatedList<RpcInfo>> {
    return this.request('GET', `/api/lua/rpcs?page=${page}&page_size=${pageSize}`);
  }

  async executeRpc(name: string, payload?: unknown): Promise<{ rpc: string; result: unknown; duration_ms: number }> {
    return this.request('POST', `/api/lua/rpcs/${name}/execute`, { name, payload });
  }

  async reloadScripts(): Promise<{ message: string; count: number }> {
    return this.request('POST', '/api/lua/reload');
  }
}

export const api = new ApiClient();
