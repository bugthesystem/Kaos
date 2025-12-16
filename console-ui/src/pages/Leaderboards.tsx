import { useState, useEffect } from 'react';
import { api } from '../api/client';

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
  metadata: any;
  updated_at: number;
}

export default function Leaderboards() {
  const [leaderboards, setLeaderboards] = useState<Leaderboard[]>([]);
  const [records, setRecords] = useState<LeaderboardRecord[]>([]);
  const [selectedLeaderboard, setSelectedLeaderboard] = useState<Leaderboard | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
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
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const loadRecords = async (leaderboardId: string) => {
    try {
      const data = await api.get(`/api/leaderboards/${leaderboardId}/records?limit=100`);
      setRecords(data.records || []);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const createLeaderboard = async () => {
    try {
      await api.post('/api/leaderboards', newLeaderboard);
      setShowCreate(false);
      setNewLeaderboard({ id: '', sort_order: 'descending' });
      loadLeaderboards();
    } catch (err: any) {
      alert('Failed to create: ' + err.message);
    }
  };

  const deleteLeaderboard = async (id: string) => {
    if (!confirm('Delete this leaderboard and all records?')) return;
    try {
      await api.delete(`/api/leaderboards/${id}`);
      setSelectedLeaderboard(null);
      loadLeaderboards();
    } catch (err: any) {
      alert('Failed to delete: ' + err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  if (loading && leaderboards.length === 0) {
    return <div className="p-6">Loading leaderboards...</div>;
  }

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Leaderboards</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
        >
          Create Leaderboard
        </button>
      </div>

      {/* Create Modal */}
      {showCreate && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-800 rounded-lg p-6 w-96">
            <h2 className="text-xl font-semibold mb-4">Create Leaderboard</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">ID</label>
                <input
                  type="text"
                  value={newLeaderboard.id}
                  onChange={(e) => setNewLeaderboard({...newLeaderboard, id: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  placeholder="e.g., weekly_scores"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Sort Order</label>
                <select
                  value={newLeaderboard.sort_order}
                  onChange={(e) => setNewLeaderboard({...newLeaderboard, sort_order: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                >
                  <option value="descending">Descending (highest first)</option>
                  <option value="ascending">Ascending (lowest first)</option>
                </select>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button
                onClick={() => setShowCreate(false)}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded"
              >
                Cancel
              </button>
              <button
                onClick={createLeaderboard}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Leaderboard List */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            Leaderboards ({leaderboards.length})
          </div>
          <div className="divide-y divide-gray-700">
            {leaderboards.length === 0 ? (
              <div className="px-4 py-3 text-gray-400">No leaderboards</div>
            ) : (
              leaderboards.map((lb) => (
                <div
                  key={lb.id}
                  className={`px-4 py-3 cursor-pointer hover:bg-gray-700 ${
                    selectedLeaderboard?.id === lb.id ? 'bg-gray-700' : ''
                  }`}
                  onClick={() => setSelectedLeaderboard(lb)}
                >
                  <div className="font-medium">{lb.id}</div>
                  <div className="text-sm text-gray-400 mt-1">
                    {lb.record_count} records - {lb.sort_order}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Records */}
        <div className="lg:col-span-2 bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 flex justify-between items-center">
            <span className="font-semibold">
              {selectedLeaderboard ? `Records - ${selectedLeaderboard.id}` : 'Select a leaderboard'}
            </span>
            {selectedLeaderboard && (
              <button
                onClick={() => deleteLeaderboard(selectedLeaderboard.id)}
                className="text-red-400 hover:text-red-300 text-sm"
              >
                Delete
              </button>
            )}
          </div>
          <table className="w-full">
            <thead className="bg-gray-900/50">
              <tr>
                <th className="px-4 py-2 text-left">Rank</th>
                <th className="px-4 py-2 text-left">Player</th>
                <th className="px-4 py-2 text-right">Score</th>
                <th className="px-4 py-2 text-right">Updated</th>
              </tr>
            </thead>
            <tbody>
              {selectedLeaderboard ? (
                records.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="px-4 py-3 text-gray-400 text-center">
                      No records
                    </td>
                  </tr>
                ) : (
                  records.map((record) => (
                    <tr key={record.owner_id} className="border-t border-gray-700">
                      <td className="px-4 py-2">
                        <span className={record.rank <= 3 ? 'text-yellow-400 font-bold' : ''}>
                          #{record.rank}
                        </span>
                      </td>
                      <td className="px-4 py-2">{record.username || record.owner_id}</td>
                      <td className="px-4 py-2 text-right font-mono">{record.score.toLocaleString()}</td>
                      <td className="px-4 py-2 text-right text-gray-400 text-sm">
                        {formatDate(record.updated_at)}
                      </td>
                    </tr>
                  ))
                )
              ) : (
                <tr>
                  <td colSpan={4} className="px-4 py-3 text-gray-400 text-center">
                    Select a leaderboard to view records
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
