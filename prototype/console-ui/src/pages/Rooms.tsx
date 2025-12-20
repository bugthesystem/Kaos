import { useCallback, useState } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { usePageData } from '../hooks/usePageData';
import { useConfirm } from '../components/ConfirmDialog';
import { useAuth } from '../contexts/AuthContext';
import { formatTimestamp } from '../utils/formatters';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { RoomsIcon, GroupIcon, PlayIcon, LockIcon, RefreshIcon } from '../components/icons';
import type { RoomInfo, RoomPlayerInfo } from '../api/types';

export function RoomsPage() {
  const { hasPermission } = useAuth();
  const canTerminate = hasPermission('terminate:room');

  const fetchRooms = useCallback(() => api.listRooms(1, 100).then(d => d.items), []);
  const { data: rooms, loading, error, selected, drawerOpen, reload, select, closeDrawer } = usePageData({
    fetchFn: fetchRooms,
    refreshInterval: 5000,
  });
  const [players, setPlayers] = useState<RoomPlayerInfo[]>([]);
  const { confirm, ConfirmDialog } = useConfirm();

  const handleRowClick = async (room: RoomInfo) => {
    select(room);
    try {
      const playersData = await api.getRoomPlayers(room.id);
      setPlayers(playersData.players);
    } catch {
      setPlayers([]);
    }
  };

  const handleTerminate = async () => {
    if (!selected) return;
    const confirmed = await confirm({
      title: 'Terminate Room',
      message: `Are you sure you want to terminate room "${selected.label || selected.id.slice(0, 8)}"?`,
      confirmLabel: 'Terminate',
      variant: 'danger',
    });
    if (!confirmed) return;
    try {
      await api.terminateRoom(selected.id);
      closeDrawer();
      reload();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to terminate room');
    }
  };

  const stateCounts = {
    open: rooms.filter(r => r.state === 'open').length,
    running: rooms.filter(r => r.state === 'running').length,
    closed: rooms.filter(r => r.state === 'closed').length,
  };
  const totalPlayers = rooms.reduce((sum, r) => sum + r.player_count, 0);

  const columns: Column<RoomInfo>[] = [
    {
      key: 'id',
      header: 'Room',
      render: (room) => (
        <div className="flex items-center gap-3">
          <RoomAvatar room={room} />
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {room.label || room.id.slice(0, 8)}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {room.id.slice(0, 12)}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'state',
      header: 'Status',
      width: '100px',
      render: (room) => <RoomStateBadge state={room.state} />,
    },
    {
      key: 'player_count',
      header: 'Players',
      width: '100px',
      render: (room) => <span style={{ color: 'var(--text-secondary)' }}>{room.player_count} / {room.max_players}</span>,
    },
    {
      key: 'module',
      header: 'Module',
      width: '120px',
      render: (room) => room.module ? (
        <span className="font-mono text-xs" style={{ color: 'var(--color-accent)' }}>{room.module}</span>
      ) : <span style={{ color: 'var(--text-muted)' }}>—</span>,
    },
    {
      key: 'tick_rate',
      header: 'Tick Rate',
      width: '100px',
      render: (room) => <span className="font-mono" style={{ color: 'var(--text-muted)' }}>{room.tick_rate} Hz</span>,
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {ConfirmDialog}
      <PageHeader
        title="Rooms"
        subtitle="Active game rooms and matches"
        actions={
          <button onClick={reload} className="btn btn-secondary flex items-center gap-2">
            <RefreshIcon className="w-4 h-4" />
            Refresh
          </button>
        }
      />

      {error && <Alert variant="danger">{error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<RoomsIcon className="w-5 h-5" />} label="Total Rooms" value={rooms.length} color="primary" />
        <StatCard icon={<LockIcon className="w-5 h-5" />} label="Open" value={stateCounts.open} color="success" />
        <StatCard icon={<PlayIcon className="w-5 h-5" />} label="Running" value={stateCounts.running} color="info" />
        <StatCard icon={<GroupIcon className="w-5 h-5" />} label="Total Players" value={totalPlayers} color="warning" />
      </StatGrid>

      <div className="card p-0 overflow-hidden">
        <DataTable
          data={rooms}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selected?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search rooms..."
          searchFields={['id', 'label']}
          pagination
          pageSize={15}
          emptyMessage="No rooms found"
        />
      </div>

      <Drawer
        open={drawerOpen}
        onClose={closeDrawer}
        title="Room Details"
        width="md"
        footer={selected && selected.state !== 'closed' && canTerminate && (
          <button onClick={handleTerminate} className="btn btn-danger flex-1">Terminate Room</button>
        )}
      >
        {selected && <RoomDetails room={selected} players={players} />}
      </Drawer>
    </div>
  );
}

// =============================================================================
// Subcomponents
// =============================================================================

function RoomAvatar({ room }: { room: RoomInfo }) {
  const bgColor = room.state === 'open'
    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
    : room.state === 'running'
    ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
    : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)';
  return (
    <div className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold" style={{ background: bgColor, color: 'white' }}>
      <RoomsIcon className="w-5 h-5" />
    </div>
  );
}

function RoomStateBadge({ state }: { state: string }) {
  const variant = state === 'open' ? 'success' : state === 'running' ? 'info' : 'danger';
  return <Badge variant={variant}>{state.charAt(0).toUpperCase() + state.slice(1)}</Badge>;
}

function RoomDetails({ room, players }: { room: RoomInfo; players: RoomPlayerInfo[] }) {
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <div
          className="w-16 h-16 rounded-xl flex items-center justify-center"
          style={{
            background: room.state === 'open'
              ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
              : room.state === 'running'
              ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
              : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
            color: 'white',
          }}
        >
          <RoomsIcon className="w-8 h-8" />
        </div>
        <div>
          <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
            {room.label || room.id.slice(0, 12)}
          </h2>
          <div className="flex items-center gap-2 mt-1">
            <RoomStateBadge state={room.state} />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-3">
        <StatBox label="Players" value={room.player_count.toString()} />
        <StatBox label="Max" value={room.max_players.toString()} />
        <StatBox label="Hz" value={room.tick_rate.toString()} />
      </div>

      <Section title="Room Information">
        <Field label="Room ID" mono>{room.id}</Field>
        <Field label="Label">{room.label || '-'}</Field>
        <Field label="Module" mono>{room.module || '-'}</Field>
        <Field label="State">{room.state}</Field>
        <Field label="Tick Rate">{room.tick_rate} Hz</Field>
        <Field label="Created At">{formatTimestamp(room.created_at)}</Field>
      </Section>

      {players.length > 0 && (
        <Section title="Connected Players">
          <div className="space-y-2">
            {players.map((player) => (
              <PlayerRow key={player.session_id} player={player} />
            ))}
          </div>
        </Section>
      )}
    </div>
  );
}

function PlayerRow({ player }: { player: RoomPlayerInfo }) {
  return (
    <div className="p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
      <div className="flex items-center justify-between">
        <div>
          <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
            {player.username || player.user_id || `Session #${player.session_id}`}
          </div>
          <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>{player.address}</div>
        </div>
        <span className="px-2 py-1 rounded text-xs font-mono" style={{ background: 'var(--bg-secondary)', color: 'var(--text-secondary)' }}>
          #{player.session_id}
        </span>
      </div>
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
