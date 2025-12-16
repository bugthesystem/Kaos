import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { SessionInfo } from '../api/types';

function formatDuration(startTimestamp: number): string {
  const seconds = Math.floor(Date.now() / 1000 - startTimestamp);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

export function SessionsPage() {
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);

  useEffect(() => {
    loadSessions();
  }, [page]);

  const loadSessions = async () => {
    setLoading(true);
    try {
      const data = await api.listSessions(page, 20);
      setSessions(data.items);
      setTotal(data.total);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load sessions');
    } finally {
      setLoading(false);
    }
  };

  const handleKick = async (id: number) => {
    if (!confirm('Are you sure you want to kick this session?')) return;
    try {
      await api.kickSession(id);
      loadSessions();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to kick session');
    }
  };

  const getStateBadge = (state: string) => {
    switch (state) {
      case 'authenticated':
        return <span className="badge badge-success">Authenticated</span>;
      case 'connected':
        return <span className="badge badge-info">Connected</span>;
      case 'connecting':
        return <span className="badge badge-warning">Connecting</span>;
      default:
        return <span className="badge badge-danger">{state}</span>;
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Sessions</h1>
          <p className="text-gray-400">Manage connected clients</p>
        </div>
        <button onClick={loadSessions} className="btn btn-secondary">
          Refresh
        </button>
      </div>

      {error && (
        <div className="bg-red-900/50 border border-red-700 text-red-300 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      <div className="card p-0 overflow-hidden">
        <table className="w-full">
          <thead className="bg-gray-900/50">
            <tr>
              <th className="table-header">ID</th>
              <th className="table-header">Address</th>
              <th className="table-header">State</th>
              <th className="table-header">User</th>
              <th className="table-header">Duration</th>
              <th className="table-header">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-700">
            {loading ? (
              <tr>
                <td colSpan={6} className="table-cell text-center text-gray-400">
                  Loading...
                </td>
              </tr>
            ) : sessions.length === 0 ? (
              <tr>
                <td colSpan={6} className="table-cell text-center text-gray-400">
                  No sessions found
                </td>
              </tr>
            ) : (
              sessions.map((session) => (
                <tr key={session.id} className="hover:bg-gray-700/50">
                  <td className="table-cell font-mono text-xs">{session.id}</td>
                  <td className="table-cell font-mono text-xs">{session.address}</td>
                  <td className="table-cell">{getStateBadge(session.state)}</td>
                  <td className="table-cell">
                    {session.username || session.user_id || '-'}
                  </td>
                  <td className="table-cell text-gray-400">
                    {formatDuration(session.connected_at)}
                  </td>
                  <td className="table-cell">
                    <button
                      onClick={() => handleKick(session.id)}
                      className="btn btn-danger btn-sm"
                    >
                      Kick
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>

        {total > 20 && (
          <div className="px-4 py-3 border-t border-gray-700 flex items-center justify-between">
            <span className="text-sm text-gray-400">
              Showing {(page - 1) * 20 + 1} - {Math.min(page * 20, total)} of {total}
            </span>
            <div className="flex gap-2">
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page === 1}
                className="btn btn-secondary btn-sm"
              >
                Previous
              </button>
              <button
                onClick={() => setPage(p => p + 1)}
                disabled={page * 20 >= total}
                className="btn btn-secondary btn-sm"
              >
                Next
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
