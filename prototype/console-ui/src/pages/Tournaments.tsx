import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';

interface Tournament {
  id: string;
  title: string;
  description: string;
  category: number;
  sort_order: string;
  size: number;
  max_size: number;
  max_num_score: number;
  start_time: number;
  end_time: number | null;
  duration: number;
  reset_schedule: string | null;
  metadata: any;
  created_at: number;
}

interface TournamentRecord {
  owner_id: string;
  username: string;
  score: number;
  num_score: number;
  rank: number;
  metadata: any;
  updated_at: number;
}

function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString();
}

function formatRelativeTime(ts: number): string {
  const seconds = Math.floor((Date.now() - ts) / 1000);
  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function formatDuration(secs: number): string {
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
  return `${Math.floor(secs / 86400)}d`;
}

export default function Tournaments() {
  const [tournaments, setTournaments] = useState<Tournament[]>([]);
  const [records, setRecords] = useState<TournamentRecord[]>([]);
  const [selectedTournament, setSelectedTournament] = useState<Tournament | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newTournament, setNewTournament] = useState({
    id: '',
    title: '',
    description: '',
    category: 0,
    sort_order: 'descending',
    max_size: 100,
    max_num_score: 1000000,
    duration: 86400,
  });

  useEffect(() => {
    loadTournaments();
  }, []);

  useEffect(() => {
    if (selectedTournament) {
      loadRecords(selectedTournament.id);
    }
  }, [selectedTournament]);

  const loadTournaments = async () => {
    try {
      setLoading(true);
      const data = await api.get('/api/tournaments');
      setTournaments(data.tournaments || []);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load tournaments');
    } finally {
      setLoading(false);
    }
  };

  const loadRecords = async (tournamentId: string) => {
    try {
      const data = await api.get(`/api/tournaments/${tournamentId}/records?limit=100`);
      setRecords(data.records || []);
    } catch (err) {
      console.error('Failed to load records:', err);
      setRecords([]);
    }
  };

  const createTournament = async () => {
    if (!newTournament.id.trim() || !newTournament.title.trim()) return;
    try {
      await api.post('/api/tournaments', newTournament);
      setShowCreateModal(false);
      setNewTournament({
        id: '',
        title: '',
        description: '',
        category: 0,
        sort_order: 'descending',
        max_size: 100,
        max_num_score: 1000000,
        duration: 86400,
      });
      loadTournaments();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to create tournament');
    }
  };

  const deleteTournament = async () => {
    if (!selectedTournament) return;
    if (!confirm('Are you sure you want to delete this tournament?')) return;
    try {
      await api.delete(`/api/tournaments/${selectedTournament.id}`);
      setDrawerOpen(false);
      setSelectedTournament(null);
      loadTournaments();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete tournament');
    }
  };

  const handleRowClick = (tournament: Tournament) => {
    setSelectedTournament(tournament);
    setDrawerOpen(true);
  };

  const getTournamentStatus = (t: Tournament): { label: string; variant: 'success' | 'warning' | 'info' | 'danger' } => {
    const now = Date.now();
    if (now < t.start_time) return { label: 'Upcoming', variant: 'warning' };
    if (t.end_time && now > t.end_time) return { label: 'Ended', variant: 'info' };
    return { label: 'Active', variant: 'success' };
  };

  const activeTournaments = tournaments.filter(t => {
    const now = Date.now();
    return now >= t.start_time && (!t.end_time || now <= t.end_time);
  }).length;

  const upcomingTournaments = tournaments.filter(t => Date.now() < t.start_time).length;
  const totalParticipants = tournaments.reduce((sum, t) => sum + t.size, 0);

  const columns: Column<Tournament>[] = [
    {
      key: 'title',
      header: 'Tournament',
      render: (tournament) => {
        const status = getTournamentStatus(tournament);
        return (
          <div className="flex items-center gap-3">
            <div
              className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
              style={{
                background: status.variant === 'success'
                  ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                  : status.variant === 'warning'
                  ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)'
                  : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                color: 'white',
              }}
            >
              <TrophyIcon className="w-5 h-5" />
            </div>
            <div>
              <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
                {tournament.title || tournament.id}
              </div>
              <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
                {tournament.id}
              </div>
            </div>
          </div>
        );
      },
    },
    {
      key: 'status',
      header: 'Status',
      width: '100px',
      render: (tournament) => {
        const status = getTournamentStatus(tournament);
        return <Badge variant={status.variant}>{status.label}</Badge>;
      },
    },
    {
      key: 'size',
      header: 'Participants',
      width: '130px',
      render: (tournament) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {tournament.size} / {tournament.max_size}
        </span>
      ),
    },
    {
      key: 'duration',
      header: 'Duration',
      width: '100px',
      render: (tournament) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatDuration(tournament.duration)}
        </span>
      ),
    },
    {
      key: 'start_time',
      header: 'Starts',
      width: '140px',
      render: (tournament) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(tournament.start_time)}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Tournaments</h1>
          <p className="page-subtitle">
            Manage competitive tournaments
          </p>
        </div>
        <div className="flex gap-2">
          <button onClick={() => setShowCreateModal(true)} className="btn btn-primary">
            Create Tournament
          </button>
          <button onClick={loadTournaments} className="btn btn-secondary">
            Refresh
          </button>
        </div>
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
            <TrophyIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{tournaments.length}</span>
          <span className="stat-label">Total Tournaments</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ActiveIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{activeTournaments}</span>
          <span className="stat-label">Active</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <UpcomingIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{upcomingTournaments}</span>
          <span className="stat-label">Upcoming</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ParticipantsIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{totalParticipants}</span>
          <span className="stat-label">Total Participants</span>
        </div>
      </div>

      {/* Tournaments Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={tournaments}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedTournament?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search tournaments..."
          searchFields={['title', 'id', 'description']}
          pagination
          pageSize={15}
          emptyMessage="No tournaments found"
        />
      </div>

      {/* Create Tournament Modal */}
      {showCreateModal && (
        <div className="modal-overlay">
          <div className="modal" style={{ maxWidth: '500px' }}>
            <h2 className="modal-title">Create Tournament</h2>
            <div className="space-y-4">
              <div>
                <label className="form-label">ID</label>
                <input
                  type="text"
                  value={newTournament.id}
                  onChange={(e) => setNewTournament({ ...newTournament, id: e.target.value })}
                  className="form-input"
                  placeholder="e.g., weekly_tournament"
                />
              </div>
              <div>
                <label className="form-label">Title</label>
                <input
                  type="text"
                  value={newTournament.title}
                  onChange={(e) => setNewTournament({ ...newTournament, title: e.target.value })}
                  className="form-input"
                  placeholder="Weekly Tournament"
                />
              </div>
              <div>
                <label className="form-label">Description</label>
                <textarea
                  value={newTournament.description}
                  onChange={(e) => setNewTournament({ ...newTournament, description: e.target.value })}
                  className="form-input"
                  rows={3}
                  placeholder="Tournament description"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="form-label">Category</label>
                  <input
                    type="number"
                    value={newTournament.category}
                    onChange={(e) => setNewTournament({ ...newTournament, category: parseInt(e.target.value) || 0 })}
                    className="form-input"
                  />
                </div>
                <div>
                  <label className="form-label">Sort Order</label>
                  <select
                    value={newTournament.sort_order}
                    onChange={(e) => setNewTournament({ ...newTournament, sort_order: e.target.value })}
                    className="form-input"
                  >
                    <option value="descending">Descending</option>
                    <option value="ascending">Ascending</option>
                  </select>
                </div>
              </div>
              <div className="grid grid-cols-3 gap-4">
                <div>
                  <label className="form-label">Max Size</label>
                  <input
                    type="number"
                    value={newTournament.max_size}
                    onChange={(e) => setNewTournament({ ...newTournament, max_size: parseInt(e.target.value) || 100 })}
                    className="form-input"
                  />
                </div>
                <div>
                  <label className="form-label">Max Scores</label>
                  <input
                    type="number"
                    value={newTournament.max_num_score}
                    onChange={(e) => setNewTournament({ ...newTournament, max_num_score: parseInt(e.target.value) || 1000000 })}
                    className="form-input"
                  />
                </div>
                <div>
                  <label className="form-label">Duration (s)</label>
                  <input
                    type="number"
                    value={newTournament.duration}
                    onChange={(e) => setNewTournament({ ...newTournament, duration: parseInt(e.target.value) || 86400 })}
                    className="form-input"
                  />
                </div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowCreateModal(false)} className="btn btn-secondary">
                Cancel
              </button>
              <button onClick={createTournament} className="btn btn-primary">
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Tournament Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Tournament Details"
        width="lg"
        footer={
          selectedTournament && (
            <button onClick={deleteTournament} className="btn btn-danger flex-1">
              Delete Tournament
            </button>
          )
        }
      >
        {selectedTournament && (
          <div className="space-y-6">
            {/* Tournament Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: getTournamentStatus(selectedTournament).variant === 'success'
                    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                    : getTournamentStatus(selectedTournament).variant === 'warning'
                    ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)'
                    : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                  color: 'white',
                }}
              >
                <TrophyIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedTournament.title || selectedTournament.id}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={getTournamentStatus(selectedTournament).variant}>
                    {getTournamentStatus(selectedTournament).label}
                  </Badge>
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-3 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedTournament.size}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Participants</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedTournament.max_size}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Max Size</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {formatDuration(selectedTournament.duration)}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Duration</div>
              </div>
            </div>

            <Section title="Tournament Information">
              <Field label="Tournament ID" mono>
                {selectedTournament.id}
              </Field>
              <Field label="Title">
                {selectedTournament.title || '-'}
              </Field>
              <Field label="Description">
                {selectedTournament.description || '-'}
              </Field>
              <Field label="Category">
                {selectedTournament.category}
              </Field>
              <Field label="Sort Order">
                <span className="capitalize">{selectedTournament.sort_order}</span>
              </Field>
              <Field label="Max Scores">
                {selectedTournament.max_num_score.toLocaleString()}
              </Field>
              <Field label="Starts At">
                {formatTimestamp(selectedTournament.start_time)}
              </Field>
              <Field label="Ends At">
                {selectedTournament.end_time ? formatTimestamp(selectedTournament.end_time) : 'Never'}
              </Field>
              <Field label="Created At">
                {formatTimestamp(selectedTournament.created_at)}
              </Field>
            </Section>

            <Section title="Leaderboard">
              {records.length > 0 ? (
                <div className="space-y-2 max-h-80 overflow-y-auto">
                  {records.map((record) => (
                    <div
                      key={record.owner_id}
                      className="flex items-center justify-between p-3 rounded-lg"
                      style={{ background: 'var(--bg-tertiary)' }}
                    >
                      <div className="flex items-center gap-3">
                        <div
                          className="w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold"
                          style={{
                            background: record.rank === 1
                              ? 'linear-gradient(135deg, #fbbf24 0%, #f59e0b 100%)'
                              : record.rank === 2
                              ? 'linear-gradient(135deg, #9ca3af 0%, #6b7280 100%)'
                              : record.rank === 3
                              ? 'linear-gradient(135deg, #cd7f32 0%, #a0522d 100%)'
                              : 'var(--bg-secondary)',
                            color: record.rank <= 3 ? 'white' : 'var(--text-secondary)',
                          }}
                        >
                          #{record.rank}
                        </div>
                        <div>
                          <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
                            {record.username || record.owner_id.slice(0, 12)}
                          </div>
                          <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
                            {record.num_score} submissions
                          </div>
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="font-mono font-bold" style={{ color: 'var(--text-primary)' }}>
                          {record.score.toLocaleString()}
                        </div>
                        <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
                          {formatRelativeTime(record.updated_at)}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)' }}>No entries yet</p>
              )}
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}

// Icons
function TrophyIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z" />
    </svg>
  );
}

function ActiveIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function UpcomingIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function ParticipantsIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}
