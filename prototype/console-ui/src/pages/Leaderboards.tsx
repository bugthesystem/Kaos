import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';

interface Leaderboard {
  id: string;
  sort_order: string;
  operator: string;
  reset_schedule: string | null;
  record_count: number;
}

interface LeaderboardRecord {
  owner_id: string;
  username: string;
  score: number;
  rank: number;
  metadata: Record<string, unknown>;
  updated_at: number;
}

function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString();
}

export default function Leaderboards() {
  const [leaderboards, setLeaderboards] = useState<Leaderboard[]>([]);
  const [records, setRecords] = useState<LeaderboardRecord[]>([]);
  const [selectedLeaderboard, setSelectedLeaderboard] = useState<Leaderboard | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [newLeaderboard, setNewLeaderboard] = useState({ id: '', sort_order: 'descending' });

  useEffect(() => {
    loadLeaderboards();
  }, []);

  useEffect(() => {
    if (selectedLeaderboard) {
      loadRecords(selectedLeaderboard.id);
    }
  }, [selectedLeaderboard]);

  const loadLeaderboards = async () => {
    try {
      setLoading(true);
      const data = await api.get('/api/leaderboards');
      setLeaderboards(data.leaderboards || []);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load leaderboards');
    } finally {
      setLoading(false);
    }
  };

  const loadRecords = async (leaderboardId: string) => {
    try {
      const data = await api.get(`/api/leaderboards/${leaderboardId}/records?limit=100`);
      setRecords(data.records || []);
    } catch (err) {
      console.error('Failed to load records:', err);
      setRecords([]);
    }
  };

  const handleRowClick = (leaderboard: Leaderboard) => {
    setSelectedLeaderboard(leaderboard);
    setDrawerOpen(true);
  };

  const createLeaderboard = async () => {
    if (!newLeaderboard.id.trim()) {
      alert('Please enter a leaderboard ID');
      return;
    }
    try {
      await api.post('/api/leaderboards', newLeaderboard);
      setShowCreate(false);
      setNewLeaderboard({ id: '', sort_order: 'descending' });
      loadLeaderboards();
    } catch (err) {
      alert('Failed to create: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const deleteLeaderboard = async () => {
    if (!selectedLeaderboard) return;
    if (!confirm('Delete this leaderboard and all records?')) return;
    try {
      await api.delete(`/api/leaderboards/${selectedLeaderboard.id}`);
      setDrawerOpen(false);
      setSelectedLeaderboard(null);
      loadLeaderboards();
    } catch (err) {
      alert('Failed to delete: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const totalRecords = leaderboards.reduce((sum, lb) => sum + lb.record_count, 0);
  const descendingCount = leaderboards.filter(lb => lb.sort_order === 'descending').length;
  const withScheduleCount = leaderboards.filter(lb => lb.reset_schedule).length;

  const columns: Column<Leaderboard>[] = [
    {
      key: 'id',
      header: 'Leaderboard',
      render: (lb) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: lb.sort_order === 'descending'
                ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)'
                : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
              color: 'white',
            }}
          >
            <TrophyIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {lb.id}
            </div>
            <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
              {lb.record_count} records
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'sort_order',
      header: 'Sort',
      width: '120px',
      render: (lb) => (
        <Badge variant={lb.sort_order === 'descending' ? 'warning' : 'info'}>
          {lb.sort_order === 'descending' ? 'Highest First' : 'Lowest First'}
        </Badge>
      ),
    },
    {
      key: 'operator',
      header: 'Operator',
      width: '100px',
      render: (lb) => (
        <span className="font-mono text-sm" style={{ color: 'var(--text-secondary)' }}>
          {lb.operator || 'best'}
        </span>
      ),
    },
    {
      key: 'record_count',
      header: 'Records',
      width: '100px',
      render: (lb) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {lb.record_count.toLocaleString()}
        </span>
      ),
    },
    {
      key: 'reset_schedule',
      header: 'Reset',
      width: '120px',
      render: (lb) => (
        lb.reset_schedule ? (
          <Badge variant="info">{lb.reset_schedule}</Badge>
        ) : (
          <span style={{ color: 'var(--text-muted)' }}>Never</span>
        )
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Leaderboards</h1>
          <p className="page-subtitle">
            Competitive rankings and scores
          </p>
        </div>
        <div className="flex gap-2">
          <button onClick={loadLeaderboards} className="btn btn-secondary">
            Refresh
          </button>
          <button onClick={() => setShowCreate(true)} className="btn btn-primary">
            Create Leaderboard
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
          <span className="stat-value">{leaderboards.length}</span>
          <span className="stat-label">Total Leaderboards</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <RecordsIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{totalRecords.toLocaleString()}</span>
          <span className="stat-label">Total Records</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <DescendingIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{descendingCount}</span>
          <span className="stat-label">Highest First</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ScheduleIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{withScheduleCount}</span>
          <span className="stat-label">With Reset</span>
        </div>
      </div>

      {/* Leaderboards Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={leaderboards}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedLeaderboard?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search leaderboards..."
          searchFields={['id']}
          pagination
          pageSize={15}
          emptyMessage="No leaderboards found"
        />
      </div>

      {/* Create Modal */}
      {showCreate && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center"
          style={{ background: 'rgba(0, 0, 0, 0.5)' }}
          onClick={() => setShowCreate(false)}
        >
          <div
            className="modal"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="modal-title">Create Leaderboard</h2>
            <div className="space-y-4">
              <div>
                <label className="form-label">Leaderboard ID</label>
                <input
                  type="text"
                  value={newLeaderboard.id}
                  onChange={(e) => setNewLeaderboard({ ...newLeaderboard, id: e.target.value })}
                  className="form-input"
                  placeholder="e.g., weekly_scores"
                />
              </div>
              <div>
                <label className="form-label">Sort Order</label>
                <select
                  value={newLeaderboard.sort_order}
                  onChange={(e) => setNewLeaderboard({ ...newLeaderboard, sort_order: e.target.value })}
                  className="form-input"
                >
                  <option value="descending">Descending (highest first)</option>
                  <option value="ascending">Ascending (lowest first)</option>
                </select>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowCreate(false)} className="btn btn-secondary">
                Cancel
              </button>
              <button onClick={createLeaderboard} className="btn btn-primary">
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Leaderboard Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Leaderboard Details"
        width="lg"
        footer={
          selectedLeaderboard && (
            <button onClick={deleteLeaderboard} className="btn btn-danger flex-1">
              Delete Leaderboard
            </button>
          )
        }
      >
        {selectedLeaderboard && (
          <div className="space-y-6">
            {/* Leaderboard Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedLeaderboard.sort_order === 'descending'
                    ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)'
                    : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                  color: 'white',
                }}
              >
                <TrophyIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedLeaderboard.id}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={selectedLeaderboard.sort_order === 'descending' ? 'warning' : 'info'}>
                    {selectedLeaderboard.sort_order === 'descending' ? 'Highest First' : 'Lowest First'}
                  </Badge>
                  {selectedLeaderboard.reset_schedule && (
                    <Badge variant="info">{selectedLeaderboard.reset_schedule}</Badge>
                  )}
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-3 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedLeaderboard.record_count.toLocaleString()}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Records</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedLeaderboard.operator || 'best'}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Operator</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {records.length > 0 ? records[0].score.toLocaleString() : '-'}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Top Score</div>
              </div>
            </div>

            <Section title="Configuration">
              <Field label="Leaderboard ID" mono>
                {selectedLeaderboard.id}
              </Field>
              <Field label="Sort Order">
                {selectedLeaderboard.sort_order}
              </Field>
              <Field label="Operator">
                {selectedLeaderboard.operator || 'best'}
              </Field>
              <Field label="Reset Schedule">
                {selectedLeaderboard.reset_schedule || 'Never'}
              </Field>
            </Section>

            <Section title="Top Rankings">
              {records.length > 0 ? (
                <div className="space-y-2">
                  {records.slice(0, 10).map((record) => (
                    <div
                      key={record.owner_id}
                      className="flex items-center justify-between p-3 rounded-lg"
                      style={{ background: 'var(--bg-tertiary)' }}
                    >
                      <div className="flex items-center gap-3">
                        <span
                          className="w-8 h-8 rounded-full flex items-center justify-center font-bold text-sm"
                          style={{
                            background: record.rank <= 3
                              ? record.rank === 1 ? '#fbbf24' : record.rank === 2 ? '#94a3b8' : '#cd7f32'
                              : 'var(--bg-secondary)',
                            color: record.rank <= 3 ? '#000' : 'var(--text-primary)',
                          }}
                        >
                          {record.rank}
                        </span>
                        <div>
                          <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
                            {record.username || 'Unknown'}
                          </div>
                          <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
                            {record.owner_id.slice(0, 12)}...
                          </div>
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="font-mono font-bold" style={{ color: 'var(--color-accent)' }}>
                          {record.score.toLocaleString()}
                        </div>
                        <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
                          {formatTimestamp(record.updated_at)}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)' }}>No records in this leaderboard</p>
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

function RecordsIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01" />
    </svg>
  );
}

function DescendingIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12" />
    </svg>
  );
}

function ScheduleIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}
