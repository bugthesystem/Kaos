import { useState, useEffect, useCallback } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { ClockIcon, ShieldIcon, CheckIcon, XIcon, RefreshIcon, UsersIcon, KeyIcon } from '../components/icons';
import { formatTimestamp, formatRelativeTime } from '../utils/formatters';
import type { AuditLogInfo, PaginatedList } from '../api/types';

// Action descriptions for display
const ACTION_LABELS: Record<string, { label: string; variant: 'success' | 'danger' | 'warning' | 'info' }> = {
  login: { label: 'Login', variant: 'info' },
  logout: { label: 'Logout', variant: 'info' },
  login_failed: { label: 'Login Failed', variant: 'danger' },
  create_account: { label: 'Create Account', variant: 'success' },
  update_account: { label: 'Update Account', variant: 'warning' },
  delete_account: { label: 'Delete Account', variant: 'danger' },
  disable_account: { label: 'Disable Account', variant: 'warning' },
  enable_account: { label: 'Enable Account', variant: 'success' },
  change_password: { label: 'Change Password', variant: 'warning' },
  create_api_key: { label: 'Create API Key', variant: 'success' },
  delete_api_key: { label: 'Delete API Key', variant: 'danger' },
  kick_session: { label: 'Kick Session', variant: 'danger' },
  terminate_room: { label: 'Terminate Room', variant: 'danger' },
  reload_scripts: { label: 'Reload Scripts', variant: 'warning' },
  execute_rpc: { label: 'Execute RPC', variant: 'info' },
  delete_storage: { label: 'Delete Storage', variant: 'danger' },
  create_storage: { label: 'Create Storage', variant: 'success' },
  delete_leaderboard: { label: 'Delete Leaderboard', variant: 'danger' },
};


