import { useEffect, useState, useCallback } from 'react';
import { api } from '../api/client';
import { useToast } from '../components/Toast';
import { formatUptime, formatTimeAgo } from '../utils/formatters';
import { Spinner, Alert, Badge } from '../components/ui';
import {
  RefreshIcon,
  ClockIcon,
  UsersIcon,
  RoomsIcon,
  ServerIcon,
  ChartIcon,
  BoltIcon,
  TerminalIcon,
  ActivityIcon,
  GamepadIcon,
  ConnectionIcon,
  GroupIcon,
  UserPlusIcon,
  UserMinusIcon,
  PlusIcon,
  XIcon,
  CircleIcon,
  TrendUpIcon,
  TrendDownIcon,
  ShieldIcon,
} from '../components/icons';
import type { ServerStatus, SessionInfo, RoomInfo } from '../api/types';

interface DashboardProps {
  onNavigate?: (page: string) => void;
}

interface ActivityItem {
  id: string;
  type: 'session_connect' | 'session_disconnect' | 'room_create' | 'room_close' | 'player_join' | 'player_leave';
  message: string;
  timestamp: Date;
  meta?: Record<string, string>;
}

interface StatusHistory {
  timestamp: Date;
  sessions: number;
  rooms: number;
  players: number;
}

export function DashboardPage({ onNavigate }: DashboardProps) {
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [recentSessions, setRecentSessions] = useState<SessionInfo[]>([]);
  const [recentRooms, setRecentRooms] = useState<RoomInfo[]>([]);
  const [activityFeed, setActivityFeed] = useState<ActivityItem[]>([]);
  const [statusHistory, setStatusHistory] = useState<StatusHistory[]>([]);
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date());
  const toast = useToast();

  const loadStatus = useCallback(async (showToast = false) => {
    try {
      const [statusData, sessionsData, roomsData] = await Promise.all([
        api.getStatus(),
        api.listSessions(1, 5).catch(() => ({ items: [] })),
        api.listRooms(1, 5).catch(() => ({ items: [] })),
      ]);

      setStatus(statusData);
      setRecentSessions(sessionsData.items || []);
      setRecentRooms(roomsData.items || []);
      setError('');
      setLastUpdated(new Date());

      setStatusHistory(prev => {
        const newEntry = {
          timestamp: new Date(),
          sessions: statusData.sessions.total,
          rooms: statusData.rooms.total,
          players: statusData.rooms.players,
        };
        return [...prev.slice(-19), newEntry];
      });

      if (status && statusData.sessions.total !== status.sessions.total) {
        const diff = statusData.sessions.total - status.sessions.total;
        if (diff > 0) {
          addActivity('session_connect', `${diff} new session${diff > 1 ? 's' : ''} connected`);
        } else {
          addActivity('session_disconnect', `${Math.abs(diff)} session${Math.abs(diff) > 1 ? 's' : ''} disconnected`);
        }
      }

      if (showToast) {
        toast.success('Stats refreshed', `${statusData.sessions.total} sessions, ${statusData.rooms.total} rooms`);
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
  }, [status, toast]);

  const addActivity = (type: ActivityItem['type'], message: string, meta?: Record<string, string>) => {
    setActivityFeed(prev => [{
      id: crypto.randomUUID(),
      type,
      message,
      timestamp: new Date(),
      meta,
    }, ...prev.slice(0, 19)]);
  };

  useEffect(() => {
    loadStatus();
    const interval = setInterval(() => loadStatus(), 5000);
    return () => clearInterval(interval);
  }, []);

  const handleRefresh = () => loadStatus(true);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Spinner label="Loading dashboard..." />
      </div>
    );
  }

  if (error && !status) {
    return (
      <Alert variant="danger" title="Connection Error">
        <p>{error}</p>
        <button onClick={handleRefresh} className="btn btn-secondary btn-sm mt-2">
          Retry
        </button>
      </Alert>
    );
  }

  const healthScore = calculateHealthScore(status);

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header with Live Indicator */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title flex items-center gap-3">
            Dashboard
            <Badge variant="success" size="sm">
              <span className="flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse" />
                Live
              </span>
            </Badge>
          </h1>
          <p className="page-subtitle">Real-time server overview and monitoring</p>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
            Updated {formatTimeAgo(Math.floor(lastUpdated.getTime() / 1000))}
          </span>
          <button onClick={handleRefresh} className="btn btn-secondary flex items-center gap-2">
            <RefreshIcon className="w-4 h-4" />
            Refresh
          </button>
        </div>
      </div>

      {/* Health Score & Server Status */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-5">
        <div className="card lg:col-span-1">
          <div className="flex items-center gap-4">
            <HealthRing score={healthScore} />
            <div>
              <h2 className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{healthScore}%</h2>
              <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>Server Health</p>
              <p className="text-xs mt-1" style={{ color: 'var(--text-muted)' }}>KaosNet v{status?.version}</p>
            </div>
          </div>
          <div className="mt-4 pt-4 flex items-center gap-6" style={{ borderTop: '1px solid var(--border-primary)' }}>
            <div className="flex items-center gap-2">
              <ClockIcon className="w-4 h-4" style={{ color: 'var(--text-muted)' }} />
              <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>
                Uptime: <span className="font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {status ? formatUptime(status.uptime_secs) : '-'}
                </span>
              </span>
            </div>
          </div>
        </div>

        <div className="card lg:col-span-2">
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Activity Trend</h3>
            <div className="flex items-center gap-4 text-xs">
              <span className="flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full" style={{ background: '#06b6d4' }} />Sessions
              </span>
              <span className="flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full" style={{ background: '#8b5cf6' }} />Rooms
              </span>
              <span className="flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full" style={{ background: '#f59e0b' }} />Players
              </span>
            </div>
          </div>
          <MiniChart data={statusHistory} />
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <StatCard
          icon={<UsersIcon className="w-6 h-6" />}
          value={status?.sessions.total || 0}
          label="Total Sessions"
          trend={getTrend(statusHistory, 'sessions')}
          color="cyan"
          onClick={() => onNavigate?.('sessions')}
        />
        <StatCard
          icon={<ConnectionIcon className="w-6 h-6" />}
          value={status?.sessions.authenticated || 0}
          label="Authenticated"
          trend={null}
          color="emerald"
        />
        <StatCard
          icon={<RoomsIcon className="w-6 h-6" />}
          value={status?.rooms.total || 0}
          label="Active Rooms"
          trend={getTrend(statusHistory, 'rooms')}
          color="violet"
          onClick={() => onNavigate?.('rooms')}
        />
        <StatCard
          icon={<GroupIcon className="w-6 h-6" />}
          value={status?.rooms.players || 0}
          label="In Game"
          trend={getTrend(statusHistory, 'players')}
          color="amber"
        />
      </div>

      {/* Three Column Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Session States */}
        <div className="card">
          <SectionHeader icon={<ChartIcon className="w-5 h-5 text-cyan-400" />} title="Session States" color="cyan" />
          <div className="space-y-4">
            <SessionStateRow label="Connecting" value={status?.sessions.connecting || 0} total={status?.sessions.total || 1} color="amber" icon={<ClockIcon className="w-4 h-4" />} />
            <SessionStateRow label="Connected" value={status?.sessions.connected || 0} total={status?.sessions.total || 1} color="sky" icon={<ConnectionIcon className="w-4 h-4" />} />
            <SessionStateRow label="Authenticated" value={status?.sessions.authenticated || 0} total={status?.sessions.total || 1} color="emerald" icon={<ShieldIcon className="w-4 h-4" />} />
          </div>
        </div>

        {/* Recent Sessions */}
        <div className="card">
          <div className="flex items-center justify-between mb-5">
            <SectionHeader icon={<UsersIcon className="w-5 h-5 text-violet-400" />} title="Recent Sessions" color="violet" />
            <button onClick={() => onNavigate?.('sessions')} className="text-xs text-cyan-500 hover:text-cyan-400">View all</button>
          </div>
          {recentSessions.length === 0 ? (
            <EmptyPlaceholder icon={<UsersIcon className="w-8 h-8" />} text="No active sessions" />
          ) : (
            <div className="space-y-2">
              {recentSessions.slice(0, 4).map((session) => (
                <SessionRow key={session.id} session={session} />
              ))}
            </div>
          )}
        </div>

        {/* Recent Rooms */}
        <div className="card">
          <div className="flex items-center justify-between mb-5">
            <SectionHeader icon={<RoomsIcon className="w-5 h-5 text-amber-400" />} title="Active Rooms" color="amber" />
            <button onClick={() => onNavigate?.('rooms')} className="text-xs text-cyan-500 hover:text-cyan-400">View all</button>
          </div>
          {recentRooms.length === 0 ? (
            <EmptyPlaceholder icon={<RoomsIcon className="w-8 h-8" />} text="No active rooms" />
          ) : (
            <div className="space-y-2">
              {recentRooms.slice(0, 4).map((room) => (
                <RoomRow key={room.id} room={room} />
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Activity Feed & Quick Actions */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="card">
          <SectionHeader icon={<ActivityIcon className="w-5 h-5 text-pink-400" />} title="Activity Feed" color="pink" />
          {activityFeed.length === 0 ? (
            <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>
              <ActivityIcon className="w-10 h-10 mx-auto mb-3 opacity-20" />
              <p className="text-sm">No recent activity</p>
              <p className="text-xs mt-1">Events will appear here in real-time</p>
            </div>
          ) : (
            <div className="space-y-3 max-h-64 overflow-y-auto">
              {activityFeed.map((item) => (
                <ActivityRow key={item.id} item={item} />
              ))}
            </div>
          )}
        </div>

        <div className="card">
          <SectionHeader icon={<BoltIcon className="w-5 h-5 text-emerald-400" />} title="Quick Actions" color="emerald" />
          <div className="grid grid-cols-2 gap-3">
            <QuickActionButton icon={<RefreshIcon className="w-5 h-5" />} label="Refresh Stats" description="Update all metrics" onClick={handleRefresh} />
            <QuickActionButton icon={<UsersIcon className="w-5 h-5" />} label="Sessions" description={`${status?.sessions.total || 0} active`} onClick={() => onNavigate?.('sessions')} />
            <QuickActionButton icon={<RoomsIcon className="w-5 h-5" />} label="Rooms" description={`${status?.rooms.total || 0} active`} onClick={() => onNavigate?.('rooms')} />
            <QuickActionButton icon={<TerminalIcon className="w-5 h-5" />} label="Lua Console" description="Scripts & RPCs" onClick={() => onNavigate?.('lua')} />
            <QuickActionButton icon={<ChartIcon className="w-5 h-5" />} label="Leaderboards" description="Rankings" onClick={() => onNavigate?.('leaderboards')} />
            <QuickActionButton icon={<BoltIcon className="w-5 h-5" />} label="Matchmaker" description="Queue status" onClick={() => onNavigate?.('matchmaker')} />
          </div>
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// Helper Functions
// =============================================================================

function calculateHealthScore(status: ServerStatus | null): number {
  if (!status) return 0;
  let score = 100;
  if (status.sessions.total > 1000) score -= 10;
  if (status.sessions.total > 5000) score -= 20;
  if (status.rooms.total > 100) score -= 5;
  return Math.max(0, Math.min(100, score));
}

function getTrend(history: StatusHistory[], key: 'sessions' | 'rooms' | 'players'): number | null {
  if (history.length < 2) return null;
  const recent = history[history.length - 1][key];
  const previous = history[history.length - 2][key];
  if (previous === 0) return recent > 0 ? 100 : 0;
  return Math.round(((recent - previous) / previous) * 100);
}

function getSessionColor(state: string): string {
  switch (state.toLowerCase()) {
    case 'authenticated': return 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)';
    case 'connected': return 'linear-gradient(135deg, #0ea5e9 0%, #0284c7 100%)';
    default: return 'linear-gradient(135deg, #6b7280 0%, #4b5563 100%)';
  }
}

function getStateBadge(state: string): string {
  switch (state.toLowerCase()) {
    case 'authenticated': return 'success';
    case 'connected': return 'info';
    default: return 'warning';
  }
}

function getActivityColor(type: ActivityItem['type']): string {
  switch (type) {
    case 'session_connect': return 'rgba(34, 197, 94, 0.2)';
    case 'session_disconnect': return 'rgba(239, 68, 68, 0.2)';
    case 'room_create': return 'rgba(139, 92, 246, 0.2)';
    case 'room_close': return 'rgba(245, 158, 11, 0.2)';
    default: return 'rgba(107, 114, 128, 0.2)';
  }
}

function getActivityIcon(type: ActivityItem['type']): React.ReactNode {
  const className = "w-4 h-4";
  switch (type) {
    case 'session_connect': return <UserPlusIcon className={className} style={{ color: '#22c55e' }} />;
    case 'session_disconnect': return <UserMinusIcon className={className} style={{ color: '#ef4444' }} />;
    case 'room_create': return <PlusIcon className={className} style={{ color: '#8b5cf6' }} />;
    case 'room_close': return <XIcon className={className} style={{ color: '#f59e0b' }} />;
    default: return <CircleIcon className={className} style={{ color: '#6b7280' }} />;
  }
}

// =============================================================================
// Subcomponents
// =============================================================================

function HealthRing({ score }: { score: number }) {
  const circumference = 2 * Math.PI * 36;
  const strokeDashoffset = circumference - (score / 100) * circumference;
  const color = score >= 80 ? '#22c55e' : score >= 50 ? '#f59e0b' : '#ef4444';

  return (
    <div className="relative w-20 h-20">
      <svg className="w-full h-full transform -rotate-90">
        <circle cx="40" cy="40" r="36" fill="none" stroke="var(--bg-tertiary)" strokeWidth="6" />
        <circle cx="40" cy="40" r="36" fill="none" stroke={color} strokeWidth="6" strokeLinecap="round"
          strokeDasharray={circumference} strokeDashoffset={strokeDashoffset} className="transition-all duration-500" />
      </svg>
      <div className="absolute inset-0 flex items-center justify-center">
        <ServerIcon className="w-6 h-6" style={{ color }} />
      </div>
    </div>
  );
}

function MiniChart({ data }: { data: StatusHistory[] }) {
  if (data.length < 2) {
    return (
      <div className="h-24 flex items-center justify-center" style={{ color: 'var(--text-muted)' }}>
        <span className="text-sm">Collecting data...</span>
      </div>
    );
  }

  const maxValue = Math.max(...data.map(d => Math.max(d.sessions, d.rooms, d.players)), 1);
  const getPath = (values: number[]) => {
    const width = 100 / (values.length - 1);
    return values.map((v, i) => `${i === 0 ? 'M' : 'L'} ${i * width} ${100 - (v / maxValue) * 80 - 10}`).join(' ');
  };

  return (
    <div className="h-24 relative">
      <svg className="w-full h-full" viewBox="0 0 100 100" preserveAspectRatio="none">
        <line x1="0" y1="50" x2="100" y2="50" stroke="var(--border-primary)" strokeWidth="0.2" />
        <path d={getPath(data.map(d => d.sessions))} fill="none" stroke="#06b6d4" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        <path d={getPath(data.map(d => d.rooms))} fill="none" stroke="#8b5cf6" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        <path d={getPath(data.map(d => d.players))} fill="none" stroke="#f59e0b" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </div>
  );
}

interface StatCardProps {
  icon: React.ReactNode;
  value: number;
  label: string;
  trend: number | null;
  color: 'cyan' | 'emerald' | 'violet' | 'amber';
  onClick?: () => void;
}

const STAT_COLORS = {
  cyan: { bg: 'rgba(6, 182, 212, 0.15)', border: 'rgba(6, 182, 212, 0.25)', text: '#06b6d4' },
  emerald: { bg: 'rgba(16, 185, 129, 0.15)', border: 'rgba(16, 185, 129, 0.25)', text: '#10b981' },
  violet: { bg: 'rgba(139, 92, 246, 0.15)', border: 'rgba(139, 92, 246, 0.25)', text: '#8b5cf6' },
  amber: { bg: 'rgba(245, 158, 11, 0.15)', border: 'rgba(245, 158, 11, 0.25)', text: '#f59e0b' },
};

function StatCard({ icon, value, label, trend, color, onClick }: StatCardProps) {
  const colors = STAT_COLORS[color];
  return (
    <div
      className={`flex items-center gap-3 p-3 rounded-lg ${onClick ? 'cursor-pointer hover:ring-1 hover:ring-cyan-500/30' : ''}`}
      style={{ background: 'var(--bg-secondary)', border: '1px solid var(--border-primary)' }}
      onClick={onClick}
    >
      <div
        className="flex items-center justify-center w-10 h-10 rounded-lg flex-shrink-0"
        style={{ background: colors.bg, border: `1px solid ${colors.border}`, color: colors.text }}
      >
        {icon}
      </div>
      <div className="flex flex-col min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-xl font-bold" style={{ color: 'var(--text-primary)' }}>{value.toLocaleString()}</span>
          {trend !== null && trend !== 0 && (
            <span className={`text-xs font-medium flex items-center gap-0.5 ${trend > 0 ? 'text-green-500' : 'text-red-500'}`}>
              {trend > 0 ? <TrendUpIcon className="w-3 h-3" /> : <TrendDownIcon className="w-3 h-3" />}
              {Math.abs(trend)}%
            </span>
          )}
        </div>
        <span className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>{label}</span>
      </div>
    </div>
  );
}

const STATE_COLORS = {
  amber: { bar: '#f59e0b', text: '#f59e0b' },
  sky: { bar: '#0ea5e9', text: '#0ea5e9' },
  emerald: { bar: '#22c55e', text: '#22c55e' },
};

function SessionStateRow({ label, value, total, color, icon }: { label: string; value: number; total: number; color: 'amber' | 'sky' | 'emerald'; icon: React.ReactNode }) {
  const percentage = total > 0 ? (value / total) * 100 : 0;
  const colors = STATE_COLORS[color];
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span style={{ color: colors.text }}>{icon}</span>
          <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>{label}</span>
        </div>
        <span className="font-semibold tabular-nums" style={{ color: 'var(--text-primary)' }}>{value}</span>
      </div>
      <div className="h-1.5 rounded-full overflow-hidden" style={{ background: 'var(--bg-tertiary)' }}>
        <div className="h-full rounded-full transition-all duration-500" style={{ width: `${percentage}%`, background: colors.bar }} />
      </div>
    </div>
  );
}

function SectionHeader({ icon, title, color }: { icon: React.ReactNode; title: string; color: string }) {
  const colorMap: Record<string, string> = {
    cyan: 'rgba(6, 182, 212, 0.2)',
    violet: 'rgba(139, 92, 246, 0.2)',
    amber: 'rgba(245, 158, 11, 0.2)',
    pink: 'rgba(236, 72, 153, 0.2)',
    emerald: 'rgba(34, 197, 94, 0.2)',
  };
  const borderMap: Record<string, string> = {
    cyan: 'rgba(6, 182, 212, 0.2)',
    violet: 'rgba(139, 92, 246, 0.2)',
    amber: 'rgba(245, 158, 11, 0.2)',
    pink: 'rgba(236, 72, 153, 0.2)',
    emerald: 'rgba(34, 197, 94, 0.2)',
  };
  return (
    <div className="flex items-center gap-3 mb-5">
      <div className="w-9 h-9 rounded-lg flex items-center justify-center"
        style={{ background: `linear-gradient(135deg, ${colorMap[color]} 0%, ${colorMap[color].replace('0.2', '0.05')} 100%)`, border: `1px solid ${borderMap[color]}` }}>
        {icon}
      </div>
      <h2 className="font-semibold" style={{ color: 'var(--text-primary)' }}>{title}</h2>
    </div>
  );
}

function EmptyPlaceholder({ icon, text }: { icon: React.ReactNode; text: string }) {
  return (
    <div className="text-center py-6" style={{ color: 'var(--text-muted)' }}>
      <div className="mx-auto mb-2 opacity-30">{icon}</div>
      <p className="text-sm">{text}</p>
    </div>
  );
}

function SessionRow({ session }: { session: SessionInfo }) {
  return (
    <div className="flex items-center gap-3 p-2.5 rounded-lg transition-colors hover:bg-white/5" style={{ background: 'var(--bg-tertiary)' }}>
      <div className="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold" style={{ background: getSessionColor(session.state), color: 'white' }}>
        {session.username?.charAt(0).toUpperCase() || '#'}
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate" style={{ color: 'var(--text-primary)' }}>{session.username || `Session #${session.id}`}</p>
        <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>{session.address}</p>
      </div>
      <span className={`badge badge-${getStateBadge(session.state)}`}>{session.state}</span>
    </div>
  );
}

function RoomRow({ room }: { room: RoomInfo }) {
  return (
    <div className="flex items-center gap-3 p-2.5 rounded-lg transition-colors hover:bg-white/5" style={{ background: 'var(--bg-tertiary)' }}>
      <div className="w-8 h-8 rounded-lg flex items-center justify-center" style={{ background: 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)', color: 'white' }}>
        <GamepadIcon className="w-4 h-4" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate" style={{ color: 'var(--text-primary)' }}>{room.label || room.id.slice(0, 8)}</p>
        <p className="text-xs" style={{ color: 'var(--text-muted)' }}>{room.player_count}/{room.max_players} players</p>
      </div>
      <span className="text-xs px-2 py-0.5 rounded" style={{ background: 'rgba(245, 158, 11, 0.15)', color: '#f59e0b' }}>{room.tick_rate} tick</span>
    </div>
  );
}

function ActivityRow({ item }: { item: ActivityItem }) {
  return (
    <div className="flex items-start gap-3">
      <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5" style={{ background: getActivityColor(item.type) }}>
        {getActivityIcon(item.type)}
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm" style={{ color: 'var(--text-primary)' }}>{item.message}</p>
        <p className="text-xs" style={{ color: 'var(--text-muted)' }}>{formatTimeAgo(Math.floor(item.timestamp.getTime() / 1000))}</p>
      </div>
    </div>
  );
}

function QuickActionButton({ icon, label, description, onClick }: { icon: React.ReactNode; label: string; description: string; onClick?: () => void }) {
  return (
    <button onClick={onClick} className="p-3 rounded-xl text-left transition-all duration-200 flex items-center gap-3 group hover:ring-1 hover:ring-cyan-500/30" style={{ background: 'var(--bg-tertiary)' }}>
      <span style={{ color: 'var(--text-muted)' }} className="group-hover:text-cyan-500 transition-colors">{icon}</span>
      <div className="min-w-0">
        <p className="text-sm font-medium truncate transition-colors" style={{ color: 'var(--text-primary)' }}>{label}</p>
        <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>{description}</p>
      </div>
    </button>
  );
}
