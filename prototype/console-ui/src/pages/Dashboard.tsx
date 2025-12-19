import { useEffect, useState, useCallback } from 'react';
import { api } from '../api/client';
import { useToast } from '../components/Toast';
import type { ServerStatus, SessionInfo, RoomInfo } from '../api/types';

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

function formatTimeAgo(timestamp: number): string {
  const seconds = Math.floor(Date.now() / 1000 - timestamp);
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

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

      // Update status
      setStatus(statusData);
      setRecentSessions(sessionsData.items || []);
      setRecentRooms(roomsData.items || []);
      setError('');
      setLastUpdated(new Date());

      // Update history for mini-chart (keep last 20 data points)
      setStatusHistory(prev => {
        const newEntry = {
          timestamp: new Date(),
          sessions: statusData.sessions.total,
          rooms: statusData.rooms.total,
          players: statusData.rooms.players,
        };
        return [...prev.slice(-19), newEntry];
      });

      // Simulate activity feed updates based on session/room changes
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
        <div className="flex items-center gap-3" style={{ color: 'var(--text-muted)' }}>
          <SpinnerIcon className="w-6 h-6 animate-spin" />
          <span>Loading dashboard...</span>
        </div>
      </div>
    );
  }

  if (error && !status) {
    return (
      <div className="alert alert-danger flex items-center gap-3">
        <ErrorIcon className="w-5 h-5 flex-shrink-0" />
        <div className="flex-1">
          <p className="font-medium">Connection Error</p>
          <p className="text-sm opacity-80">{error}</p>
        </div>
        <button onClick={handleRefresh} className="btn btn-secondary btn-sm">
          Retry
        </button>
      </div>
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
            <span className="flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium"
              style={{
                background: 'rgba(34, 197, 94, 0.15)',
                color: '#22c55e',
              }}>
              <span className="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse" />
              Live
            </span>
          </h1>
          <p className="page-subtitle">
            Real-time server overview and monitoring
          </p>
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
        {/* Server Health Card */}
        <div className="card lg:col-span-1">
          <div className="flex items-center gap-4">
            <HealthRing score={healthScore} />
            <div>
              <h2 className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                {healthScore}%
              </h2>
              <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>Server Health</p>
              <p className="text-xs mt-1" style={{ color: 'var(--text-muted)' }}>
                KaosNet v{status?.version}
              </p>
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

        {/* Mini Stats Chart */}
        <div className="card lg:col-span-2">
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Activity Trend</h3>
            <div className="flex items-center gap-4 text-xs">
              <span className="flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full" style={{ background: '#06b6d4' }} />
                Sessions
              </span>
              <span className="flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full" style={{ background: '#8b5cf6' }} />
                Rooms
              </span>
              <span className="flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full" style={{ background: '#f59e0b' }} />
                Players
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
          icon={<PlayersIcon className="w-6 h-6" />}
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
          <div className="flex items-center gap-3 mb-5">
            <div className="w-9 h-9 rounded-lg flex items-center justify-center"
              style={{
                background: 'linear-gradient(135deg, rgba(6, 182, 212, 0.2) 0%, rgba(6, 182, 212, 0.05) 100%)',
                border: '1px solid rgba(6, 182, 212, 0.2)',
              }}
            >
              <ChartIcon className="w-5 h-5 text-cyan-400" />
            </div>
            <h2 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Session States</h2>
          </div>
          <div className="space-y-4">
            <SessionStateRow
              label="Connecting"
              value={status?.sessions.connecting || 0}
              total={status?.sessions.total || 1}
              color="amber"
              icon={<PendingIcon className="w-4 h-4" />}
            />
            <SessionStateRow
              label="Connected"
              value={status?.sessions.connected || 0}
              total={status?.sessions.total || 1}
              color="sky"
              icon={<ConnectedIcon className="w-4 h-4" />}
            />
            <SessionStateRow
              label="Authenticated"
              value={status?.sessions.authenticated || 0}
              total={status?.sessions.total || 1}
              color="emerald"
              icon={<AuthenticatedIcon className="w-4 h-4" />}
            />
          </div>
        </div>

        {/* Recent Sessions */}
        <div className="card">
          <div className="flex items-center justify-between mb-5">
            <div className="flex items-center gap-3">
              <div className="w-9 h-9 rounded-lg flex items-center justify-center"
                style={{
                  background: 'linear-gradient(135deg, rgba(139, 92, 246, 0.2) 0%, rgba(139, 92, 246, 0.05) 100%)',
                  border: '1px solid rgba(139, 92, 246, 0.2)',
                }}
              >
                <UsersIcon className="w-5 h-5 text-violet-400" />
              </div>
              <h2 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Recent Sessions</h2>
            </div>
            <button onClick={() => onNavigate?.('sessions')} className="text-xs text-cyan-500 hover:text-cyan-400">
              View all
            </button>
          </div>
          {recentSessions.length === 0 ? (
            <div className="text-center py-6" style={{ color: 'var(--text-muted)' }}>
              <UsersIcon className="w-8 h-8 mx-auto mb-2 opacity-30" />
              <p className="text-sm">No active sessions</p>
            </div>
          ) : (
            <div className="space-y-2">
              {recentSessions.slice(0, 4).map((session) => (
                <div
                  key={session.id}
                  className="flex items-center gap-3 p-2.5 rounded-lg transition-colors hover:bg-white/5"
                  style={{ background: 'var(--bg-tertiary)' }}
                >
                  <div
                    className="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold"
                    style={{
                      background: getSessionColor(session.state),
                      color: 'white',
                    }}
                  >
                    {session.username?.charAt(0).toUpperCase() || '#'}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate" style={{ color: 'var(--text-primary)' }}>
                      {session.username || `Session #${session.id}`}
                    </p>
                    <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>
                      {session.address}
                    </p>
                  </div>
                  <span className={`badge badge-${getStateBadge(session.state)}`}>
                    {session.state}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Recent Rooms */}
        <div className="card">
          <div className="flex items-center justify-between mb-5">
            <div className="flex items-center gap-3">
              <div className="w-9 h-9 rounded-lg flex items-center justify-center"
                style={{
                  background: 'linear-gradient(135deg, rgba(245, 158, 11, 0.2) 0%, rgba(245, 158, 11, 0.05) 100%)',
                  border: '1px solid rgba(245, 158, 11, 0.2)',
                }}
              >
                <RoomsIcon className="w-5 h-5 text-amber-400" />
              </div>
              <h2 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Active Rooms</h2>
            </div>
            <button onClick={() => onNavigate?.('rooms')} className="text-xs text-cyan-500 hover:text-cyan-400">
              View all
            </button>
          </div>
          {recentRooms.length === 0 ? (
            <div className="text-center py-6" style={{ color: 'var(--text-muted)' }}>
              <RoomsIcon className="w-8 h-8 mx-auto mb-2 opacity-30" />
              <p className="text-sm">No active rooms</p>
            </div>
          ) : (
            <div className="space-y-2">
              {recentRooms.slice(0, 4).map((room) => (
                <div
                  key={room.id}
                  className="flex items-center gap-3 p-2.5 rounded-lg transition-colors hover:bg-white/5"
                  style={{ background: 'var(--bg-tertiary)' }}
                >
                  <div
                    className="w-8 h-8 rounded-lg flex items-center justify-center"
                    style={{
                      background: 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
                      color: 'white',
                    }}
                  >
                    <GamepadIcon className="w-4 h-4" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate" style={{ color: 'var(--text-primary)' }}>
                      {room.label || room.id.slice(0, 8)}
                    </p>
                    <p className="text-xs" style={{ color: 'var(--text-muted)' }}>
                      {room.player_count}/{room.max_players} players
                    </p>
                  </div>
                  <span className="text-xs px-2 py-0.5 rounded" style={{
                    background: 'rgba(245, 158, 11, 0.15)',
                    color: '#f59e0b',
                  }}>
                    {room.tick_rate} tick
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Activity Feed & Quick Actions */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Activity Feed */}
        <div className="card">
          <div className="flex items-center gap-3 mb-5">
            <div className="w-9 h-9 rounded-lg flex items-center justify-center"
              style={{
                background: 'linear-gradient(135deg, rgba(236, 72, 153, 0.2) 0%, rgba(236, 72, 153, 0.05) 100%)',
                border: '1px solid rgba(236, 72, 153, 0.2)',
              }}
            >
              <ActivityIcon className="w-5 h-5 text-pink-400" />
            </div>
            <h2 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Activity Feed</h2>
          </div>
          {activityFeed.length === 0 ? (
            <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>
              <ActivityIcon className="w-10 h-10 mx-auto mb-3 opacity-20" />
              <p className="text-sm">No recent activity</p>
              <p className="text-xs mt-1">Events will appear here in real-time</p>
            </div>
          ) : (
            <div className="space-y-3 max-h-64 overflow-y-auto">
              {activityFeed.map((item) => (
                <div key={item.id} className="flex items-start gap-3">
                  <div
                    className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5"
                    style={{
                      background: getActivityColor(item.type),
                    }}
                  >
                    {getActivityIcon(item.type)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm" style={{ color: 'var(--text-primary)' }}>
                      {item.message}
                    </p>
                    <p className="text-xs" style={{ color: 'var(--text-muted)' }}>
                      {formatTimeAgo(Math.floor(item.timestamp.getTime() / 1000))}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Quick Actions */}
        <div className="card">
          <div className="flex items-center gap-3 mb-5">
            <div className="w-9 h-9 rounded-lg flex items-center justify-center"
              style={{
                background: 'linear-gradient(135deg, rgba(34, 197, 94, 0.2) 0%, rgba(34, 197, 94, 0.05) 100%)',
                border: '1px solid rgba(34, 197, 94, 0.2)',
              }}
            >
              <BoltIcon className="w-5 h-5 text-emerald-400" />
            </div>
            <h2 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Quick Actions</h2>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <QuickActionButton
              icon={<RefreshIcon className="w-5 h-5" />}
              label="Refresh Stats"
              description="Update all metrics"
              onClick={handleRefresh}
            />
            <QuickActionButton
              icon={<UsersIcon className="w-5 h-5" />}
              label="Sessions"
              description={`${status?.sessions.total || 0} active`}
              onClick={() => onNavigate?.('sessions')}
            />
            <QuickActionButton
              icon={<RoomsIcon className="w-5 h-5" />}
              label="Rooms"
              description={`${status?.rooms.total || 0} active`}
              onClick={() => onNavigate?.('rooms')}
            />
            <QuickActionButton
              icon={<TerminalIcon className="w-5 h-5" />}
              label="Lua Console"
              description="Scripts & RPCs"
              onClick={() => onNavigate?.('lua')}
            />
            <QuickActionButton
              icon={<LeaderboardIcon className="w-5 h-5" />}
              label="Leaderboards"
              description="Rankings"
              onClick={() => onNavigate?.('leaderboards')}
            />
            <QuickActionButton
              icon={<MatchmakerIcon className="w-5 h-5" />}
              label="Matchmaker"
              description="Queue status"
              onClick={() => onNavigate?.('matchmaker')}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

// Helper functions
function calculateHealthScore(status: ServerStatus | null): number {
  if (!status) return 0;
  // Simple health calculation - can be expanded
  let score = 100;
  // Deduct points for high session load (over 1000 sessions)
  if (status.sessions.total > 1000) score -= 10;
  if (status.sessions.total > 5000) score -= 20;
  // Deduct for high rooms
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

// Components
function HealthRing({ score }: { score: number }) {
  const circumference = 2 * Math.PI * 36;
  const strokeDashoffset = circumference - (score / 100) * circumference;
  const color = score >= 80 ? '#22c55e' : score >= 50 ? '#f59e0b' : '#ef4444';

  return (
    <div className="relative w-20 h-20">
      <svg className="w-full h-full transform -rotate-90">
        <circle
          cx="40"
          cy="40"
          r="36"
          fill="none"
          stroke="var(--bg-tertiary)"
          strokeWidth="6"
        />
        <circle
          cx="40"
          cy="40"
          r="36"
          fill="none"
          stroke={color}
          strokeWidth="6"
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={strokeDashoffset}
          className="transition-all duration-500"
        />
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

  const maxSessions = Math.max(...data.map(d => d.sessions), 1);
  const maxRooms = Math.max(...data.map(d => d.rooms), 1);
  const maxPlayers = Math.max(...data.map(d => d.players), 1);
  const maxValue = Math.max(maxSessions, maxRooms, maxPlayers);

  const getPath = (values: number[], max: number) => {
    const width = 100 / (values.length - 1);
    return values.map((v, i) => {
      const x = i * width;
      const y = 100 - (v / max) * 80 - 10;
      return `${i === 0 ? 'M' : 'L'} ${x} ${y}`;
    }).join(' ');
  };

  return (
    <div className="h-24 relative">
      <svg className="w-full h-full" viewBox="0 0 100 100" preserveAspectRatio="none">
        {/* Grid lines */}
        <line x1="0" y1="50" x2="100" y2="50" stroke="var(--border-primary)" strokeWidth="0.2" />

        {/* Sessions line */}
        <path
          d={getPath(data.map(d => d.sessions), maxValue)}
          fill="none"
          stroke="#06b6d4"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Rooms line */}
        <path
          d={getPath(data.map(d => d.rooms), maxValue)}
          fill="none"
          stroke="#8b5cf6"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Players line */}
        <path
          d={getPath(data.map(d => d.players), maxValue)}
          fill="none"
          stroke="#f59e0b"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
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

function StatCard({ icon, value, label, trend, color, onClick }: StatCardProps) {
  const colors = {
    cyan: { bg: 'rgba(6, 182, 212, 0.15)', border: 'rgba(6, 182, 212, 0.25)', text: '#06b6d4' },
    emerald: { bg: 'rgba(16, 185, 129, 0.15)', border: 'rgba(16, 185, 129, 0.25)', text: '#10b981' },
    violet: { bg: 'rgba(139, 92, 246, 0.15)', border: 'rgba(139, 92, 246, 0.25)', text: '#8b5cf6' },
    amber: { bg: 'rgba(245, 158, 11, 0.15)', border: 'rgba(245, 158, 11, 0.25)', text: '#f59e0b' },
  };

  return (
    <div
      className={`stat-card ${onClick ? 'cursor-pointer hover:ring-1 hover:ring-cyan-500/30' : ''}`}
      onClick={onClick}
    >
      <div className="flex items-start justify-between w-full mb-2">
        <div
          className="stat-icon"
          style={{
            background: colors[color].bg,
            border: `1px solid ${colors[color].border}`,
            color: colors[color].text,
          }}
        >
          {icon}
        </div>
        {trend !== null && trend !== 0 && (
          <span className={`text-xs font-medium flex items-center gap-0.5 ${trend > 0 ? 'text-green-500' : 'text-red-500'}`}>
            {trend > 0 ? <TrendUpIcon className="w-3 h-3" /> : <TrendDownIcon className="w-3 h-3" />}
            {Math.abs(trend)}%
          </span>
        )}
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
  icon: React.ReactNode;
}

function SessionStateRow({ label, value, total, color, icon }: SessionStateRowProps) {
  const percentage = total > 0 ? (value / total) * 100 : 0;
  const colors = {
    amber: { bg: 'rgba(245, 158, 11, 0.2)', bar: '#f59e0b', text: '#f59e0b' },
    sky: { bg: 'rgba(14, 165, 233, 0.2)', bar: '#0ea5e9', text: '#0ea5e9' },
    emerald: { bg: 'rgba(34, 197, 94, 0.2)', bar: '#22c55e', text: '#22c55e' },
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span style={{ color: colors[color].text }}>{icon}</span>
          <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>{label}</span>
        </div>
        <span className="font-semibold tabular-nums" style={{ color: 'var(--text-primary)' }}>{value}</span>
      </div>
      <div className="h-1.5 rounded-full overflow-hidden" style={{ background: 'var(--bg-tertiary)' }}>
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
  description: string;
  onClick?: () => void;
}

function QuickActionButton({ icon, label, description, onClick }: QuickActionButtonProps) {
  return (
    <button
      onClick={onClick}
      className="p-3 rounded-xl text-left transition-all duration-200 flex items-center gap-3 group hover:ring-1 hover:ring-cyan-500/30"
      style={{
        background: 'var(--bg-tertiary)',
      }}
    >
      <span style={{ color: 'var(--text-muted)' }} className="group-hover:text-cyan-500 transition-colors">{icon}</span>
      <div className="min-w-0">
        <p className="text-sm font-medium truncate transition-colors" style={{ color: 'var(--text-primary)' }}>{label}</p>
        <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>{description}</p>
      </div>
    </button>
  );
}

// Icons
function ServerIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
    </svg>
  );
}

function ClockIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
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

function LeaderboardIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
    </svg>
  );
}

function MatchmakerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  );
}

function ActivityIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
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

function SpinnerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
    </svg>
  );
}

function GamepadIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function PendingIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function ConnectedIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
    </svg>
  );
}

function AuthenticatedIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
    </svg>
  );
}

function TrendUpIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
    </svg>
  );
}

function TrendDownIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 17h8m0 0V9m0 8l-8-8-4 4-6-6" />
    </svg>
  );
}

function UserPlusIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
    </svg>
  );
}

function UserMinusIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 7a4 4 0 11-8 0 4 4 0 018 0zM9 14a6 6 0 00-6 6v1h12v-1a6 6 0 00-6-6zM21 12h-6" />
    </svg>
  );
}

function PlusIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
    </svg>
  );
}

function XIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

function CircleIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="currentColor" viewBox="0 0 24 24">
      <circle cx="12" cy="12" r="4" />
    </svg>
  );
}
