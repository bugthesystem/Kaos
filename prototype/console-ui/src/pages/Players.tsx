import { useState, useEffect, useCallback } from 'react';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { formatRelativeTime, formatTimestamp } from '../data/sampleData';
import { api } from '../api/client';

// Player type matching backend API response
interface PlayerDevice {
  device_id: string;
  linked_at: number;
}

interface Player {
  id: string;
  username: string | null;
  display_name: string | null;
  email: string | null;
  avatar_url: string | null;
  devices: PlayerDevice[];
  custom_id: string | null;
  created_at: number;
  updated_at: number;
  disabled: boolean;
  metadata: Record<string, unknown>;
  online: boolean;
}

interface PlayersResponse {
  items: Player[];
  total: number;
  page: number;
  page_size: number;
}

export default function Players() {
  const [players, setPlayers] = useState<Player[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedPlayer, setSelectedPlayer] = useState<Player | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const fetchPlayers = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams({
        page: page.toString(),
        page_size: '20',
      });
      if (searchQuery) {
        params.set('search', searchQuery);
      }
      const response = await api.get<PlayersResponse>(`/api/players?${params}`);
      setPlayers(response.items);
      setTotal(response.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch players');
      setPlayers([]);
    } finally {
      setLoading(false);
    }
  }, [page, searchQuery]);

  useEffect(() => {
    fetchPlayers();
  }, [fetchPlayers]);

  const handleRowClick = (player: Player) => {
    setSelectedPlayer(player);
    setDrawerOpen(true);
  };

  const handleBan = async () => {
    if (!selectedPlayer) return;
    const reason = prompt('Ban reason (optional):');
    try {
      await api.post(`/api/players/${selectedPlayer.id}/ban`, { reason });
      // Refresh player data
      setSelectedPlayer({ ...selectedPlayer, disabled: true });
      fetchPlayers();
    } catch (err) {
      alert('Failed to ban player: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const handleUnban = async () => {
    if (!selectedPlayer) return;
    try {
      await api.post(`/api/players/${selectedPlayer.id}/unban`, {});
      setSelectedPlayer({ ...selectedPlayer, disabled: false });
      fetchPlayers();
    } catch (err) {
      alert('Failed to unban player: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const handleDelete = async () => {
    if (!selectedPlayer) return;
    if (!confirm('Are you sure you want to delete this player? This action cannot be undone.')) return;
    try {
      await api.delete(`/api/players/${selectedPlayer.id}`);
      setDrawerOpen(false);
      setSelectedPlayer(null);
      fetchPlayers();
    } catch (err) {
      alert('Failed to delete player: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const handleSearch = (query: string) => {
    setSearchQuery(query);
    setPage(1);
  };

  const columns: Column<Player>[] = [
    {
      key: 'username',
      header: 'Player',
      render: (player) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: player.online
                ? 'linear-gradient(135deg, var(--color-success) 0%, #22c55e 100%)'
                : 'linear-gradient(135deg, var(--color-accent) 0%, #8b5cf6 100%)',
              color: 'white',
            }}
          >
            {(player.username || player.display_name || 'U').charAt(0).toUpperCase()}
          </div>
          <div>
            <div className="font-medium flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
              {player.username || player.display_name || 'Anonymous'}
              {player.online && (
                <span className="w-2 h-2 rounded-full bg-green-500" title="Online" />
              )}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {player.id.slice(0, 8)}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'email',
      header: 'Email',
      render: (player) => (
        <span style={{ color: player.email ? 'var(--text-secondary)' : 'var(--text-muted)' }}>
          {player.email || 'No email'}
        </span>
      ),
    },
    {
      key: 'devices',
      header: 'Devices',
      width: '100px',
      render: (player) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {player.devices.length}
        </span>
      ),
    },
    {
      key: 'created_at',
      header: 'Created',
      width: '140px',
      render: (player) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(player.created_at)}
        </span>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      width: '100px',
      render: (player) =>
        player.disabled ? (
          <Badge variant="danger">Banned</Badge>
        ) : player.online ? (
          <Badge variant="success">Online</Badge>
        ) : (
          <Badge variant="default">Offline</Badge>
        ),
    },
  ];

  const activeCount = players.filter(p => !p.disabled).length;
  const bannedCount = players.filter(p => p.disabled).length;
  const onlineCount = players.filter(p => p.online).length;

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Players</h1>
          <p className="page-subtitle">
            {total} total players
          </p>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="stat-card">
          <div className="stat-icon">
            <UsersIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{total}</span>
          <span className="stat-label">Total Players</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <OnlineIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{onlineCount}</span>
          <span className="stat-label">Online Now</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ActiveIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{activeCount}</span>
          <span className="stat-label">Active</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <BannedIcon className="w-6 h-6" style={{ color: 'var(--color-danger)' }} />
          </div>
          <span className="stat-value">{bannedCount}</span>
          <span className="stat-label">Banned</span>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="p-4 rounded-lg" style={{ background: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.2)' }}>
          <p style={{ color: 'var(--color-danger)' }}>{error}</p>
          <button onClick={fetchPlayers} className="btn btn-secondary mt-2">
            Retry
          </button>
        </div>
      )}

      {/* Players Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={players}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedPlayer?.id}
          searchable
          searchPlaceholder="Search players..."
          onSearch={handleSearch}
          loading={loading}
          pagination
          pageSize={20}
          totalItems={total}
          currentPage={page}
          onPageChange={setPage}
          emptyMessage={searchQuery ? 'No players match your search' : 'No players found'}
        />
      </div>

      {/* Player Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Player Details"
        width="lg"
        footer={
          selectedPlayer && (
            <>
              {selectedPlayer.disabled ? (
                <button onClick={handleUnban} className="btn btn-secondary flex-1">
                  Unban Player
                </button>
              ) : (
                <button onClick={handleBan} className="btn btn-secondary flex-1">
                  Ban Player
                </button>
              )}
              <button onClick={handleDelete} className="btn btn-danger">
                Delete
              </button>
            </>
          )
        }
      >
        {selectedPlayer && (
          <div className="space-y-6">
            {/* Player Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedPlayer.online
                    ? 'linear-gradient(135deg, var(--color-success) 0%, #22c55e 100%)'
                    : 'linear-gradient(135deg, var(--color-accent) 0%, #8b5cf6 100%)',
                  color: 'white',
                }}
              >
                {(selectedPlayer.username || selectedPlayer.display_name || 'U').charAt(0).toUpperCase()}
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedPlayer.username || selectedPlayer.display_name || 'Anonymous'}
                </h2>
                <p style={{ color: 'var(--text-secondary)' }}>
                  {selectedPlayer.display_name || 'No display name'}
                </p>
                <div className="flex gap-2 mt-1">
                  {selectedPlayer.online && <Badge variant="success">Online</Badge>}
                  {selectedPlayer.disabled && <Badge variant="danger">Banned</Badge>}
                </div>
              </div>
            </div>

            <Section title="Account Information">
              <Field label="Player ID" mono>
                {selectedPlayer.id}
              </Field>
              <Field label="Username">
                {selectedPlayer.username || 'Not set'}
              </Field>
              <Field label="Email">
                {selectedPlayer.email || 'Not set'}
              </Field>
              <Field label="Custom ID">
                {selectedPlayer.custom_id || 'Not set'}
              </Field>
              <Field label="Created">
                {formatTimestamp(selectedPlayer.created_at)}
              </Field>
              <Field label="Last Updated">
                {formatTimestamp(selectedPlayer.updated_at)}
              </Field>
            </Section>

            <Section title="Devices">
              {selectedPlayer.devices.length > 0 ? (
                <div className="space-y-2">
                  {selectedPlayer.devices.map((device, i) => (
                    <div
                      key={i}
                      className="flex items-center justify-between p-2 rounded-lg"
                      style={{ background: 'var(--bg-tertiary)' }}
                    >
                      <div className="flex items-center gap-2 font-mono text-sm" style={{ color: 'var(--text-secondary)' }}>
                        <DeviceIcon className="w-4 h-4" />
                        <span>{device.device_id}</span>
                      </div>
                      <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
                        {formatRelativeTime(device.linked_at)}
                      </span>
                    </div>
                  ))}
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)' }}>No devices linked</p>
              )}
            </Section>

            {selectedPlayer.metadata && Object.keys(selectedPlayer.metadata).length > 0 && (
              <Section title="Metadata">
                <pre
                  className="p-3 rounded-lg text-sm font-mono overflow-auto"
                  style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}
                >
                  {JSON.stringify(selectedPlayer.metadata, null, 2)}
                </pre>
              </Section>
            )}

            {selectedPlayer.disabled && (
              <Section title="Ban Status">
                <div className="p-3 rounded-lg" style={{ background: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.2)' }}>
                  <p className="text-sm" style={{ color: 'var(--color-danger)' }}>
                    This player is currently banned and cannot access the game.
                  </p>
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
function UsersIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}

function OnlineIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.636 18.364a9 9 0 010-12.728m12.728 0a9 9 0 010 12.728m-9.9-2.829a5 5 0 010-7.07m7.072 0a5 5 0 010 7.07M13 12a1 1 0 11-2 0 1 1 0 012 0z" />
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

function BannedIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
    </svg>
  );
}

function DeviceIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z" />
    </svg>
  );
}
