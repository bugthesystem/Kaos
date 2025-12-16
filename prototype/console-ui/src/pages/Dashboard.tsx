import { useEffect, useState } from 'react';
import { api } from '../api/client';
import { useToast } from '../components/Toast';
import type { ServerStatus } from '../api/types';

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (days > 0) return `${days}d ${hours}h ${mins}m`;
  if (hours > 0) return `${hours}h ${mins}m`;
  if (mins > 0) return `${mins}m ${secs}s`;
  return `${secs}s`;
}

interface DashboardProps {
  onNavigate?: (page: string) => void;
}

export function DashboardPage({ onNavigate }: DashboardProps) {
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const toast = useToast();

  useEffect(() => {
    loadStatus();
    const interval = setInterval(loadStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  const loadStatus = async (showToast = false) => {
    try {
      const data = await api.getStatus();
      setStatus(data);
      setError('');
      if (showToast) {
        toast.success('Stats refreshed', `${data.sessions.total} sessions, ${data.rooms.total} rooms`);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load status';
      setError(message);
      if (showToast) {
        toast.error('Refresh failed', message);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleRefresh = () => loadStatus(true);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="spinner" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="alert alert-error">
        <ErrorIcon className="w-5 h-5 flex-shrink-0" />
        <span>{error}</span>
      </div>
    );
  }

  return (
    <div className="space-y-8 animate-fade-in">
      {/* Page Header */}
      <div className="page-header">
        <h1 className="page-title">Dashboard</h1>
        <p className="page-subtitle">Server overview and real-time statistics</p>
      </div>

      {/* Server Status Card */}
      <div className="card">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="w-14 h-14 rounded-2xl flex items-center justify-center"
              style={{
                background: 'linear-gradient(135deg, rgba(16, 185, 129, 0.2) 0%, rgba(16, 185, 129, 0.05) 100%)',
                border: '1px solid rgba(16, 185, 129, 0.25)',
              }}
            >
              <ServerIcon className="w-7 h-7 text-emerald-400" />
            </div>
            <div>
              <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>Server Status</h2>
              <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>KaosNet v{status?.version}</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <div className="status-dot online" />
            <span className="badge badge-success">Online</span>
          </div>
        </div>
        <div className="mt-6 flex items-center gap-8">
          <div className="flex items-center gap-2" style={{ color: 'var(--text-secondary)' }}>
            <ClockIcon className="w-4 h-4" />
            <span className="text-sm">Uptime: <span className="font-medium" style={{ color: 'var(--text-primary)' }}>{status ? formatUptime(status.uptime_secs) : '-'}</span></span>
          </div>
          <button onClick={handleRefresh} className="btn btn-ghost btn-sm flex items-center gap-2 btn-ripple">
            <RefreshIcon className="w-4 h-4" />
            Refresh
          </button>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-5">
        <StatCard
          icon={<UsersIcon className="w-6 h-6 text-indigo-400" />}
          value={status?.sessions.total || 0}
          label="Total Sessions"
          color="indigo"
        />
        <StatCard
          icon={<ConnectionIcon className="w-6 h-6 text-emerald-400" />}
          value={status?.sessions.connected || 0}
          label="Connected"
          color="emerald"
        />
        <StatCard
          icon={<RoomsIcon className="w-6 h-6 text-violet-400" />}
          value={status?.rooms.total || 0}
          label="Active Rooms"
          color="violet"
        />
        <StatCard
          icon={<PlayersIcon className="w-6 h-6 text-amber-400" />}
          value={status?.rooms.players || 0}
          label="Players in Rooms"
          color="amber"
        />
      </div>

      {/* Two Column Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Session States */}
        <div className="card">
          <div className="flex items-center gap-3 mb-6">
            <div className="w-10 h-10 rounded-xl flex items-center justify-center"
              style={{
                background: 'linear-gradient(135deg, rgba(6, 182, 212, 0.2) 0%, rgba(139, 92, 246, 0.1) 100%)',
                border: '1px solid rgba(6, 182, 212, 0.2)',
              }}
            >
              <ChartIcon className="w-5 h-5 text-cyan-400" />
            </div>
            <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>Session States</h2>
          </div>
          <div className="space-y-4">
            <SessionStateRow
              label="Connecting"
              value={status?.sessions.connecting || 0}
              total={status?.sessions.total || 0}
              color="amber"
            />
            <SessionStateRow
              label="Connected"
              value={status?.sessions.connected || 0}
              total={status?.sessions.total || 0}
              color="sky"
            />
            <SessionStateRow
              label="Authenticated"
              value={status?.sessions.authenticated || 0}
              total={status?.sessions.total || 0}
              color="emerald"
            />
          </div>
        </div>

        {/* Quick Actions */}
        <div className="card">
          <div className="flex items-center gap-3 mb-6">
            <div className="w-10 h-10 rounded-xl flex items-center justify-center"
              style={{
                background: 'linear-gradient(135deg, rgba(139, 92, 246, 0.2) 0%, rgba(236, 72, 153, 0.1) 100%)',
                border: '1px solid rgba(139, 92, 246, 0.2)',
              }}
            >
              <BoltIcon className="w-5 h-5 text-violet-400" />
            </div>
            <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>Quick Actions</h2>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <QuickActionButton icon={<RefreshIcon className="w-5 h-5" />} label="Refresh Stats" onClick={handleRefresh} />
            <QuickActionButton icon={<UsersIcon className="w-5 h-5" />} label="View Sessions" onClick={() => onNavigate?.('sessions')} />
            <QuickActionButton icon={<RoomsIcon className="w-5 h-5" />} label="View Rooms" onClick={() => onNavigate?.('rooms')} />
            <QuickActionButton icon={<TerminalIcon className="w-5 h-5" />} label="Lua Console" onClick={() => onNavigate?.('lua')} />
          </div>
        </div>
      </div>
    </div>
  );
}

// Components
interface StatCardProps {
  icon: React.ReactNode;
  value: number;
  label: string;
  color: 'indigo' | 'emerald' | 'violet' | 'amber' | 'rose';
}

function StatCard({ icon, value, label, color }: StatCardProps) {
  const colors = {
    indigo: { bg: 'rgba(6, 182, 212, 0.15)', border: 'rgba(6, 182, 212, 0.25)' },
    emerald: { bg: 'rgba(16, 185, 129, 0.15)', border: 'rgba(16, 185, 129, 0.25)' },
    violet: { bg: 'rgba(139, 92, 246, 0.15)', border: 'rgba(139, 92, 246, 0.25)' },
    amber: { bg: 'rgba(245, 158, 11, 0.15)', border: 'rgba(245, 158, 11, 0.25)' },
    rose: { bg: 'rgba(236, 72, 153, 0.15)', border: 'rgba(236, 72, 153, 0.25)' },
  };

  return (
    <div className="stat-card">
      <div
        className="stat-icon"
        style={{
          background: colors[color].bg,
          border: `1px solid ${colors[color].border}`,
        }}
      >
        {icon}
      </div>
      <span className="stat-value">{value.toLocaleString()}</span>
      <span className="stat-label">{label}</span>
    </div>
  );
}

interface SessionStateRowProps {
  label: string;
  value: number;
  total: number;
  color: 'amber' | 'sky' | 'emerald';
}

function SessionStateRow({ label, value, total, color }: SessionStateRowProps) {
  const percentage = total > 0 ? (value / total) * 100 : 0;
  const colors = {
    amber: { bg: 'rgba(245, 158, 11, 0.2)', bar: '#f59e0b' },
    sky: { bg: 'rgba(14, 165, 233, 0.2)', bar: '#0ea5e9' },
    emerald: { bg: 'rgba(34, 197, 94, 0.2)', bar: '#22c55e' },
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>{label}</span>
        <span className="font-semibold" style={{ color: 'var(--text-primary)' }}>{value}</span>
      </div>
      <div className="h-2 rounded-full overflow-hidden" style={{ background: 'var(--bg-tertiary)' }}>
        <div
          className="h-full rounded-full transition-all duration-500"
          style={{
            width: `${percentage}%`,
            background: colors[color].bar,
          }}
        />
      </div>
    </div>
  );
}

interface QuickActionButtonProps {
  icon: React.ReactNode;
  label: string;
  onClick?: () => void;
  disabled?: boolean;
}

function QuickActionButton({ icon, label, onClick, disabled }: QuickActionButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="p-4 rounded-xl text-left transition-all duration-200 flex items-center gap-3 group disabled:opacity-50 disabled:cursor-not-allowed"
      style={{
        background: 'var(--bg-elevated)',
        border: '1px solid var(--border-primary)',
      }}
      onMouseOver={(e) => {
        if (!disabled) {
          e.currentTarget.style.borderColor = 'var(--border-hover)';
          e.currentTarget.style.background = 'var(--bg-hover)';
        }
      }}
      onMouseOut={(e) => {
        e.currentTarget.style.borderColor = 'var(--border-primary)';
        e.currentTarget.style.background = 'var(--bg-elevated)';
      }}
    >
      <span style={{ color: 'var(--text-muted)' }} className="group-hover:text-cyan-500 transition-colors">{icon}</span>
      <span className="text-sm transition-colors" style={{ color: 'var(--text-secondary)' }}>{label}</span>
    </button>
  );
}

// Icons
function ServerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
    </svg>
  );
}

function ClockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function RefreshIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
    </svg>
  );
}

function UsersIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}

function ConnectionIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  );
}

function RoomsIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
  );
}

function PlayersIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}

function ChartIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
    </svg>
  );
}

function BoltIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  );
}

function TerminalIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
    </svg>
  );
}

function ErrorIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}
