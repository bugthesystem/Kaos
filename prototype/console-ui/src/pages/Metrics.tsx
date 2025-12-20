import { useEffect, useState, useCallback } from 'react';
import { api } from '../api/client';
import type { MetricsData } from '../api/types';
import { formatBytes, formatNumber, formatUptime } from '../utils';
import { StatCard, PageHeader, Card, Spinner, Alert, StatGrid, Badge } from '../components/ui';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
  PieChart,
  Pie,
  Cell,
} from 'recharts';

interface MetricsHistory {
  timestamp: Date;
  sessions: number;
  rooms: number;
  websockets: number;
  matchmakerQueue: number;
}

const COLORS = {
  primary: '#06b6d4',
  secondary: '#8b5cf6',
  success: '#22c55e',
  warning: '#f59e0b',
  danger: '#ef4444',
  info: '#3b82f6',
};

const SESSION_COLORS = ['#22c55e', '#3b82f6', '#f59e0b'];

export function MetricsPage() {
  const [metrics, setMetrics] = useState<MetricsData | null>(null);
  const [history, setHistory] = useState<MetricsHistory[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date());

  const loadMetrics = useCallback(async () => {
    try {
      const data = await api.getMetrics();
      setMetrics(data);
      setError('');
      setLastUpdated(new Date());

      setHistory((prev) => {
        const newEntry = {
          timestamp: new Date(),
          sessions: data.sessions_active,
          rooms: data.rooms_active,
          websockets: data.websocket_connections,
          matchmakerQueue: data.matchmaker_queue_size,
        };
        return [...prev.slice(-59), newEntry];
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load metrics');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadMetrics();
    const interval = setInterval(loadMetrics, 5000);
    return () => clearInterval(interval);
  }, [loadMetrics]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Spinner label="Loading metrics..." />
      </div>
    );
  }

  if (error && !metrics) {
    return (
      <Alert variant="danger" title="Metrics Error">
        {error}
        <button onClick={loadMetrics} className="btn btn-secondary btn-sm ml-3">
          Retry
        </button>
      </Alert>
    );
  }

  const sessionStateData = metrics
    ? Object.entries(metrics.sessions_by_state).map(([name, value]) => ({
        name: name.charAt(0).toUpperCase() + name.slice(1),
        value,
      }))
    : [];

  const historyData = history.map((h, i) => ({
    time: i,
    sessions: h.sessions,
    rooms: h.rooms,
    websockets: h.websockets,
    queue: h.matchmakerQueue,
  }));

  const serviceStats = metrics
    ? [
        { name: 'Chat', value: metrics.chat_messages_total, color: COLORS.primary },
        { name: 'Leaderboard', value: metrics.leaderboard_submissions_total, color: COLORS.secondary },
        { name: 'Matches', value: metrics.matchmaker_matches_total, color: COLORS.success },
        { name: 'Notifications', value: metrics.notifications_total, color: COLORS.warning },
      ]
    : [];

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader
        title="Metrics"
        subtitle="Real-time server metrics and performance data"
        badge={
          <Badge variant="success" size="sm">
            <span className="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse mr-1.5" />
            Live
          </Badge>
        }
        actions={
          <>
            <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
              Updated {lastUpdated.toLocaleTimeString()}
            </span>
            <button onClick={loadMetrics} className="btn btn-secondary flex items-center gap-2">
              <RefreshIcon className="w-4 h-4" />
              Refresh
            </button>
          </>
        }
      />

      {/* Key Stats */}
      <StatGrid columns={6}>
        <StatCard
          label="Uptime"
          value={metrics ? formatUptime(metrics.uptime_seconds) : '-'}
          icon={<ClockIcon className="w-4 h-4" />}
          color="success"
        />
        <StatCard
          label="Active Sessions"
          value={formatNumber(metrics?.sessions_active || 0)}
          icon={<UsersIcon className="w-4 h-4" />}
          color="primary"
        />
        <StatCard
          label="Active Rooms"
          value={formatNumber(metrics?.rooms_active || 0)}
          icon={<RoomsIcon className="w-4 h-4" />}
          color="secondary"
        />
        <StatCard
          label="WebSocket Conns"
          value={formatNumber(metrics?.websocket_connections || 0)}
          icon={<ConnectionIcon className="w-4 h-4" />}
          color="info"
        />
        <StatCard
          label="Matchmaker Queue"
          value={formatNumber(metrics?.matchmaker_queue_size || 0)}
          icon={<QueueIcon className="w-4 h-4" />}
          color="warning"
        />
        <StatCard
          label="Total Sessions"
          value={formatNumber(metrics?.sessions_total || 0)}
          icon={<TotalIcon className="w-4 h-4" />}
          color="muted"
        />
      </StatGrid>

      {/* Charts Row 1 */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card title="Real-time Activity">
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={historyData}>
                <defs>
                  <linearGradient id="colorSessions" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={COLORS.primary} stopOpacity={0.3} />
                    <stop offset="95%" stopColor={COLORS.primary} stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="colorRooms" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={COLORS.secondary} stopOpacity={0.3} />
                    <stop offset="95%" stopColor={COLORS.secondary} stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--border-primary)" />
                <XAxis dataKey="time" tick={false} stroke="var(--text-muted)" />
                <YAxis stroke="var(--text-muted)" />
                <Tooltip
                  contentStyle={{
                    background: 'var(--bg-secondary)',
                    border: '1px solid var(--border-primary)',
                    borderRadius: '8px',
                  }}
                />
                <Legend />
                <Area type="monotone" dataKey="sessions" stroke={COLORS.primary} fillOpacity={1} fill="url(#colorSessions)" name="Sessions" />
                <Area type="monotone" dataKey="rooms" stroke={COLORS.secondary} fillOpacity={1} fill="url(#colorRooms)" name="Rooms" />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </Card>

        <Card title="Session States">
          <div className="h-64 flex items-center justify-center">
            {sessionStateData.length > 0 ? (
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={sessionStateData}
                    cx="50%"
                    cy="50%"
                    innerRadius={60}
                    outerRadius={80}
                    paddingAngle={5}
                    dataKey="value"
                    label={({ name, value }) => `${name}: ${value}`}
                    labelLine={false}
                  >
                    {sessionStateData.map((_, index) => (
                      <Cell key={`cell-${index}`} fill={SESSION_COLORS[index % SESSION_COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip contentStyle={{ background: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', borderRadius: '8px' }} />
                  <Legend />
                </PieChart>
              </ResponsiveContainer>
            ) : (
              <div className="text-center" style={{ color: 'var(--text-muted)' }}>
                <UsersIcon className="w-12 h-12 mx-auto mb-2 opacity-30" />
                <p>No active sessions</p>
              </div>
            )}
          </div>
        </Card>
      </div>

      {/* Charts Row 2 */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card title="Network Traffic">
          <div className="grid grid-cols-2 gap-3 mb-3">
            <div className="p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
              <div className="flex items-center gap-2 mb-1">
                <span style={{ color: COLORS.success }}><DownloadIcon className="w-4 h-4" /></span>
                <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>Received</span>
              </div>
              <p className="text-xl font-bold" style={{ color: 'var(--text-primary)' }}>
                {formatBytes(metrics?.bytes_received_total || 0)}
              </p>
            </div>
            <div className="p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
              <div className="flex items-center gap-2 mb-1">
                <span style={{ color: COLORS.info }}><UploadIcon className="w-4 h-4" /></span>
                <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>Sent</span>
              </div>
              <p className="text-xl font-bold" style={{ color: 'var(--text-primary)' }}>
                {formatBytes(metrics?.bytes_sent_total || 0)}
              </p>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
              <div className="flex items-center gap-2 mb-1">
                <span style={{ color: COLORS.warning }}><PacketIcon className="w-4 h-4" /></span>
                <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>UDP Recv</span>
              </div>
              <p className="text-lg font-bold" style={{ color: 'var(--text-primary)' }}>
                {formatNumber(metrics?.udp_packets_received_total || 0)}
              </p>
            </div>
            <div className="p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
              <div className="flex items-center gap-2 mb-1">
                <span style={{ color: COLORS.secondary }}><PacketIcon className="w-4 h-4" /></span>
                <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>UDP Sent</span>
              </div>
              <p className="text-lg font-bold" style={{ color: 'var(--text-primary)' }}>
                {formatNumber(metrics?.udp_packets_sent_total || 0)}
              </p>
            </div>
          </div>
        </Card>

        <Card title="Service Activity (Total)">
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={serviceStats} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="var(--border-primary)" />
                <XAxis type="number" stroke="var(--text-muted)" />
                <YAxis type="category" dataKey="name" stroke="var(--text-muted)" width={100} />
                <Tooltip
                  contentStyle={{ background: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', borderRadius: '8px' }}
                  formatter={(value: number) => [formatNumber(value), 'Count']}
                />
                <Bar dataKey="value" radius={[0, 4, 4, 0]}>
                  {serviceStats.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        </Card>
      </div>

      {/* WebSocket & Matchmaker Chart */}
      <Card title="WebSocket Connections & Matchmaker Queue">
        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={historyData}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border-primary)" />
              <XAxis dataKey="time" tick={false} stroke="var(--text-muted)" />
              <YAxis stroke="var(--text-muted)" />
              <Tooltip contentStyle={{ background: 'var(--bg-secondary)', border: '1px solid var(--border-primary)', borderRadius: '8px' }} />
              <Legend />
              <Line type="monotone" dataKey="websockets" stroke={COLORS.info} strokeWidth={2} dot={false} name="WebSocket Connections" />
              <Line type="monotone" dataKey="queue" stroke={COLORS.warning} strokeWidth={2} dot={false} name="Matchmaker Queue" />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </Card>
    </div>
  );
}

// Icons (minimal set needed for this page)
function RefreshIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
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

function UsersIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
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

function ConnectionIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
    </svg>
  );
}

function QueueIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}

function TotalIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
    </svg>
  );
}

function DownloadIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
    </svg>
  );
}

function UploadIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
    </svg>
  );
}

function PacketIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
    </svg>
  );
}
