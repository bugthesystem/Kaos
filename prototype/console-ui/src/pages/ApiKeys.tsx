import { useEffect, useState } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import type { ApiKeyInfo, CreateApiKeyResponse } from '../api/types';

function formatDate(timestamp: number | null): string {
  if (!timestamp) return 'Never';
  return new Date(timestamp * 1000).toLocaleString();
}

function formatRelativeTime(timestamp: number | null): string {
  if (!timestamp) return 'Never';
  const seconds = Math.floor(Date.now() / 1000 - timestamp);
  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

export function ApiKeysPage() {
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [newKey, setNewKey] = useState<CreateApiKeyResponse | null>(null);
  const [selectedKey, setSelectedKey] = useState<ApiKeyInfo | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);

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

  const handleRowClick = (key: ApiKeyInfo) => {
    setSelectedKey(key);
    setDrawerOpen(true);
  };

  const handleDelete = async () => {
    if (!selectedKey) return;
    if (!confirm(`Are you sure you want to delete "${selectedKey.name}"?`)) return;
    try {
      await api.deleteApiKey(selectedKey.id);
      setDrawerOpen(false);
      setSelectedKey(null);
      loadKeys();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete API key');
    }
  };

  const activeKeys = keys.filter(k => !k.disabled);
  const totalRequests = keys.reduce((sum, k) => sum + k.request_count, 0);

  const columns: Column<ApiKeyInfo>[] = [
    {
      key: 'name',
      header: 'API Key',
      render: (key) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: key.disabled
                ? 'linear-gradient(135deg, #64748b 0%, #475569 100%)'
                : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
              color: 'white',
            }}
          >
            <KeyIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="font-medium flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
              {key.name}
              {key.disabled && <Badge variant="neutral">Disabled</Badge>}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {key.key_prefix}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'scopes',
      header: 'Scopes',
      render: (key) => (
        <div className="flex flex-wrap gap-1">
          {key.scopes.slice(0, 2).map((scope) => (
            <Badge key={scope} variant="info">
              {scope}
            </Badge>
          ))}
          {key.scopes.length > 2 && (
            <Badge variant="neutral">+{key.scopes.length - 2}</Badge>
          )}
        </div>
      ),
    },
    {
      key: 'request_count',
      header: 'Requests',
      width: '100px',
      render: (key) => (
        <span className="font-mono" style={{ color: 'var(--text-secondary)' }}>
          {key.request_count.toLocaleString()}
        </span>
      ),
    },
    {
      key: 'last_used',
      header: 'Last Used',
      width: '120px',
      render: (key) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(key.last_used)}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">API Keys</h1>
          <p className="page-subtitle">
            Manage API access tokens
          </p>
        </div>
        <button onClick={() => setShowCreate(true)} className="btn btn-primary">
          + New API Key
        </button>
      </div>

      {error && (
        <div className="alert alert-danger">
          {error}
        </div>
      )}

      {newKey && (
        <div className="alert alert-success">
          <div className="flex-1">
            <p className="font-medium mb-2">API Key Created!</p>
            <p className="text-sm mb-2 opacity-80">Copy this key now - it won't be shown again:</p>
            <code className="block bg-black/20 px-3 py-2 rounded text-sm font-mono break-all">
              {newKey.key}
            </code>
          </div>
          <button
            onClick={() => setNewKey(null)}
            className="btn btn-ghost btn-sm"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="stat-card">
          <div className="stat-icon">
            <KeyIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{keys.length}</span>
          <span className="stat-label">Total Keys</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ActiveIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{activeKeys.length}</span>
          <span className="stat-label">Active</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <DisabledIcon className="w-6 h-6" style={{ color: 'var(--color-danger)' }} />
          </div>
          <span className="stat-value">{keys.length - activeKeys.length}</span>
          <span className="stat-label">Disabled</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <RequestsIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{totalRequests.toLocaleString()}</span>
          <span className="stat-label">Total Requests</span>
        </div>
      </div>

      {/* API Keys Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={keys}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedKey?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search API keys..."
          searchFields={['name']}
          pagination
          pageSize={10}
          emptyMessage="No API keys found"
        />
      </div>

      {/* API Key Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="API Key Details"
        width="md"
        footer={
          selectedKey && (
            <button onClick={handleDelete} className="btn btn-danger flex-1">
              Delete API Key
            </button>
          )
        }
      >
        {selectedKey && (
          <div className="space-y-6">
            {/* Key Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center"
                style={{
                  background: selectedKey.disabled
                    ? 'linear-gradient(135deg, #64748b 0%, #475569 100%)'
                    : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                  color: 'white',
                }}
              >
                <KeyIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedKey.name}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={selectedKey.disabled ? 'neutral' : 'success'}>
                    {selectedKey.disabled ? 'Disabled' : 'Active'}
                  </Badge>
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-2 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedKey.request_count.toLocaleString()}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Total Requests</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedKey.scopes.length}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Scopes</div>
              </div>
            </div>

            <Section title="Key Information">
              <Field label="Key ID" mono>
                {selectedKey.id}
              </Field>
              <Field label="Key Prefix" mono>
                {selectedKey.key_prefix}...
              </Field>
              <Field label="Created At">
                {formatDate(selectedKey.created_at)}
              </Field>
              <Field label="Expires At">
                {selectedKey.expires_at ? formatDate(selectedKey.expires_at) : 'Never'}
              </Field>
              <Field label="Last Used">
                {selectedKey.last_used ? formatDate(selectedKey.last_used) : 'Never'}
              </Field>
            </Section>

            <Section title="Scopes">
              <div className="flex flex-wrap gap-2">
                {selectedKey.scopes.map((scope) => (
                  <span
                    key={scope}
                    className="px-3 py-1.5 rounded-lg text-sm"
                    style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}
                  >
                    {scope}
                  </span>
                ))}
              </div>
            </Section>
          </div>
        )}
      </Drawer>

      {/* Create Key Modal */}
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
    { id: 'read:status', label: 'Read Status', desc: 'View server status' },
    { id: 'read:sessions', label: 'Read Sessions', desc: 'List sessions' },
    { id: 'read:rooms', label: 'Read Rooms', desc: 'List rooms' },
    { id: 'read:config', label: 'Read Config', desc: 'View configuration' },
    { id: 'read:lua', label: 'Read Lua', desc: 'View Lua scripts' },
    { id: 'write:kick', label: 'Kick Sessions', desc: 'Kick sessions' },
    { id: 'write:terminate', label: 'Terminate Rooms', desc: 'Terminate rooms' },
    { id: 'write:rpc', label: 'Execute RPC', desc: 'Call Lua functions' },
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
    <div className="modal-overlay">
      <div className="modal" style={{ maxHeight: '90vh', overflowY: 'auto' }}>
        <h2 className="modal-title">Create API Key</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="alert alert-danger text-sm">
              {error}
            </div>
          )}

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input w-full"
              placeholder="My API Key"
              required
              autoFocus
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              Scopes
            </label>
            <div className="space-y-2">
              {availableScopes.map((scope) => (
                <label
                  key={scope.id}
                  className="flex items-center gap-3 p-2 rounded-lg cursor-pointer"
                  style={{ background: scopes.includes(scope.id) ? 'var(--bg-hover)' : 'transparent' }}
                >
                  <input
                    type="checkbox"
                    checked={scopes.includes(scope.id)}
                    onChange={() => toggleScope(scope.id)}
                    className="w-4 h-4 rounded"
                    style={{ accentColor: 'var(--color-accent)' }}
                  />
                  <div>
                    <div className="font-medium text-sm" style={{ color: 'var(--text-primary)' }}>
                      {scope.label}
                    </div>
                    <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
                      {scope.desc}
                    </div>
                  </div>
                </label>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              Expires In (days)
            </label>
            <input
              type="number"
              value={expiresInDays || ''}
              onChange={(e) => setExpiresInDays(e.target.value ? parseInt(e.target.value) : undefined)}
              className="input w-full"
              placeholder="Never expires (leave empty)"
              min="1"
            />
          </div>

          <div className="flex gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">
              Cancel
            </button>
            <button type="submit" disabled={loading || scopes.length === 0} className="btn btn-primary flex-1">
              {loading ? 'Creating...' : 'Create Key'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// Icons
function KeyIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
    </svg>
  );
}

function ActiveIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function DisabledIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
    </svg>
  );
}

function RequestsIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4h16v12a1 1 0 01-1 1H5a1 1 0 01-1-1V4z" />
    </svg>
  );
}
