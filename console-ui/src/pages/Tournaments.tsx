import { useState, useEffect } from 'react';
import { api } from '../api/client';

interface Tournament {
  id: string;
  title: string;
  description: string;
  category: number;
  sort_order: string;
  size: number;
  max_size: number;
  max_num_score: number;
  start_time: number;
  end_time: number | null;
  duration: number;
  reset_schedule: string | null;
  metadata: any;
  created_at: number;
}

interface TournamentRecord {
  owner_id: string;
  username: string;
  score: number;
  num_score: number;
  rank: number;
  metadata: any;
  updated_at: number;
}

export default function Tournaments() {
  const [tournaments, setTournaments] = useState<Tournament[]>([]);
  const [records, setRecords] = useState<TournamentRecord[]>([]);
  const [selectedTournament, setSelectedTournament] = useState<Tournament | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [newTournament, setNewTournament] = useState({
    id: '',
    title: '',
    description: '',
    category: 0,
    sort_order: 'descending',
    max_size: 100,
    max_num_score: 1000000,
    duration: 86400,
  });

  useEffect(() => {
    loadTournaments();
  }, []);

  useEffect(() => {
    if (selectedTournament) {
      loadRecords(selectedTournament.id);
    }
  }, [selectedTournament]);

  const loadTournaments = async () => {
    try {
      setLoading(true);
      const data = await api.get('/api/tournaments');
      setTournaments(data.tournaments || []);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const loadRecords = async (tournamentId: string) => {
    try {
      const data = await api.get(`/api/tournaments/${tournamentId}/records?limit=100`);
      setRecords(data.records || []);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const createTournament = async () => {
    try {
      await api.post('/api/tournaments', newTournament);
      setShowCreate(false);
      setNewTournament({
        id: '',
        title: '',
        description: '',
        category: 0,
        sort_order: 'descending',
        max_size: 100,
        max_num_score: 1000000,
        duration: 86400,
      });
      loadTournaments();
    } catch (err: any) {
      alert('Failed to create: ' + err.message);
    }
  };

  const deleteTournament = async (id: string) => {
    if (!confirm('Delete this tournament?')) return;
    try {
      await api.delete(`/api/tournaments/${id}`);
      setSelectedTournament(null);
      loadTournaments();
    } catch (err: any) {
      alert('Failed to delete: ' + err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  const formatDuration = (secs: number) => {
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
    return `${Math.floor(secs / 86400)}d`;
  };

  const getTournamentStatus = (t: Tournament) => {
    const now = Date.now();
    if (now < t.start_time) return { label: 'Upcoming', color: 'text-yellow-400' };
    if (t.end_time && now > t.end_time) return { label: 'Ended', color: 'text-gray-400' };
    return { label: 'Active', color: 'text-green-400' };
  };

  if (loading && tournaments.length === 0) {
    return <div className="p-6">Loading tournaments...</div>;
  }

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Tournaments</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
        >
          Create Tournament
        </button>
      </div>

      {/* Create Modal */}
      {showCreate && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-800 rounded-lg p-6 w-[500px] max-h-[90vh] overflow-y-auto">
            <h2 className="text-xl font-semibold mb-4">Create Tournament</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">ID</label>
                <input
                  type="text"
                  value={newTournament.id}
                  onChange={(e) => setNewTournament({...newTournament, id: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  placeholder="e.g., weekly_tournament"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Title</label>
                <input
                  type="text"
                  value={newTournament.title}
                  onChange={(e) => setNewTournament({...newTournament, title: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  placeholder="Weekly Tournament"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Description</label>
                <textarea
                  value={newTournament.description}
                  onChange={(e) => setNewTournament({...newTournament, description: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700 h-20"
                  placeholder="Tournament description"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Category</label>
                  <input
                    type="number"
                    value={newTournament.category}
                    onChange={(e) => setNewTournament({...newTournament, category: parseInt(e.target.value) || 0})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Sort Order</label>
                  <select
                    value={newTournament.sort_order}
                    onChange={(e) => setNewTournament({...newTournament, sort_order: e.target.value})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  >
                    <option value="descending">Descending</option>
                    <option value="ascending">Ascending</option>
                  </select>
                </div>
              </div>
              <div className="grid grid-cols-3 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Max Size</label>
                  <input
                    type="number"
                    value={newTournament.max_size}
                    onChange={(e) => setNewTournament({...newTournament, max_size: parseInt(e.target.value) || 100})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Max Scores</label>
                  <input
                    type="number"
                    value={newTournament.max_num_score}
                    onChange={(e) => setNewTournament({...newTournament, max_num_score: parseInt(e.target.value) || 1000000})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Duration (secs)</label>
                  <input
                    type="number"
                    value={newTournament.duration}
                    onChange={(e) => setNewTournament({...newTournament, duration: parseInt(e.target.value) || 86400})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  />
                </div>
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
                onClick={createTournament}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Tournaments List */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            Tournaments ({tournaments.length})
          </div>
          <div className="divide-y divide-gray-700 max-h-[600px] overflow-y-auto">
            {tournaments.length === 0 ? (
              <div className="px-4 py-3 text-gray-400">No tournaments</div>
            ) : (
              tournaments.map((t) => {
                const status = getTournamentStatus(t);
                return (
                  <div
                    key={t.id}
                    className={`px-4 py-3 cursor-pointer hover:bg-gray-700 ${
                      selectedTournament?.id === t.id ? 'bg-gray-700' : ''
                    }`}
                    onClick={() => setSelectedTournament(t)}
                  >
                    <div className="flex justify-between items-center">
                      <span className="font-medium">{t.title || t.id}</span>
                      <span className={`text-sm ${status.color}`}>{status.label}</span>
                    </div>
                    <div className="text-sm text-gray-400 mt-1">
                      {t.size}/{t.max_size} participants - {formatDuration(t.duration)}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Tournament Details & Records */}
        <div className="lg:col-span-2 space-y-6">
          {selectedTournament ? (
            <>
              <div className="bg-gray-800 rounded-lg p-6">
                <div className="flex justify-between items-start mb-4">
                  <div>
                    <h2 className="text-xl font-semibold">{selectedTournament.title || selectedTournament.id}</h2>
                    <span className={`text-sm ${getTournamentStatus(selectedTournament).color}`}>
                      {getTournamentStatus(selectedTournament).label}
                    </span>
                  </div>
                  <button
                    onClick={() => deleteTournament(selectedTournament.id)}
                    className="text-red-400 hover:text-red-300 text-sm"
                  >
                    Delete
                  </button>
                </div>
                <p className="text-gray-300 mb-4">{selectedTournament.description || 'No description'}</p>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <span className="text-gray-400 block">ID</span>
                    <span className="font-mono">{selectedTournament.id}</span>
                  </div>
                  <div>
                    <span className="text-gray-400 block">Category</span>
                    <span>{selectedTournament.category}</span>
                  </div>
                  <div>
                    <span className="text-gray-400 block">Sort</span>
                    <span className="capitalize">{selectedTournament.sort_order}</span>
                  </div>
                  <div>
                    <span className="text-gray-400 block">Duration</span>
                    <span>{formatDuration(selectedTournament.duration)}</span>
                  </div>
                  <div>
                    <span className="text-gray-400 block">Participants</span>
                    <span>{selectedTournament.size}/{selectedTournament.max_size}</span>
                  </div>
                  <div>
                    <span className="text-gray-400 block">Max Scores</span>
                    <span>{selectedTournament.max_num_score.toLocaleString()}</span>
                  </div>
                  <div>
                    <span className="text-gray-400 block">Starts</span>
                    <span>{formatDate(selectedTournament.start_time)}</span>
                  </div>
                  <div>
                    <span className="text-gray-400 block">Ends</span>
                    <span>{selectedTournament.end_time ? formatDate(selectedTournament.end_time) : 'Never'}</span>
                  </div>
                </div>
              </div>

              <div className="bg-gray-800 rounded-lg overflow-hidden">
                <div className="bg-gray-900 px-4 py-3 font-semibold">
                  Leaderboard ({records.length} entries)
                </div>
                <table className="w-full">
                  <thead className="bg-gray-900/50">
                    <tr>
                      <th className="px-4 py-2 text-left">Rank</th>
                      <th className="px-4 py-2 text-left">Player</th>
                      <th className="px-4 py-2 text-right">Score</th>
                      <th className="px-4 py-2 text-right">Submissions</th>
                      <th className="px-4 py-2 text-right">Updated</th>
                    </tr>
                  </thead>
                  <tbody>
                    {records.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="px-4 py-3 text-gray-400 text-center">
                          No entries yet
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
                          <td className="px-4 py-2 text-right text-gray-400">{record.num_score}</td>
                          <td className="px-4 py-2 text-right text-gray-400 text-sm">
                            {formatDate(record.updated_at)}
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </>
          ) : (
            <div className="bg-gray-800 rounded-lg p-6 text-gray-400 text-center">
              Select a tournament to view details
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
