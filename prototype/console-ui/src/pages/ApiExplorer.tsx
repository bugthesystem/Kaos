import { useState, useCallback } from 'react';
import { api } from '../api/client';
import { Badge } from '../components/DataTable';
import { PageHeader, Spinner } from '../components/ui';
import { TerminalIcon } from '../components/icons';

interface ApiEndpoint {
  id: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  path: string;
  description: string;
  category: string;
  params?: { name: string; type: string; required?: boolean; description?: string }[];
  body?: { example: string; description?: string };
}

const API_ENDPOINTS: ApiEndpoint[] = [
  { id: 'status', method: 'GET', path: '/api/status', description: 'Get server status and metrics', category: 'System' },
  { id: 'config', method: 'GET', path: '/api/config', description: 'Get server configuration', category: 'System' },
  { id: 'sessions-list', method: 'GET', path: '/api/sessions', description: 'List active sessions', category: 'Sessions', params: [{ name: 'page', type: 'number' }, { name: 'page_size', type: 'number' }] },
  { id: 'session-get', method: 'GET', path: '/api/sessions/:id', description: 'Get session details', category: 'Sessions', params: [{ name: 'id', type: 'number', required: true }] },
  { id: 'session-kick', method: 'POST', path: '/api/sessions/:id/kick', description: 'Kick a session', category: 'Sessions', params: [{ name: 'id', type: 'number', required: true }] },
  { id: 'rooms-list', method: 'GET', path: '/api/rooms', description: 'List active rooms', category: 'Rooms', params: [{ name: 'page', type: 'number' }, { name: 'page_size', type: 'number' }] },
  { id: 'room-get', method: 'GET', path: '/api/rooms/:id', description: 'Get room details', category: 'Rooms', params: [{ name: 'id', type: 'string', required: true }] },
  { id: 'room-state', method: 'GET', path: '/api/rooms/:id/state', description: 'Get room state', category: 'Rooms', params: [{ name: 'id', type: 'string', required: true }] },
  { id: 'room-terminate', method: 'POST', path: '/api/rooms/:id/terminate', description: 'Terminate a room', category: 'Rooms', params: [{ name: 'id', type: 'string', required: true }] },
  { id: 'players-list', method: 'GET', path: '/api/players', description: 'List player accounts', category: 'Players', params: [{ name: 'page', type: 'number' }, { name: 'page_size', type: 'number' }, { name: 'search', type: 'string' }] },
  { id: 'player-get', method: 'GET', path: '/api/players/:id', description: 'Get player details', category: 'Players', params: [{ name: 'id', type: 'string', required: true }] },
  { id: 'player-ban', method: 'POST', path: '/api/players/:id/ban', description: 'Ban a player', category: 'Players', params: [{ name: 'id', type: 'string', required: true }], body: { example: '{\n  "reason": "Optional ban reason"\n}' } },
  { id: 'player-unban', method: 'POST', path: '/api/players/:id/unban', description: 'Unban a player', category: 'Players', params: [{ name: 'id', type: 'string', required: true }] },
  { id: 'player-delete', method: 'DELETE', path: '/api/players/:id', description: 'Delete a player', category: 'Players', params: [{ name: 'id', type: 'string', required: true }] },
  { id: 'auth-device', method: 'POST', path: '/api/auth/device', description: 'Authenticate with device ID', category: 'Client Auth', body: { example: '{\n  "device_id": "unique-device-id",\n  "create": true,\n  "username": "optional-username"\n}' } },
  { id: 'auth-email', method: 'POST', path: '/api/auth/email', description: 'Authenticate with email/password', category: 'Client Auth', body: { example: '{\n  "email": "user@example.com",\n  "password": "password123",\n  "create": true\n}' } },
  { id: 'storage-list', method: 'GET', path: '/api/storage', description: 'List storage objects', category: 'Storage', params: [{ name: 'collection', type: 'string' }, { name: 'user_id', type: 'string' }, { name: 'limit', type: 'number' }] },
  { id: 'leaderboards-list', method: 'GET', path: '/api/leaderboards', description: 'List leaderboards', category: 'Leaderboards' },
  { id: 'leaderboard-records', method: 'GET', path: '/api/leaderboards/:id/records', description: 'Get leaderboard records', category: 'Leaderboards', params: [{ name: 'id', type: 'string', required: true }, { name: 'limit', type: 'number' }] },
  { id: 'accounts-list', method: 'GET', path: '/api/accounts', description: 'List console admin accounts', category: 'Console' },
  { id: 'apikeys-list', method: 'GET', path: '/api/keys', description: 'List API keys', category: 'Console' },
  { id: 'matchmaker-queues', method: 'GET', path: '/api/matchmaker/queues', description: 'List matchmaker queues with stats', category: 'Matchmaker' },
  { id: 'matchmaker-tickets', method: 'GET', path: '/api/matchmaker/tickets', description: 'Get user matchmaker ticket', category: 'Matchmaker', params: [{ name: 'user_id', type: 'string', description: 'User ID to look up ticket for' }] },
  { id: 'chat-channels', method: 'GET', path: '/api/chat/channels', description: 'List chat channels', category: 'Chat', params: [{ name: 'page', type: 'number' }, { name: 'page_size', type: 'number' }] },
  { id: 'groups-list', method: 'GET', path: '/api/groups', description: 'List groups', category: 'Social', params: [{ name: 'page', type: 'number' }, { name: 'page_size', type: 'number' }] },
  { id: 'tournaments-list', method: 'GET', path: '/api/tournaments', description: 'List tournaments', category: 'Tournaments', params: [{ name: 'page', type: 'number' }, { name: 'page_size', type: 'number' }] },
];

