import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { ApiKeyInfo, CreateApiKeyResponse } from '../api/types';

function formatDate(timestamp: number | null): string {
  if (!timestamp) return 'Never';
  return new Date(timestamp * 1000).toLocaleString();
}

export function ApiKeysPage() {
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [newKey, setNewKey] = useState<CreateApiKeyResponse | null>(null);

  useEffect(() => {
    loadKeys();
  }, []);

  const loadKeys = async () => {
    setLoading(true);
    try {
      const data = await api.listApiKeys();
      setKeys(data.items);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load API keys');
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Are you sure you want to delete this API key?')) return;
    try {
      await api.deleteApiKey(id);
      loadKeys();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete API key');
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">API Keys</h1>
          <p className="text-gray-400">Manage API access tokens</p>
        </div>
        <button onClick={() => setShowCreate(true)} className="btn btn-primary">
          + New API Key
        </button>
      </div>

      {error && (
        <div className="bg-red-900/50 border border-red-700 text-red-300 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      {newKey && (
        <div className="bg-green-900/50 border border-green-700 text-green-300 px-4 py-3 rounded-lg">
          <p className="font-medium mb-2">API Key Created!</p>
          <p className="text-sm mb-2">Copy this key now - it won't be shown again:</p>
          <code className="block bg-gray-900 px-3 py-2 rounded text-sm font-mono break-all">
            {newKey.key}
          </code>
          <button
            onClick={() => setNewKey(null)}
            className="mt-3 text-sm text-green-400 hover:text-green-300"
          >
            Dismiss
          </button>
        </div>
      )}

      <div className="card p-0 overflow-hidden">
        <table className="w-full">
          <thead className="bg-gray-900/50">
            <tr>
              <th className="table-header">Name</th>
              <th className="table-header">Key Prefix</th>
              <th className="table-header">Scopes</th>
              <th className="table-header">Last Used</th>
              <th className="table-header">Requests</th>
              <th className="table-header">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-700">
            {loading ? (
              <tr>
                <td colSpan={6} className="table-cell text-center text-gray-400">
                  Loading...
                </td>
              </tr>
            ) : keys.length === 0 ? (
              <tr>
                <td colSpan={6} className="table-cell text-center text-gray-400">
                  No API keys found
                </td>
              </tr>
            ) : (
              keys.map((key) => (
                <tr key={key.id} className="hover:bg-gray-700/50">
                  <td className="table-cell">
                    <span className="font-medium text-white">{key.name}</span>
                    {key.disabled && (
                      <span className="ml-2 badge badge-danger">Disabled</span>
                    )}
                  </td>
                  <td className="table-cell font-mono text-xs text-gray-400">
                    {key.key_prefix}...
                  </td>
                  <td className="table-cell">
                    <div className="flex flex-wrap gap-1">
                      {key.scopes.slice(0, 2).map((scope) => (
                        <span key={scope} className="badge badge-info text-xs">
                          {scope}
                        </span>
                      ))}
                      {key.scopes.length > 2 && (
                        <span className="badge text-xs">
                          +{key.scopes.length - 2}
                        </span>
                      )}
                    </div>
                  </td>
                  <td className="table-cell text-gray-400 text-sm">
                    {formatDate(key.last_used)}
                  </td>
                  <td className="table-cell text-gray-400">
                    {key.request_count}
                  </td>
                  <td className="table-cell">
                    <button
                      onClick={() => handleDelete(key.id)}
                      className="btn btn-danger btn-sm"
                    >
                      Delete
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {showCreate && (
        <CreateKeyModal
          onClose={() => setShowCreate(false)}
          onCreated={(key) => {
            setShowCreate(false);
            setNewKey(key);
            loadKeys();
          }}
        />
      )}
    </div>
  );
}

function CreateKeyModal({ onClose, onCreated }: { onClose: () => void; onCreated: (key: CreateApiKeyResponse) => void }) {
  const [name, setName] = useState('');
  const [scopes, setScopes] = useState<string[]>(['read:status']);
  const [expiresInDays, setExpiresInDays] = useState<number | undefined>();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const availableScopes = [
    'read:status',
    'read:sessions',
    'read:rooms',
    'read:config',
    'read:lua',
    'write:kick',
    'write:terminate',
    'write:rpc',
  ];

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');

    try {
      const key = await api.createApiKey({ name, scopes, expires_in_days: expiresInDays });
      onCreated(key);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create API key');
    } finally {
      setLoading(false);
    }
  };

  const toggleScope = (scope: string) => {
    setScopes((prev) =>
      prev.includes(scope)
        ? prev.filter((s) => s !== scope)
        : [...prev, scope]
    );
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50">
      <div className="card max-w-md w-full max-h-[90vh] overflow-y-auto">
        <h2 className="text-xl font-bold text-white mb-4">Create API Key</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="bg-red-900/50 border border-red-700 text-red-300 px-4 py-3 rounded-lg text-sm">
              {error}
            </div>
          )}

          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input"
              placeholder="My API Key"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Scopes
            </label>
            <div className="grid grid-cols-2 gap-2">
              {availableScopes.map((scope) => (
                <label key={scope} className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={scopes.includes(scope)}
                    onChange={() => toggleScope(scope)}
                    className="rounded bg-gray-700 border-gray-600 text-sky-600 focus:ring-sky-500"
                  />
                  <span className="text-gray-300">{scope}</span>
                </label>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Expires In (days, optional)
            </label>
            <input
              type="number"
              value={expiresInDays || ''}
              onChange={(e) => setExpiresInDays(e.target.value ? parseInt(e.target.value) : undefined)}
              className="input"
              placeholder="Never expires"
              min="1"
            />
          </div>

          <div className="flex gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">
              Cancel
            </button>
            <button type="submit" disabled={loading || scopes.length === 0} className="btn btn-primary flex-1">
              {loading ? 'Creating...' : 'Create'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
