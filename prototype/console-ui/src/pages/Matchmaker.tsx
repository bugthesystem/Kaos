import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';

interface Queue {
  name: string;
  tickets: number;
  players: number;
}

interface TicketPlayer {
  user_id: string;
  username: string;
  skill: number;
}

interface Ticket {
  id: string;
  queue: string;
  players: TicketPlayer[];
  created_at: number;
}

function formatRelativeTime(ts: number): string {
  const seconds = Math.floor((Date.now() - ts) / 1000);
  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

export default function Matchmaker() {
  const [queues, setQueues] = useState<Queue[]>([]);
  const [selectedQueue, setSelectedQueue] = useState<Queue | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchUserId, setSearchUserId] = useState('');
  const [searchedTickets, setSearchedTickets] = useState<Ticket[]>([]);

  useEffect(() => {
    loadQueues();
    const interval = setInterval(loadQueues, 5000);
    return () => clearInterval(interval);
  }, []);

  const loadQueues = async () => {
    try {
      setLoading(true);
      const data = await api.get('/api/matchmaker/queues');
      setQueues(data.queues || []);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load queues');
    } finally {
      setLoading(false);
    }
  };

  const searchTickets = async () => {
    if (!searchUserId.trim()) return;
    try {
      const data = await api.get(`/api/matchmaker/tickets?user_id=${searchUserId}`);
      setSearchedTickets(data.tickets || []);
    } catch (err) {
      console.error('Failed to search tickets:', err);
      setSearchedTickets([]);
    }
  };

  const handleRowClick = (queue: Queue) => {
    setSelectedQueue(queue);
    setDrawerOpen(true);
  };

  const totalTickets = queues.reduce((sum, q) => sum + q.tickets, 0);
  const totalPlayers = queues.reduce((sum, q) => sum + q.players, 0);
  const avgPlayersPerTicket = totalTickets > 0 ? (totalPlayers / totalTickets).toFixed(1) : '0';

  const columns: Column<Queue>[] = [
    {
      key: 'name',
      header: 'Queue',
      render: (queue) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: queue.tickets > 0
                ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
              color: 'white',
            }}
          >
            <QueueIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {queue.name}
            </div>
            <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
              {queue.tickets > 0 ? 'Active' : 'Idle'}
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'tickets',
      header: 'Tickets',
      width: '120px',
      render: (queue) => (
        <Badge variant={queue.tickets > 0 ? 'success' : 'info'}>
          {queue.tickets} tickets
        </Badge>
      ),
    },
    {
      key: 'players',
      header: 'Players',
      width: '120px',
      render: (queue) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {queue.players} waiting
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Matchmaker</h1>
          <p className="page-subtitle">
            Queue status and ticket search
          </p>
        </div>
        <button onClick={loadQueues} className="btn btn-secondary">
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
            <QueueIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{queues.length}</span>
          <span className="stat-label">Active Queues</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <TicketIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{totalTickets}</span>
          <span className="stat-label">Total Tickets</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <PlayersIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{totalPlayers}</span>
          <span className="stat-label">Players Waiting</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <AvgIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{avgPlayersPerTicket}</span>
          <span className="stat-label">Avg Players/Ticket</span>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Queues Table */}
        <div className="card p-0 overflow-hidden">
          <div className="px-4 py-3 border-b" style={{ borderColor: 'var(--border-primary)', background: 'var(--bg-tertiary)' }}>
            <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Active Queues</h3>
          </div>
          <DataTable
            data={queues}
            columns={columns}
            keyField="name"
            onRowClick={handleRowClick}
            selectedId={selectedQueue?.name}
            loading={loading}
            emptyMessage="No active queues"
          />
        </div>

        {/* Ticket Search */}
        <div className="card">
          <h3 className="font-semibold mb-4" style={{ color: 'var(--text-primary)' }}>Search Tickets</h3>
          <div className="flex gap-2 mb-4">
            <input
              type="text"
              value={searchUserId}
              onChange={(e) => setSearchUserId(e.target.value)}
              placeholder="Enter User ID"
              className="form-input flex-1"
              onKeyDown={(e) => e.key === 'Enter' && searchTickets()}
            />
            <button onClick={searchTickets} className="btn btn-primary">
              Search
            </button>
          </div>

          {searchedTickets.length > 0 ? (
            <div className="space-y-3">
              {searchedTickets.map((ticket) => (
                <div
                  key={ticket.id}
                  className="p-4 rounded-lg"
                  style={{ background: 'var(--bg-tertiary)' }}
                >
                  <div className="flex justify-between items-start mb-2">
                    <div>
                      <span style={{ color: 'var(--text-muted)' }}>Queue: </span>
                      <span className="font-medium" style={{ color: 'var(--color-accent)' }}>
                        {ticket.queue}
                      </span>
                    </div>
                    <span style={{ color: 'var(--text-muted)' }} className="text-sm">
                      {formatRelativeTime(ticket.created_at)}
                    </span>
                  </div>
                  <div className="text-sm font-mono mb-2" style={{ color: 'var(--text-muted)' }}>
                    Ticket: {ticket.id.slice(0, 16)}...
                  </div>
                  <div className="text-sm">
                    <span style={{ color: 'var(--text-muted)' }}>Players:</span>
                    <ul className="mt-2 space-y-1">
                      {ticket.players.map((player) => (
                        <li key={player.user_id} className="flex justify-between items-center">
                          <span style={{ color: 'var(--text-primary)' }}>{player.username}</span>
                          <Badge variant="info">Skill: {player.skill}</Badge>
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              ))}
            </div>
          ) : searchUserId ? (
            <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>
              No tickets found for this user
            </div>
          ) : (
            <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>
              Enter a User ID to search for their tickets
            </div>
          )}
        </div>
      </div>

      {/* Queue Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Queue Details"
        width="md"
      >
        {selectedQueue && (
          <div className="space-y-6">
            {/* Queue Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedQueue.tickets > 0
                    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                    : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                  color: 'white',
                }}
              >
                <QueueIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedQueue.name}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={selectedQueue.tickets > 0 ? 'success' : 'info'}>
                    {selectedQueue.tickets > 0 ? 'Active' : 'Idle'}
                  </Badge>
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-2 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedQueue.tickets}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Tickets</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedQueue.players}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Players</div>
              </div>
            </div>

            <Section title="Queue Information">
              <Field label="Queue Name" mono>
                {selectedQueue.name}
              </Field>
              <Field label="Active Tickets">
                {selectedQueue.tickets}
              </Field>
              <Field label="Players Waiting">
                {selectedQueue.players}
              </Field>
              <Field label="Average Party Size">
                {selectedQueue.tickets > 0 ? (selectedQueue.players / selectedQueue.tickets).toFixed(1) : '-'}
              </Field>
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}

// Icons
function QueueIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
  );
}

function TicketIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 5v2m0 4v2m0 4v2M5 5a2 2 0 00-2 2v3a2 2 0 110 4v3a2 2 0 002 2h14a2 2 0 002-2v-3a2 2 0 110-4V7a2 2 0 00-2-2H5z" />
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

function AvgIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
    </svg>
  );
}