export default function AuditLogs() {
  const [logs, setLogs] = useState<AuditLogInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selectedLog, setSelectedLog] = useState<AuditLogInfo | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [page] = useState(1);
  const [total, setTotal] = useState(0);
  const [actionFilter, setActionFilter] = useState('');
  const [actorFilter, setActorFilter] = useState('');

  const loadLogs = useCallback(async () => {
    try {
      setLoading(true);
      let url = `/api/audit-logs?page=${page}&page_size=50`;
      if (actionFilter) url += `&action=${actionFilter}`;
      if (actorFilter) url += `&actor=${actorFilter}`;
      const data: PaginatedList<AuditLogInfo> = await api.get(url);
      setLogs(data.items);
      setTotal(data.total);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load audit logs');
      setLogs([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [page, actionFilter, actorFilter]);

  useEffect(() => {
    loadLogs();
  }, [loadLogs]);

  const handleRowClick = (log: AuditLogInfo) => {
    setSelectedLog(log);
    setDrawerOpen(true);
  };

  const successCount = logs.filter(l => l.success).length;
  const failedCount = logs.filter(l => !l.success).length;
  const userActions = logs.filter(l => l.actor_type === 'user').length;
  const apiKeyActions = logs.filter(l => l.actor_type === 'api_key').length;

  const columns: Column<AuditLogInfo>[] = [
    {
      key: 'timestamp',
      header: 'Time',
      width: '140px',
      render: (log) => (
        <span className="text-sm" style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(log.timestamp * 1000)}
        </span>
      ),
    },
    {
      key: 'actor_name',
      header: 'Actor',
      render: (log) => (
        <div className="flex items-center gap-2">
          <div
            className="w-7 h-7 rounded-lg flex items-center justify-center text-xs"
            style={{
              background: log.actor_type === 'user'
                ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
                : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
              color: 'white',
            }}
          >
            {log.actor_type === 'user' ? <UsersIcon className="w-4 h-4" /> : <KeyIcon className="w-4 h-4" />}
          </div>
          <div>
            <span className="font-medium" style={{ color: 'var(--text-primary)' }}>{log.actor_name}</span>
            <Badge variant={log.actor_type === 'user' ? 'info' : 'warning'} className="ml-2">
              {log.actor_type === 'user' ? 'User' : 'API Key'}
            </Badge>
          </div>
        </div>
      ),
    },
    {
      key: 'action',
      header: 'Action',
      render: (log) => {
        const actionInfo = ACTION_LABELS[log.action] || { label: log.action, variant: 'info' as const };
        return (
          <div className="flex items-center gap-2">
            <Badge variant={actionInfo.variant}>{actionInfo.label}</Badge>
            {!log.success && (
              <span className="text-xs px-2 py-0.5 rounded-full" style={{ background: 'rgba(239, 68, 68, 0.15)', color: '#ef4444' }}>
                Failed
              </span>
            )}
          </div>
        );
      },
    },
    {
      key: 'resource_type',
      header: 'Resource',
      render: (log) => (
        <div>
          <span className="capitalize" style={{ color: 'var(--text-secondary)' }}>{log.resource_type}</span>
          {log.resource_id && (
            <span className="text-xs font-mono ml-2" style={{ color: 'var(--text-muted)' }}>
              {log.resource_id.length > 12 ? `${log.resource_id.slice(0, 12)}...` : log.resource_id}
            </span>
          )}
        </div>
      ),
    },
    {
      key: 'ip_address',
      header: 'IP',
      width: '120px',
      render: (log) => (
        <span className="font-mono text-sm" style={{ color: 'var(--text-muted)' }}>
          {log.ip_address || '-'}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader title="Audit Logs" subtitle="Track all administrative actions">
        <button onClick={loadLogs} className="btn btn-secondary">
          <RefreshIcon className="w-4 h-4" />
        </button>
      </PageHeader>

      {error && <Alert variant="warning" onDismiss={() => setError('')}>Using demo data - {error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<ClockIcon className="w-5 h-5" />} label="Total Events" value={total} color="primary" />
        <StatCard icon={<CheckIcon className="w-5 h-5" />} label="Successful" value={successCount} color="success" />
        <StatCard icon={<XIcon className="w-5 h-5" />} label="Failed" value={failedCount} color="danger" />
        <StatCard icon={<ShieldIcon className="w-5 h-5" />} label="User / API Key" value={`${userActions} / ${apiKeyActions}`} color="info" />
      </StatGrid>

      <div className="card">
        <div className="flex gap-4 mb-4">
          <select
            value={actionFilter}
            onChange={(e) => setActionFilter(e.target.value)}
            className="form-input"
            style={{ width: '200px' }}
          >
            <option value="">All Actions</option>
            <option value="login">Login</option>
            <option value="logout">Logout</option>
            <option value="create_account">Create Account</option>
            <option value="delete_account">Delete Account</option>
            <option value="kick_session">Kick Session</option>
            <option value="terminate_room">Terminate Room</option>
            <option value="execute_rpc">Execute RPC</option>
          </select>
          <input
            type="text"
            value={actorFilter}
            onChange={(e) => setActorFilter(e.target.value)}
            placeholder="Filter by actor name..."
            className="form-input flex-1"
          />
        </div>
      </div>

      <div className="card p-0 overflow-hidden">
        <DataTable
          data={logs}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedLog?.id}
          loading={loading}
          pagination
          pageSize={20}
          emptyMessage="No audit logs found"
        />
      </div>

      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Audit Log Details"
        width="md"
      >
        {selectedLog && (
          <div className="space-y-6">
            <div className="flex items-center gap-4">
              <div
                className="w-14 h-14 rounded-xl flex items-center justify-center"
                style={{
                  background: selectedLog.success
                    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                    : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
                  color: 'white',
                }}
              >
                {selectedLog.success ? <CheckIcon className="w-7 h-7" /> : <XIcon className="w-7 h-7" />}
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {ACTION_LABELS[selectedLog.action]?.label || selectedLog.action}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={selectedLog.success ? 'success' : 'danger'}>
                    {selectedLog.success ? 'Success' : 'Failed'}
                  </Badge>
                  <Badge variant={selectedLog.actor_type === 'user' ? 'info' : 'warning'}>
                    {selectedLog.actor_type === 'user' ? 'User' : 'API Key'}
                  </Badge>
                </div>
              </div>
            </div>

            <Section title="Event Details">
              <Field label="Timestamp">{formatTimestamp(selectedLog.timestamp * 1000)}</Field>
              <Field label="Action">{selectedLog.action}</Field>
              <Field label="Resource Type">{selectedLog.resource_type}</Field>
              <Field label="Resource ID" mono>{selectedLog.resource_id || '-'}</Field>
            </Section>

            <Section title="Actor Information">
              <Field label="Actor Name">{selectedLog.actor_name}</Field>
              <Field label="Actor ID" mono>{selectedLog.actor_id}</Field>
              <Field label="Actor Type">{selectedLog.actor_type === 'user' ? 'Console User' : 'API Key'}</Field>
              <Field label="IP Address" mono>{selectedLog.ip_address || '-'}</Field>
            </Section>

            {selectedLog.details && (
              <Section title="Additional Details">
                <pre
                  className="p-4 rounded-lg overflow-x-auto text-sm font-mono"
                  style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}
                >
                  {JSON.stringify(selectedLog.details, null, 2)}
                </pre>
              </Section>
            )}
          </div>
        )}
      </Drawer>
    </div>
  );
}
