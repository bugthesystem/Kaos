import { useEffect, useState } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import type { RoomInfo } from '../api/types';

function formatTimestamp(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

export function RoomsPage() {
  const [rooms, setRooms] = useState<RoomInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selectedRoom, setSelectedRoom] = useState<RoomInfo | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [players, setPlayers] = useState<number[]>([]);

  useEffect(() => {
    loadRooms();
    const interval = setInterval(loadRooms, 5000);
    return () => clearInterval(interval);
  }, []);

  const loadRooms = async () => {
    try {
      const data = await api.listRooms(1, 100);
      setRooms(data.items);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load rooms');
    } finally {
      setLoading(false);
    }
  };

  const handleRowClick = async (room: RoomInfo) => {
    setSelectedRoom(room);
    setDrawerOpen(true);
    try {
      const playersData = await api.getRoomPlayers(room.id);
      setPlayers(playersData.players);
    } catch {
      setPlayers([]);
    }
  };

  const handleTerminate = async () => {
    if (!selectedRoom) return;
    if (!confirm('Are you sure you want to terminate this room?')) return;
    try {
      await api.terminateRoom(selectedRoom.id);
      setDrawerOpen(false);
      setSelectedRoom(null);
      loadRooms();
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
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: room.state === 'open'
                ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                : room.state === 'running'
                ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
                : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
              color: 'white',
            }}
          >
            <RoomIcon className="w-5 h-5" />
          </div>
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
      render: (room) => (
        <Badge variant={
          room.state === 'open' ? 'success' :
          room.state === 'running' ? 'info' : 'danger'
        }>
          {room.state.charAt(0).toUpperCase() + room.state.slice(1)}
        </Badge>
      ),
    },
    {
      key: 'player_count',
      header: 'Players',
      width: '100px',
      render: (room) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {room.player_count} / {room.max_players}
        </span>
      ),
    },
    {
      key: 'tick_rate',
      header: 'Tick Rate',
      width: '100px',
      render: (room) => (
        <span className="font-mono" style={{ color: 'var(--text-muted)' }}>
          {room.tick_rate} Hz
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Rooms</h1>
          <p className="page-subtitle">
            Active game rooms and matches
          </p>
        </div>
        <button onClick={loadRooms} className="btn btn-secondary">
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
            <RoomIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{rooms.length}</span>
          <span className="stat-label">Total Rooms</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <OpenIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{stateCounts.open}</span>
          <span className="stat-label">Open</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <RunningIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{stateCounts.running}</span>
          <span className="stat-label">Running</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <PlayersIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{totalPlayers}</span>
          <span className="stat-label">Total Players</span>
        </div>
      </div>

      {/* Rooms Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={rooms}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedRoom?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search rooms..."
          searchFields={['id', 'label']}
          pagination
          pageSize={15}
          emptyMessage="No rooms found"
        />
      </div>

      {/* Room Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Room Details"
        width="md"
        footer={
          selectedRoom && selectedRoom.state !== 'closed' && (
            <button onClick={handleTerminate} className="btn btn-danger flex-1">
              Terminate Room
            </button>
          )
        }
      >
        {selectedRoom && (
          <div className="space-y-6">
            {/* Room Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedRoom.state === 'open'
                    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                    : selectedRoom.state === 'running'
                    ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
                    : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
                  color: 'white',
                }}
              >
                <RoomIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedRoom.label || selectedRoom.id.slice(0, 12)}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={
                    selectedRoom.state === 'open' ? 'success' :
                    selectedRoom.state === 'running' ? 'info' : 'danger'
                  }>
                    {selectedRoom.state.charAt(0).toUpperCase() + selectedRoom.state.slice(1)}
                  </Badge>
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-3 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedRoom.player_count}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Players</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedRoom.max_players}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Max</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedRoom.tick_rate}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Hz</div>
              </div>
            </div>

            <Section title="Room Information">
              <Field label="Room ID" mono>
                {selectedRoom.id}
              </Field>
              <Field label="Label">
                {selectedRoom.label || '-'}
              </Field>
              <Field label="State">
                {selectedRoom.state}
              </Field>
              <Field label="Tick Rate">
                {selectedRoom.tick_rate} Hz
              </Field>
              <Field label="Created At">
                {formatTimestamp(selectedRoom.created_at)}
              </Field>
            </Section>

            {players.length > 0 && (
              <Section title="Connected Players">
                <div className="flex flex-wrap gap-2">
                  {players.map((id) => (
                    <span
                      key={id}
                      className="px-3 py-1.5 rounded-lg font-mono text-sm"
                      style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}
                    >
                      Session #{id}
                    </span>
                  ))}
                </div>
              </Section>
            )}
          </div>
        )}
      </Drawer>
    </div>
  );
}

// Icons
function RoomIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
  );
}

function OpenIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z" />
    </svg>
  );
}

function RunningIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function PlayersIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}
