import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { useConfirm } from '../components/ConfirmDialog';
import { useAuth } from '../contexts/AuthContext';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { TrophyIcon, DatabaseIcon, TrendDownIcon, ClockIcon, RefreshIcon } from '../components/icons';
import { formatTimestamp } from '../utils/formatters';

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

export default function Leaderboards() {
  const { hasPermission } = useAuth();
  const canDelete = hasPermission('delete:leaderboard');

  const [leaderboards, setLeaderboards] = useState<Leaderboard[]>([]);
  const [records, setRecords] = useState<LeaderboardRecord[]>([]);
  const [selectedLeaderboard, setSelectedLeaderboard] = useState<Leaderboard | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [newLeaderboard, setNewLeaderboard] = useState({ id: '', sort_order: 'descending' });
  const { confirm, ConfirmDialog } = useConfirm();

  useEffect(() => { loadLeaderboards(); }, []);
  useEffect(() => { if (selectedLeaderboard) loadRecords(selectedLeaderboard.id); }, [selectedLeaderboard]);

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
    if (!newLeaderboard.id.trim()) { alert('Please enter a leaderboard ID'); return; }
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
    const confirmed = await confirm({
      title: 'Delete Leaderboard',
      message: 'Delete this leaderboard and all records? This cannot be undone.',
      confirmLabel: 'Delete',
      variant: 'danger',
    });
    if (!confirmed) return;
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
          <LeaderboardAvatar lb={lb} size="sm" />
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>{lb.id}</div>
            <div className="text-xs" style={{ color: 'var(--text-muted)' }}>{lb.record_count} records</div>
          </div>
        </div>
      ),
    },
    {
      key: 'sort_order',
      header: 'Sort',
      width: '120px',
      render: (lb) => <Badge variant={lb.sort_order === 'descending' ? 'warning' : 'info'}>{lb.sort_order === 'descending' ? 'Highest First' : 'Lowest First'}</Badge>,
    },
    {
      key: 'operator',
      header: 'Operator',
      width: '100px',
      render: (lb) => <span className="font-mono text-sm" style={{ color: 'var(--text-secondary)' }}>{lb.operator || 'best'}</span>,
    },
    {
      key: 'record_count',
      header: 'Records',
      width: '100px',
      render: (lb) => <span style={{ color: 'var(--text-secondary)' }}>{lb.record_count.toLocaleString()}</span>,
    },
    {
      key: 'reset_schedule',
      header: 'Reset',
      width: '120px',
      render: (lb) => lb.reset_schedule ? <Badge variant="info">{lb.reset_schedule}</Badge> : <span style={{ color: 'var(--text-muted)' }}>Never</span>,
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {ConfirmDialog}
      <PageHeader title="Leaderboards" subtitle="Competitive rankings and scores">
        <button onClick={loadLeaderboards} className="btn btn-secondary"><RefreshIcon className="w-4 h-4" /></button>
        {canDelete && <button onClick={() => setShowCreate(true)} className="btn btn-primary">Create Leaderboard</button>}
      </PageHeader>

      {error && <Alert variant="danger" onDismiss={() => setError('')}>{error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<TrophyIcon className="w-5 h-5" />} label="Total Leaderboards" value={leaderboards.length} color="primary" />
        <StatCard icon={<DatabaseIcon className="w-5 h-5" />} label="Total Records" value={totalRecords.toLocaleString()} color="success" />
        <StatCard icon={<TrendDownIcon className="w-5 h-5" />} label="Highest First" value={descendingCount} color="warning" />
        <StatCard icon={<ClockIcon className="w-5 h-5" />} label="With Reset" value={withScheduleCount} color="info" />
      </StatGrid>

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

      {showCreate && (
        <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: 'rgba(0, 0, 0, 0.5)' }} onClick={() => setShowCreate(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2 className="modal-title">Create Leaderboard</h2>
            <div className="space-y-4">
              <div>
                <label className="form-label">Leaderboard ID</label>
                <input type="text" value={newLeaderboard.id} onChange={(e) => setNewLeaderboard({ ...newLeaderboard, id: e.target.value })} className="form-input" placeholder="e.g., weekly_scores" />
              </div>
              <div>
                <label className="form-label">Sort Order</label>
                <select value={newLeaderboard.sort_order} onChange={(e) => setNewLeaderboard({ ...newLeaderboard, sort_order: e.target.value })} className="form-input">
                  <option value="descending">Descending (highest first)</option>
                  <option value="ascending">Ascending (lowest first)</option>
                </select>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowCreate(false)} className="btn btn-secondary">Cancel</button>
              <button onClick={createLeaderboard} className="btn btn-primary">Create</button>
            </div>
          </div>
        </div>
      )}

      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Leaderboard Details"
        width="lg"
        footer={selectedLeaderboard && canDelete && <button onClick={deleteLeaderboard} className="btn btn-danger flex-1">Delete Leaderboard</button>}
      >
        {selectedLeaderboard && <LeaderboardDetails leaderboard={selectedLeaderboard} records={records} />}
      </Drawer>
    </div>
  );
}

