import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { RoomsIcon, DatabaseIcon, UsersIcon, ChartIcon, RefreshIcon } from '../components/icons';
import { formatRelativeTime } from '../utils/formatters';

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
          <QueueAvatar queue={queue} size="sm" />
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>{queue.name}</div>
            <div className="text-xs" style={{ color: 'var(--text-muted)' }}>{queue.tickets > 0 ? 'Active' : 'Idle'}</div>
          </div>
        </div>
      ),
    },
    {
      key: 'tickets',
      header: 'Tickets',
      width: '120px',
      render: (queue) => <Badge variant={queue.tickets > 0 ? 'success' : 'info'}>{queue.tickets} tickets</Badge>,
    },
    {
      key: 'players',
      header: 'Players',
      width: '120px',
      render: (queue) => <span style={{ color: 'var(--text-secondary)' }}>{queue.players} waiting</span>,
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader title="Matchmaker" subtitle="Queue status and ticket search">
        <button onClick={loadQueues} className="btn btn-secondary"><RefreshIcon className="w-4 h-4" /></button>
      </PageHeader>

      {error && <Alert variant="danger" onDismiss={() => setError('')}>{error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<RoomsIcon className="w-5 h-5" />} label="Active Queues" value={queues.length} color="primary" />
        <StatCard icon={<DatabaseIcon className="w-5 h-5" />} label="Total Tickets" value={totalTickets} color="success" />
        <StatCard icon={<UsersIcon className="w-5 h-5" />} label="Players Waiting" value={totalPlayers} color="warning" />
        <StatCard icon={<ChartIcon className="w-5 h-5" />} label="Avg Players/Ticket" value={avgPlayersPerTicket} color="info" />
      </StatGrid>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
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
            <button onClick={searchTickets} className="btn btn-primary">Search</button>
          </div>

          {searchedTickets.length > 0 ? (
            <div className="space-y-3">
              {searchedTickets.map((ticket) => (
                <div key={ticket.id} className="p-4 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                  <div className="flex justify-between items-start mb-2">
                    <div>
                      <span style={{ color: 'var(--text-muted)' }}>Queue: </span>
                      <span className="font-medium" style={{ color: 'var(--color-accent)' }}>{ticket.queue}</span>
                    </div>
                    <span style={{ color: 'var(--text-muted)' }} className="text-sm">{formatRelativeTime(ticket.created_at)}</span>
                  </div>
                  <div className="text-sm font-mono mb-2" style={{ color: 'var(--text-muted)' }}>Ticket: {ticket.id.slice(0, 16)}...</div>
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
            <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>No tickets found for this user</div>
          ) : (
            <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>Enter a User ID to search for their tickets</div>
          )}
        </div>
      </div>

      <Drawer open={drawerOpen} onClose={() => setDrawerOpen(false)} title="Queue Details" width="md">
        {selectedQueue && <QueueDetails queue={selectedQueue} />}
      </Drawer>
    </div>
  );
}

function QueueAvatar({ queue, size = 'sm' }: { queue: Queue; size?: 'sm' | 'lg' }) {
  const sizeClasses = size === 'lg' ? 'w-16 h-16' : 'w-9 h-9';
  const iconSize = size === 'lg' ? 'w-8 h-8' : 'w-5 h-5';
  const bg = queue.tickets > 0 ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)' : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)';
  return (
    <div className={`${sizeClasses} rounded-${size === 'lg' ? 'xl' : 'lg'} flex items-center justify-center`} style={{ background: bg, color: 'white' }}>
      <RoomsIcon className={iconSize} />
    </div>
  );
}

function QueueDetails({ queue }: { queue: Queue }) {
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <QueueAvatar queue={queue} size="lg" />
        <div>
          <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>{queue.name}</h2>
          <div className="flex items-center gap-2 mt-1">
            <Badge variant={queue.tickets > 0 ? 'success' : 'info'}>{queue.tickets > 0 ? 'Active' : 'Idle'}</Badge>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
          <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{queue.tickets}</div>
          <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Tickets</div>
        </div>
        <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
          <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{queue.players}</div>
          <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Players</div>
        </div>
      </div>

      <Section title="Queue Information">
        <Field label="Queue Name" mono>{queue.name}</Field>
        <Field label="Active Tickets">{queue.tickets}</Field>
        <Field label="Players Waiting">{queue.players}</Field>
        <Field label="Average Party Size">{queue.tickets > 0 ? (queue.players / queue.tickets).toFixed(1) : '-'}</Field>
      </Section>
    </div>
  );
}
