import { useCallback } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { usePageData } from '../hooks/usePageData';
import { useConfirm } from '../components/ConfirmDialog';
import { formatDuration, formatTimestamp } from '../utils/formatters';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { UsersIcon, ShieldIcon, ConnectionIcon, ClockIcon, RefreshIcon } from '../components/icons';
import type { SessionInfo } from '../api/types';

export function SessionsPage() {
  const fetchSessions = useCallback(() => api.listSessions(1, 100).then(d => d.items), []);
  const { data: sessions, loading, error, selected, drawerOpen, reload, select, closeDrawer } = usePageData({
    fetchFn: fetchSessions,
    refreshInterval: 5000,
  });
  const { confirm, ConfirmDialog } = useConfirm();

  const handleKick = async () => {
    if (!selected) return;
    const confirmed = await confirm({
      title: 'Kick Session',
      message: `Are you sure you want to kick session #${selected.id}?`,
      confirmLabel: 'Kick',
      variant: 'danger',
    });
    if (!confirmed) return;
    try {
      await api.kickSession(selected.id);
      closeDrawer();
      reload();
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
          <SessionAvatar session={session} />
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
      render: (session) => <SessionStateBadge state={session.state} />,
    },
    {
      key: 'room_id',
      header: 'Room',
      width: '150px',
      render: (session) => session.room_id ? (
        <span className="font-mono text-xs" style={{ color: 'var(--color-info)' }}>
          {session.room_id.substring(0, 8)}...
        </span>
      ) : <span style={{ color: 'var(--text-muted)' }}>—</span>,
    },
    {
      key: 'connected_at',
      header: 'Duration',
      width: '100px',
      render: (session) => (
        <span style={{ color: 'var(--text-secondary)' }}>{formatDuration(Math.floor(Date.now() / 1000) - session.connected_at)}</span>
      ),
    },
    {
      key: 'last_heartbeat',
      header: 'Last Activity',
      width: '120px',
      render: (session) => (
        <span style={{ color: 'var(--text-muted)' }}>{formatDuration(Math.floor(Date.now() / 1000) - session.last_heartbeat)} ago</span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {ConfirmDialog}
      <PageHeader
        title="Sessions"
        subtitle="Manage connected clients"
        actions={
          <button onClick={reload} className="btn btn-secondary flex items-center gap-2">
            <RefreshIcon className="w-4 h-4" />
            Refresh
          </button>
        }
      />

      {error && <Alert variant="danger">{error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<UsersIcon className="w-5 h-5" />} label="Total Sessions" value={sessions.length} color="primary" />
        <StatCard icon={<ShieldIcon className="w-5 h-5" />} label="Authenticated" value={stateCounts.authenticated} color="success" />
        <StatCard icon={<ConnectionIcon className="w-5 h-5" />} label="Connected" value={stateCounts.connected} color="info" />
        <StatCard icon={<ClockIcon className="w-5 h-5" />} label="Connecting" value={stateCounts.connecting} color="warning" />
      </StatGrid>

      <div className="card p-0 overflow-hidden">
        <DataTable
          data={sessions}
          columns={columns}
          keyField="id"
          onRowClick={select}
          selectedId={selected?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search sessions..."
          searchFields={['username', 'address', 'user_id']}
          pagination
          pageSize={15}
          emptyMessage="No sessions found"
        />
      </div>

      <Drawer
        open={drawerOpen}
        onClose={closeDrawer}
        title="Session Details"
        width="md"
        footer={selected && (
          <button onClick={handleKick} className="btn btn-danger flex-1">Kick Session</button>
        )}
      >
        {selected && <SessionDetails session={selected} />}
      </Drawer>
    </div>
  );
}

// =============================================================================
// Subcomponents
// =============================================================================

function SessionAvatar({ session }: { session: SessionInfo }) {
  const bgColor = session.state === 'authenticated'
    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
    : session.state === 'connected'
    ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
    : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)';
  return (
    <div className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold" style={{ background: bgColor, color: 'white' }}>
      #{session.id}
    </div>
  );
}

function SessionStateBadge({ state }: { state: string }) {
  const variant = state === 'authenticated' ? 'success' : state === 'connected' ? 'info' : 'warning';
  return <Badge variant={variant}>{state.charAt(0).toUpperCase() + state.slice(1)}</Badge>;
}

function SessionDetails({ session }: { session: SessionInfo }) {
  const connectedDuration = formatDuration(Math.floor(Date.now() / 1000) - session.connected_at);
  const lastActivityDuration = formatDuration(Math.floor(Date.now() / 1000) - session.last_heartbeat);

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <div
          className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
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
          <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
            {session.username || session.user_id || `Session #${session.id}`}
          </h2>
          <div className="flex items-center gap-2 mt-1">
            <SessionStateBadge state={session.state} />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <StatBox label="Connected" value={connectedDuration} />
        <StatBox label="Last Activity" value={lastActivityDuration} />
      </div>

      <Section title="Connection Information">
        <Field label="Session ID" mono>{session.id}</Field>
        <Field label="IP Address" mono>{session.address}</Field>
        <Field label="Connected At">{formatTimestamp(session.connected_at)}</Field>
        <Field label="Last Heartbeat">{formatTimestamp(session.last_heartbeat)}</Field>
      </Section>

      <Section title="User Information">
        <Field label="User ID" mono>{session.user_id || '-'}</Field>
        <Field label="Username">{session.username || '-'}</Field>
      </Section>

      <Section title="Game">
        <Field label="Room ID" mono>{session.room_id || 'Not in a room'}</Field>
      </Section>
    </div>
  );
}

function StatBox({ label, value }: { label: string; value: string }) {
  return (
    <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
      <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{value}</div>
      <div className="text-xs" style={{ color: 'var(--text-muted)' }}>{label}</div>
    </div>
  );
}