function LeaderboardAvatar({ lb, size = 'sm' }: { lb: Leaderboard; size?: 'sm' | 'lg' }) {
  const sizeClasses = size === 'lg' ? 'w-16 h-16' : 'w-9 h-9';
  const iconSize = size === 'lg' ? 'w-8 h-8' : 'w-5 h-5';
  const bg = lb.sort_order === 'descending' ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)' : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)';
  return (
    <div className={`${sizeClasses} rounded-${size === 'lg' ? 'xl' : 'lg'} flex items-center justify-center`} style={{ background: bg, color: 'white' }}>
      <TrophyIcon className={iconSize} />
    </div>
  );
}

function LeaderboardDetails({ leaderboard, records }: { leaderboard: Leaderboard; records: LeaderboardRecord[] }) {
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <LeaderboardAvatar lb={leaderboard} size="lg" />
        <div>
          <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>{leaderboard.id}</h2>
          <div className="flex items-center gap-2 mt-1">
            <Badge variant={leaderboard.sort_order === 'descending' ? 'warning' : 'info'}>{leaderboard.sort_order === 'descending' ? 'Highest First' : 'Lowest First'}</Badge>
            {leaderboard.reset_schedule && <Badge variant="info">{leaderboard.reset_schedule}</Badge>}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-3">
        <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
          <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{leaderboard.record_count.toLocaleString()}</div>
          <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Records</div>
        </div>
        <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
          <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{leaderboard.operator || 'best'}</div>
          <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Operator</div>
        </div>
        <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
          <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{records.length > 0 ? records[0].score.toLocaleString() : '-'}</div>
          <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Top Score</div>
        </div>
      </div>

      <Section title="Configuration">
        <Field label="Leaderboard ID" mono>{leaderboard.id}</Field>
        <Field label="Sort Order">{leaderboard.sort_order}</Field>
        <Field label="Operator">{leaderboard.operator || 'best'}</Field>
        <Field label="Reset Schedule">{leaderboard.reset_schedule || 'Never'}</Field>
      </Section>

      <Section title="Top Rankings">
        {records.length > 0 ? (
          <div className="space-y-2">
            {records.slice(0, 10).map((record) => (
              <div key={record.owner_id} className="flex items-center justify-between p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="flex items-center gap-3">
                  <span className="w-8 h-8 rounded-full flex items-center justify-center font-bold text-sm" style={{
                    background: record.rank <= 3 ? (record.rank === 1 ? '#fbbf24' : record.rank === 2 ? '#94a3b8' : '#cd7f32') : 'var(--bg-secondary)',
                    color: record.rank <= 3 ? '#000' : 'var(--text-primary)',
                  }}>{record.rank}</span>
                  <div>
                    <div className="font-medium" style={{ color: 'var(--text-primary)' }}>{record.username || 'Unknown'}</div>
                    <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>{record.owner_id.slice(0, 12)}...</div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="font-mono font-bold" style={{ color: 'var(--color-accent)' }}>{record.score.toLocaleString()}</div>
                  <div className="text-xs" style={{ color: 'var(--text-muted)' }}>{formatTimestamp(record.updated_at)}</div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p style={{ color: 'var(--text-muted)' }}>No records in this leaderboard</p>
        )}
      </Section>
    </div>
  );
}
