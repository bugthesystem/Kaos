import { useEffect, useState } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import type { SessionInfo } from '../api/types';

function formatDuration(startTimestamp: number): string {
  const seconds = Math.floor(Date.now() / 1000 - startTimestamp);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

function formatTimestamp(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

export function SessionsPage() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selectedSession, setSelectedSession] = useState<SessionInfo | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);

  useEffect(() => {
    loadSessions();
    const interval = setInterval(loadSessions, 5000);
    return () => clearInterval(interval);
  }, []);

  const loadSessions = async () => {
    try {
      const data = await api.listSessions(1, 100);
      setSessions(data.items);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load sessions');
    } finally {
      setLoading(false);
    }
  };

  const handleRowClick = (session: SessionInfo) => {
    setSelectedSession(session);
    setDrawerOpen(true);
  };

  const handleKick = async () => {
    if (!selectedSession) return;
    if (!confirm('Are you sure you want to kick this session?')) return;
    try {
      await api.kickSession(selectedSession.id);
      setDrawerOpen(false);
      setSelectedSession(null);
      loadSessions();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to kick session');
    }
  };

  const stateCounts = {
    authenticated: sessions.filter(s => s.state === 'authenticated').length,
    connected: sessions.filter(s => s.state === 'connected').length,
    connecting: sessions.filter(s => s.state === 'connecting').length,
  };

  const columns: Column<SessionInfo>[] = [
    {
      key: 'id',
      header: 'Session',
      render: (session) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: session.state === 'authenticated'
                ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                : session.state === 'connected'
                ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
                : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
              color: 'white',
            }}
          >
            #{session.id}
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {session.username || session.user_id || `Session #${session.id}`}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {session.address}
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'state',
      header: 'Status',
      width: '130px',
      render: (session) => (
        <Badge variant={
          session.state === 'authenticated' ? 'success' :
          session.state === 'connected' ? 'info' : 'warning'
        }>
          {session.state.charAt(0).toUpperCase() + session.state.slice(1)}
        </Badge>
      ),
    },
    {
      key: 'room_id',
      header: 'Room',
      width: '150px',
      render: (session) => (
        session.room_id ? (
          <span className="font-mono text-xs" style={{ color: 'var(--color-info)' }}>
            {session.room_id.substring(0, 8)}...
          </span>
        ) : (
          <span style={{ color: 'var(--text-muted)' }}>—</span>
        )
      ),
    },
    {
      key: 'connected_at',
      header: 'Duration',
      width: '100px',
      render: (session) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {formatDuration(session.connected_at)}
        </span>
      ),
    },
    {
      key: 'last_heartbeat',
      header: 'Last Activity',
      width: '120px',
      render: (session) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatDuration(session.last_heartbeat)} ago
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Sessions</h1>
          <p className="page-subtitle">
            Manage connected clients
          </p>
        </div>
        <button onClick={loadSessions} className="btn btn-secondary">
          Refresh
        </button>
      </div>

      {error && (
        <div className="alert alert-danger">
          {error}
        </div>
      )}

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="stat-card">
          <div className="stat-icon">
            <UsersIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{sessions.length}</span>
          <span className="stat-label">Total Sessions</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <AuthIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{stateCounts.authenticated}</span>
          <span className="stat-label">Authenticated</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ConnectedIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{stateCounts.connected}</span>
          <span className="stat-label">Connected</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <PendingIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{stateCounts.connecting}</span>
          <span className="stat-label">Connecting</span>
        </div>
      </div>

      {/* Sessions Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={sessions}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedSession?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search sessions..."
          searchFields={['username', 'address', 'user_id']}
          pagination
          pageSize={15}
          emptyMessage="No sessions found"
        />
      </div>

      {/* Session Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Session Details"
        width="md"
        footer={
          selectedSession && (
            <button onClick={handleKick} className="btn btn-danger flex-1">
              Kick Session
            </button>
          )
        }
      >
        {selectedSession && (
          <div className="space-y-6">
            {/* Session Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedSession.state === 'authenticated'
                    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                    : selectedSession.state === 'connected'
                    ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
                    : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
                  color: 'white',
                }}
              >
                #{selectedSession.id}
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedSession.username || selectedSession.user_id || `Session #${selectedSession.id}`}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={
                    selectedSession.state === 'authenticated' ? 'success' :
                    selectedSession.state === 'connected' ? 'info' : 'warning'
                  }>
                    {selectedSession.state.charAt(0).toUpperCase() + selectedSession.state.slice(1)}
                  </Badge>
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-2 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {formatDuration(selectedSession.connected_at)}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Connected</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {formatDuration(selectedSession.last_heartbeat)}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Last Activity</div>
              </div>
            </div>

            <Section title="Connection Information">
              <Field label="Session ID" mono>
                {selectedSession.id}
              </Field>
              <Field label="IP Address" mono>
                {selectedSession.address}
              </Field>
              <Field label="Connected At">
                {formatTimestamp(selectedSession.connected_at)}
              </Field>
              <Field label="Last Heartbeat">
                {formatTimestamp(selectedSession.last_heartbeat)}
              </Field>
            </Section>

            <Section title="User Information">
              <Field label="User ID" mono>
                {selectedSession.user_id || '-'}
              </Field>
              <Field label="Username">
                {selectedSession.username || '-'}
              </Field>
            </Section>

            <Section title="Game">
              <Field label="Room ID" mono>
                {selectedSession.room_id || 'Not in a room'}
              </Field>
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}

// Icons
function UsersIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}

function AuthIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
    </svg>
  );
}

function ConnectedIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
    </svg>
  );
}

function PendingIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}