interface RequestHistory { id: string; method: string; path: string; status: number; duration: number; timestamp: Date; response: string; success: boolean; }

export default function ApiExplorer() {
  const [selectedEndpoint, setSelectedEndpoint] = useState<ApiEndpoint | null>(null);
  const [paramValues, setParamValues] = useState<Record<string, string>>({});
  const [bodyValue, setBodyValue] = useState('');
  const [response, setResponse] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [statusCode, setStatusCode] = useState<number | null>(null);
  const [duration, setDuration] = useState<number | null>(null);
  const [history, setHistory] = useState<RequestHistory[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);

  const categories = [...new Set(API_ENDPOINTS.map(e => e.category))];
  const filteredEndpoints = API_ENDPOINTS.filter(endpoint => {
    const matchesSearch = !searchQuery || endpoint.path.toLowerCase().includes(searchQuery.toLowerCase()) || endpoint.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = !selectedCategory || endpoint.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const selectEndpoint = useCallback((endpoint: ApiEndpoint) => {
    setSelectedEndpoint(endpoint);
    setParamValues({});
    setBodyValue(endpoint.body?.example || '');
    setResponse(null);
    setError(null);
    setStatusCode(null);
    setDuration(null);
  }, []);

  const buildPath = useCallback(() => {
    if (!selectedEndpoint) return '';
    let path = selectedEndpoint.path;
    const pathParams = selectedEndpoint.params?.filter(p => path.includes(`:${p.name}`)) || [];
    for (const param of pathParams) { const value = paramValues[param.name]; if (value) path = path.replace(`:${param.name}`, encodeURIComponent(value)); }
    const queryParams = selectedEndpoint.params?.filter(p => !selectedEndpoint.path.includes(`:${p.name}`)) || [];
    const queryParts: string[] = [];
    for (const param of queryParams) { const value = paramValues[param.name]; if (value) queryParts.push(`${param.name}=${encodeURIComponent(value)}`); }
    if (queryParts.length > 0) path += `?${queryParts.join('&')}`;
    return path;
  }, [selectedEndpoint, paramValues]);

  const executeRequest = async () => {
    if (!selectedEndpoint) return;
    setLoading(true); setError(null); setResponse(null);
    const startTime = Date.now();
    try {
      const path = buildPath();
      let result: unknown;
      switch (selectedEndpoint.method) {
        case 'GET': result = await api.get(path); break;
        case 'POST': result = await api.post(path, bodyValue ? JSON.parse(bodyValue) : undefined); break;
        case 'PUT': result = await api.put(path, bodyValue ? JSON.parse(bodyValue) : undefined); break;
        case 'DELETE': result = await api.delete(path); break;
      }
      const responseStr = JSON.stringify(result, null, 2);
      const dur = Date.now() - startTime;
      setResponse(responseStr); setStatusCode(200); setDuration(dur);
      setHistory(prev => [{ id: Date.now().toString(), method: selectedEndpoint.method, path, status: 200, duration: dur, timestamp: new Date(), response: responseStr, success: true }, ...prev.slice(0, 49)]);
    } catch (err) {
      const dur = Date.now() - startTime;
      const errMsg = err instanceof Error ? err.message : 'Unknown error';
      setError(errMsg); setStatusCode(400); setDuration(dur);
      setHistory(prev => [{ id: Date.now().toString(), method: selectedEndpoint.method, path: buildPath(), status: 400, duration: dur, timestamp: new Date(), response: errMsg, success: false }, ...prev.slice(0, 49)]);
    } finally { setLoading(false); }
  };

  const methodColor = (method: string) => {
    switch (method) {
      case 'GET': return 'var(--color-success)';
      case 'POST': return 'var(--color-info)';
      case 'PUT': return 'var(--color-warning)';
      case 'DELETE': return 'var(--color-danger)';
      default: return 'var(--text-secondary)';
    }
  };

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader title="API Explorer" subtitle="Test and explore KaosNet API endpoints" />

      <div className="grid grid-cols-12 gap-6">
        <div className="col-span-4">
          <div className="card p-0 overflow-hidden">
            <div className="p-4 border-b" style={{ borderColor: 'var(--border-primary)' }}>
              <input type="text" placeholder="Search endpoints..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="form-input w-full mb-3" />
              <div className="flex flex-wrap gap-2">
                <button onClick={() => setSelectedCategory(null)} className={`px-2 py-1 text-xs rounded ${!selectedCategory ? 'font-semibold' : ''}`} style={{ background: !selectedCategory ? 'var(--color-accent)' : 'var(--bg-tertiary)', color: !selectedCategory ? 'white' : 'var(--text-secondary)' }}>All</button>
                {categories.map(cat => <button key={cat} onClick={() => setSelectedCategory(cat)} className={`px-2 py-1 text-xs rounded ${selectedCategory === cat ? 'font-semibold' : ''}`} style={{ background: selectedCategory === cat ? 'var(--color-accent)' : 'var(--bg-tertiary)', color: selectedCategory === cat ? 'white' : 'var(--text-secondary)' }}>{cat}</button>)}
              </div>
            </div>
            <div className="max-h-[600px] overflow-y-auto">
              {filteredEndpoints.map(endpoint => (
                <button key={endpoint.id} onClick={() => selectEndpoint(endpoint)} className="w-full text-left p-3 border-b transition-colors" style={{ borderColor: 'var(--border-primary)', background: selectedEndpoint?.id === endpoint.id ? 'var(--bg-tertiary)' : 'transparent' }}>
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-xs font-mono font-semibold px-1.5 py-0.5 rounded" style={{ background: 'var(--bg-tertiary)', color: methodColor(endpoint.method) }}>{endpoint.method}</span>
                    <span className="text-sm font-mono" style={{ color: 'var(--text-primary)' }}>{endpoint.path}</span>
                  </div>
                  <p className="text-xs" style={{ color: 'var(--text-muted)' }}>{endpoint.description}</p>
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="col-span-8 space-y-4">
          {selectedEndpoint ? (
            <>
              <div className="card p-4">
                <div className="flex items-center gap-3 mb-4">
                  <span className="text-sm font-mono font-bold px-2 py-1 rounded" style={{ background: 'var(--bg-tertiary)', color: methodColor(selectedEndpoint.method) }}>{selectedEndpoint.method}</span>
                  <span className="text-lg font-mono flex-1" style={{ color: 'var(--text-primary)' }}>{buildPath() || selectedEndpoint.path}</span>
                  <button onClick={executeRequest} disabled={loading} className="btn btn-primary">{loading ? 'Sending...' : 'Send'}</button>
                </div>
                {selectedEndpoint.params && selectedEndpoint.params.length > 0 && (
                  <div className="mb-4">
                    <h4 className="text-sm font-semibold mb-2" style={{ color: 'var(--text-primary)' }}>Parameters</h4>
                    <div className="grid grid-cols-2 gap-3">
                      {selectedEndpoint.params.map(param => (
                        <div key={param.name}>
                          <label className="text-xs block mb-1" style={{ color: 'var(--text-secondary)' }}>{param.name}{param.required && <span style={{ color: 'var(--color-danger)' }}> *</span>}<span className="ml-1 opacity-50">({param.type})</span></label>
                          <input type="text" value={paramValues[param.name] || ''} onChange={(e) => setParamValues(prev => ({ ...prev, [param.name]: e.target.value }))} placeholder={param.name} className="form-input w-full text-sm font-mono" />
                        </div>
                      ))}
                    </div>
                  </div>
                )}
                {selectedEndpoint.body && (
                  <div>
                    <h4 className="text-sm font-semibold mb-2" style={{ color: 'var(--text-primary)' }}>Request Body</h4>
                    <textarea value={bodyValue} onChange={(e) => setBodyValue(e.target.value)} rows={8} className="form-input w-full text-sm font-mono" style={{ resize: 'vertical' }} />
                  </div>
                )}
              </div>

              <div className="card p-4">
                <div className="flex items-center justify-between mb-3">
                  <h4 className="text-sm font-semibold" style={{ color: 'var(--text-primary)' }}>Response</h4>
                  {statusCode !== null && (
                    <div className="flex items-center gap-3">
                      <Badge variant={statusCode < 400 ? 'success' : 'danger'}>{statusCode}</Badge>
                      {duration !== null && <span className="text-xs" style={{ color: 'var(--text-muted)' }}>{duration}ms</span>}
                    </div>
                  )}
                </div>
                {loading && <div className="p-8 text-center"><Spinner /></div>}
                {error && <pre className="p-4 rounded text-sm font-mono overflow-auto" style={{ background: 'rgba(239, 68, 68, 0.1)', color: 'var(--color-danger)', maxHeight: '400px' }}>{error}</pre>}
                {response && <pre className="p-4 rounded text-sm font-mono overflow-auto" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)', maxHeight: '400px' }}>{response}</pre>}
                {!loading && !error && !response && <p className="text-center py-8" style={{ color: 'var(--text-muted)' }}>Click "Send" to execute the request</p>}
              </div>
            </>
          ) : (
            <div className="card p-8 text-center">
              <div className="w-16 h-16 rounded-full mx-auto mb-4 flex items-center justify-center" style={{ background: 'var(--bg-tertiary)' }}><TerminalIcon className="w-8 h-8" style={{ color: 'var(--text-muted)' }} /></div>
              <h3 className="text-lg font-semibold mb-2" style={{ color: 'var(--text-primary)' }}>Select an Endpoint</h3>
              <p style={{ color: 'var(--text-muted)' }}>Choose an API endpoint from the list to get started</p>
            </div>
          )}

          {history.length > 0 && (
            <div className="card p-4">
              <h4 className="text-sm font-semibold mb-3" style={{ color: 'var(--text-primary)' }}>Recent Requests</h4>
              <div className="space-y-2 max-h-48 overflow-y-auto">
                {history.slice(0, 10).map(item => (
                  <div key={item.id} className="flex items-center gap-3 p-2 rounded text-sm" style={{ background: 'var(--bg-tertiary)' }}>
                    <span className="text-xs font-mono font-semibold" style={{ color: methodColor(item.method) }}>{item.method}</span>
                    <span className="font-mono flex-1 truncate" style={{ color: 'var(--text-secondary)' }}>{item.path}</span>
                    <Badge variant={item.success ? 'success' : 'danger'}>{item.status}</Badge>
                    <span className="text-xs" style={{ color: 'var(--text-muted)' }}>{item.duration}ms</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
