import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { ServerStatus } from '../api/types';

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);

  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

export function DashboardPage() {
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    loadStatus();
    const interval = setInterval(loadStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  const loadStatus = async () => {
    try {
      const data = await api.getStatus();
      setStatus(data);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load status');
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-900/50 border border-red-700 text-red-300 px-4 py-3 rounded-lg">
        {error}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-white">Dashboard</h1>
        <p className="text-gray-400">Server overview and statistics</p>
      </div>

      {/* Server Info */}
      <div className="card">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold text-white">Server Status</h2>
            <p className="text-gray-400 text-sm">KaosNet v{status?.version}</p>
          </div>
          <span className="badge badge-success">Online</span>
        </div>
        <div className="mt-4 text-sm text-gray-400">
          Uptime: {status ? formatUptime(status.uptime_secs) : '-'}
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="stat-card">
          <span className="stat-value">{status?.sessions.total || 0}</span>
          <span className="stat-label">Total Sessions</span>
        </div>
        <div className="stat-card">
          <span className="stat-value">{status?.sessions.connected || 0}</span>
          <span className="stat-label">Connected</span>
        </div>
        <div className="stat-card">
          <span className="stat-value">{status?.rooms.total || 0}</span>
          <span className="stat-label">Active Rooms</span>
        </div>
        <div className="stat-card">
          <span className="stat-value">{status?.rooms.players || 0}</span>
          <span className="stat-label">Players in Rooms</span>
        </div>
      </div>

      {/* Session Breakdown */}
      <div className="card">
        <h2 className="text-lg font-semibold text-white mb-4">Session States</h2>
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-gray-400">Connecting</span>
            <span className="text-white font-medium">{status?.sessions.connecting || 0}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-gray-400">Connected</span>
            <span className="text-white font-medium">{status?.sessions.connected || 0}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-gray-400">Authenticated</span>
            <span className="text-white font-medium">{status?.sessions.authenticated || 0}</span>
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <div className="card">
        <h2 className="text-lg font-semibold text-white mb-4">Quick Actions</h2>
        <div className="flex gap-3 flex-wrap">
          <button className="btn btn-secondary" onClick={loadStatus}>
            Refresh Stats
          </button>
        </div>
      </div>
    </div>
  );
}
