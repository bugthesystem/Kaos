import { useState, useEffect, useCallback } from 'react';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { useConfirm } from '../components/ConfirmDialog';
import { formatRelativeTime, formatTimestamp } from '../utils/formatters';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { UsersIcon, ConnectionIcon, CheckIcon, XIcon } from '../components/icons';
import { api } from '../api/client';

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
  const { confirm, ConfirmDialog } = useConfirm();

  const fetchPlayers = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams({ page: page.toString(), page_size: '20' });
      if (searchQuery) params.set('search', searchQuery);
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

  useEffect(() => { fetchPlayers(); }, [fetchPlayers]);

  const handleRowClick = (player: Player) => {
    setSelectedPlayer(player);
    setDrawerOpen(true);
  };

  const handleBan = async () => {
    if (!selectedPlayer) return;
    const reason = prompt('Ban reason (optional):');
    try {
      await api.post(`/api/players/${selectedPlayer.id}/ban`, { reason });
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
    const confirmed = await confirm({
      title: 'Delete Player',
      message: 'Are you sure you want to delete this player? This action cannot be undone.',
      confirmLabel: 'Delete',
      variant: 'danger',
    });
    if (!confirmed) return;
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
          <PlayerAvatar player={player} />
          <div>
            <div className="font-medium flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
              {player.username || player.display_name || 'Anonymous'}
              {player.online && <span className="w-2 h-2 rounded-full bg-green-500" title="Online" />}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>{player.id.slice(0, 8)}...</div>
          </div>
        </div>
      ),
    },
    { key: 'email', header: 'Email', render: (p) => <span style={{ color: p.email ? 'var(--text-secondary)' : 'var(--text-muted)' }}>{p.email || 'No email'}</span> },
    { key: 'devices', header: 'Devices', width: '100px', render: (p) => <span style={{ color: 'var(--text-secondary)' }}>{p.devices.length}</span> },
    { key: 'created_at', header: 'Created', width: '140px', render: (p) => <span style={{ color: 'var(--text-muted)' }}>{formatRelativeTime(p.created_at)}</span> },
    {
      key: 'status', header: 'Status', width: '100px',
      render: (p) => p.disabled ? <Badge variant="danger">Banned</Badge> : p.online ? <Badge variant="success">Online</Badge> : <Badge variant="default">Offline</Badge>,
    },
  ];

  const activeCount = players.filter(p => !p.disabled).length;
  const bannedCount = players.filter(p => p.disabled).length;
  const onlineCount = players.filter(p => p.online).length;

  return (
    <div className="space-y-6 animate-fade-in">
      {ConfirmDialog}
      <PageHeader title="Players" subtitle={`${total} total players`} />

      <StatGrid columns={4}>
        <StatCard icon={<UsersIcon className="w-5 h-5" />} label="Total Players" value={total} color="primary" />
        <StatCard icon={<ConnectionIcon className="w-5 h-5" />} label="Online Now" value={onlineCount} color="success" />
        <StatCard icon={<CheckIcon className="w-5 h-5" />} label="Active" value={activeCount} color="info" />
        <StatCard icon={<XIcon className="w-5 h-5" />} label="Banned" value={bannedCount} color="danger" />
      </StatGrid>

      {error && <Alert variant="danger" onDismiss={() => setError(null)}>{error}<button onClick={fetchPlayers} className="btn btn-secondary btn-sm ml-2">Retry</button></Alert>}

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

      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Player Details"
        width="lg"
        footer={selectedPlayer && (
          <>
            {selectedPlayer.disabled ? (
              <button onClick={handleUnban} className="btn btn-secondary flex-1">Unban Player</button>
            ) : (
              <button onClick={handleBan} className="btn btn-secondary flex-1">Ban Player</button>
            )}
            <button onClick={handleDelete} className="btn btn-danger">Delete</button>
          </>
        )}
      >
        {selectedPlayer && <PlayerDetails player={selectedPlayer} />}
      </Drawer>
    </div>
  );
}

function PlayerAvatar({ player }: { player: Player }) {
  const bgColor = player.online
    ? 'linear-gradient(135deg, var(--color-success) 0%, #22c55e 100%)'
    : 'linear-gradient(135deg, var(--color-accent) 0%, #8b5cf6 100%)';
  return (
    <div className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold" style={{ background: bgColor, color: 'white' }}>
      {(player.username || player.display_name || 'U').charAt(0).toUpperCase()}
    </div>
  );
}

function PlayerDetails({ player }: { player: Player }) {
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <div className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
          style={{ background: player.online ? 'linear-gradient(135deg, var(--color-success) 0%, #22c55e 100%)' : 'linear-gradient(135deg, var(--color-accent) 0%, #8b5cf6 100%)', color: 'white' }}>
          {(player.username || player.display_name || 'U').charAt(0).toUpperCase()}
        </div>
        <div>
          <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>{player.username || player.display_name || 'Anonymous'}</h2>
          <p style={{ color: 'var(--text-secondary)' }}>{player.display_name || 'No display name'}</p>
          <div className="flex gap-2 mt-1">
            {player.online && <Badge variant="success">Online</Badge>}
            {player.disabled && <Badge variant="danger">Banned</Badge>}
          </div>
        </div>
      </div>

      <Section title="Account Information">
        <Field label="Player ID" mono>{player.id}</Field>
        <Field label="Username">{player.username || 'Not set'}</Field>
        <Field label="Email">{player.email || 'Not set'}</Field>
        <Field label="Custom ID">{player.custom_id || 'Not set'}</Field>
        <Field label="Created">{formatTimestamp(player.created_at)}</Field>
        <Field label="Last Updated">{formatTimestamp(player.updated_at)}</Field>
      </Section>

      <Section title="Devices">
        {player.devices.length > 0 ? (
          <div className="space-y-2">
            {player.devices.map((device, i) => (
              <div key={i} className="flex items-center justify-between p-2 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="flex items-center gap-2 font-mono text-sm" style={{ color: 'var(--text-secondary)' }}>
                  <DeviceIcon className="w-4 h-4" />
                  <span>{device.device_id}</span>
                </div>
                <span className="text-xs" style={{ color: 'var(--text-muted)' }}>{formatRelativeTime(device.linked_at)}</span>
              </div>
            ))}
          </div>
        ) : <p style={{ color: 'var(--text-muted)' }}>No devices linked</p>}
      </Section>

      {player.metadata && Object.keys(player.metadata).length > 0 && (
        <Section title="Metadata">
          <pre className="p-3 rounded-lg text-sm font-mono overflow-auto" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}>
            {JSON.stringify(player.metadata, null, 2)}
          </pre>
        </Section>
      )}

      {player.disabled && (
        <Section title="Ban Status">
          <div className="p-3 rounded-lg" style={{ background: 'rgba(239, 68, 68, 0.1)', border: '1px solid rgba(239, 68, 68, 0.2)' }}>
            <p className="text-sm" style={{ color: 'var(--color-danger)' }}>This player is currently banned and cannot access the game.</p>
          </div>
        </Section>
      )}
    </div>
  );
}

function DeviceIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z" />
    </svg>
  );
}
