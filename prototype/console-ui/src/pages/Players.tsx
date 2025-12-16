import { useState } from 'react';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { samplePlayers, formatRelativeTime, formatTimestamp, type Player } from '../data/sampleData';

const USE_SAMPLE_DATA = true; // Toggle for demo mode

export default function Players() {
  const [players] = useState<Player[]>(USE_SAMPLE_DATA ? samplePlayers : []);
  const [selectedPlayer, setSelectedPlayer] = useState<Player | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const handleRowClick = (player: Player) => {
    setSelectedPlayer(player);
    setDrawerOpen(true);
  };

  const handleBan = () => {
    if (!selectedPlayer) return;
    const reason = prompt('Ban reason (optional):');
    console.log('Banning player:', selectedPlayer.id, 'Reason:', reason);
    // In real app: await api.post(`/api/players/${selectedPlayer.id}/ban`, { reason });
  };

  const handleUnban = () => {
    if (!selectedPlayer) return;
    console.log('Unbanning player:', selectedPlayer.id);
    // In real app: await api.post(`/api/players/${selectedPlayer.id}/unban`, {});
  };

  const handleDelete = () => {
    if (!selectedPlayer) return;
    if (!confirm('Are you sure you want to delete this player?')) return;
    console.log('Deleting player:', selectedPlayer.id);
    setDrawerOpen(false);
    // In real app: await api.delete(`/api/players/${selectedPlayer.id}`);
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
              background: 'linear-gradient(135deg, var(--color-accent) 0%, #8b5cf6 100%)',
              color: 'white',
            }}
          >
            {player.username.charAt(0).toUpperCase()}
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {player.username}
            </div>
            <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
              {player.display_name || 'No display name'}
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'level',
      header: 'Level',
      width: '80px',
      render: (player) => (
        <span className="font-mono font-semibold" style={{ color: 'var(--text-primary)' }}>
          {player.level}
        </span>
      ),
    },
    {
      key: 'games_played',
      header: 'Games',
      width: '100px',
      render: (player) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {player.games_played}
        </span>
      ),
    },
    {
      key: 'win_rate',
      header: 'Win Rate',
      width: '100px',
      render: (player) => {
        const winRate = player.games_played > 0
          ? Math.round((player.wins / player.games_played) * 100)
          : 0;
        return (
          <span
            className="font-medium"
            style={{
              color: winRate >= 60 ? 'var(--color-success)' :
                winRate >= 40 ? 'var(--color-warning)' : 'var(--color-danger)',
            }}
          >
            {winRate}%
          </span>
        );
      },
    },
    {
      key: 'last_seen',
      header: 'Last Seen',
      width: '120px',
      render: (player) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(player.last_seen)}
        </span>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      width: '100px',
      render: (player) =>
        player.banned ? (
          <Badge variant="danger">Banned</Badge>
        ) : (
          <Badge variant="success">Active</Badge>
        ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Players</h1>
          <p className="page-subtitle">
            {players.length} total players
          </p>
        </div>
        <button className="btn btn-primary">
          Add Player
        </button>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="stat-card">
          <div className="stat-icon">
            <UsersIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{players.length}</span>
          <span className="stat-label">Total Players</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ActiveIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{players.filter((p) => !p.banned).length}</span>
          <span className="stat-label">Active</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <BannedIcon className="w-6 h-6" style={{ color: 'var(--color-danger)' }} />
          </div>
          <span className="stat-value">{players.filter((p) => p.banned).length}</span>
          <span className="stat-label">Banned</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <NewIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{players.filter((p) => p.level <= 10).length}</span>
          <span className="stat-label">New (Level 1-10)</span>
        </div>
      </div>

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
          searchFields={['username', 'display_name', 'email']}
          pagination
          pageSize={10}
          emptyMessage="No players found"
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
              {selectedPlayer.banned ? (
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
                  background: 'linear-gradient(135deg, var(--color-accent) 0%, #8b5cf6 100%)',
                  color: 'white',
                }}
              >
                {selectedPlayer.username.charAt(0).toUpperCase()}
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedPlayer.username}
                </h2>
                <p style={{ color: 'var(--text-secondary)' }}>
                  {selectedPlayer.display_name || 'No display name'}
                </p>
                {selectedPlayer.banned && (
                  <Badge variant="danger">Banned</Badge>
                )}
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-4 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedPlayer.level}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Level</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedPlayer.games_played}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Games</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--color-success)' }}>
                  {selectedPlayer.wins}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Wins</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--color-danger)' }}>
                  {selectedPlayer.losses}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Losses</div>
              </div>
            </div>

            <Section title="Account Information">
              <Field label="Player ID" mono>
                {selectedPlayer.id}
              </Field>
              <Field label="Email">
                {selectedPlayer.email}
                {selectedPlayer.email && (
                  <Badge variant={selectedPlayer.email_verified ? 'success' : 'warning'}>
                    {selectedPlayer.email_verified ? 'Verified' : 'Unverified'}
                  </Badge>
                )}
              </Field>
              <Field label="Created">
                {formatTimestamp(selectedPlayer.created_at)}
              </Field>
              <Field label="Last Updated">
                {formatTimestamp(selectedPlayer.updated_at)}
              </Field>
              <Field label="Last Seen">
                {formatRelativeTime(selectedPlayer.last_seen)}
              </Field>
            </Section>

            <Section title="Devices">
              {selectedPlayer.devices.length > 0 ? (
                <div className="space-y-2">
                  {selectedPlayer.devices.map((device, i) => (
                    <div
                      key={i}
                      className="flex items-center gap-2 p-2 rounded-lg font-mono text-sm"
                      style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}
                    >
                      <DeviceIcon className="w-4 h-4" />
                      {device}
                    </div>
                  ))}
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)' }}>No devices linked</p>
              )}
            </Section>

            <Section title="Social Links">
              {selectedPlayer.social_links.length > 0 ? (
                <div className="space-y-2">
                  {selectedPlayer.social_links.map((link, i) => (
                    <div
                      key={i}
                      className="flex items-center justify-between p-2 rounded-lg"
                      style={{ background: 'var(--bg-tertiary)' }}
                    >
                      <div className="flex items-center gap-2 capitalize">
                        <span style={{ color: 'var(--text-primary)' }}>{link.provider}</span>
                      </div>
                      <span className="font-mono text-sm" style={{ color: 'var(--text-secondary)' }}>
                        {link.provider_id}
                      </span>
                    </div>
                  ))}
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)' }}>No social accounts linked</p>
              )}
            </Section>

            {selectedPlayer.banned && selectedPlayer.ban_reason && (
              <Section title="Ban Information">
                <div className="p-3 rounded-lg" style={{ background: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.2)' }}>
                  <p className="text-sm" style={{ color: 'var(--color-danger)' }}>
                    {selectedPlayer.ban_reason}
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

function NewIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
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
