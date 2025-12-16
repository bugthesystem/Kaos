import { useState, useEffect } from 'react';
import { api } from '../api/client';

interface Player {
  id: string;
  username: string;
  display_name: string | null;
  email: string | null;
  email_verified: boolean;
  devices: string[];
  social_links: { provider: string; provider_id: string; linked_at: number }[];
  created_at: number;
  updated_at: number;
  banned: boolean;
  ban_reason: string | null;
}

export default function Players() {
  const [players, setPlayers] = useState<Player[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedPlayer, setSelectedPlayer] = useState<Player | null>(null);

  useEffect(() => {
    loadPlayers();
  }, [page]);

  const loadPlayers = async () => {
    try {
      setLoading(true);
      const data = await api.get(`/api/players?page=${page}&page_size=20`);
      setPlayers(data.items || []);
      setTotal(data.total || 0);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleBan = async (playerId: string) => {
    const reason = prompt('Ban reason (optional):');
    try {
      await api.post(`/api/players/${playerId}/ban`, { reason });
      loadPlayers();
    } catch (err: any) {
      alert('Failed to ban: ' + err.message);
    }
  };

  const handleUnban = async (playerId: string) => {
    try {
      await api.post(`/api/players/${playerId}/unban`, {});
      loadPlayers();
    } catch (err: any) {
      alert('Failed to unban: ' + err.message);
    }
  };

  const handleDelete = async (playerId: string) => {
    if (!confirm('Are you sure you want to delete this player?')) return;
    try {
      await api.delete(`/api/players/${playerId}`);
      loadPlayers();
      setSelectedPlayer(null);
    } catch (err: any) {
      alert('Failed to delete: ' + err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  if (loading && players.length === 0) {
    return <div className="p-6">Loading players...</div>;
  }

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Players</h1>
        <span className="text-gray-400">{total} total players</span>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Player List */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <table className="w-full">
            <thead className="bg-gray-900">
              <tr>
                <th className="px-4 py-3 text-left">Username</th>
                <th className="px-4 py-3 text-left">Email</th>
                <th className="px-4 py-3 text-left">Status</th>
              </tr>
            </thead>
            <tbody>
              {players.map((player) => (
                <tr
                  key={player.id}
                  className={`border-t border-gray-700 cursor-pointer hover:bg-gray-700 ${
                    selectedPlayer?.id === player.id ? 'bg-gray-700' : ''
                  }`}
                  onClick={() => setSelectedPlayer(player)}
                >
                  <td className="px-4 py-3">{player.username}</td>
                  <td className="px-4 py-3 text-gray-400">
                    {player.email || '-'}
                  </td>
                  <td className="px-4 py-3">
                    {player.banned ? (
                      <span className="text-red-400">Banned</span>
                    ) : (
                      <span className="text-green-400">Active</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {/* Pagination */}
          <div className="px-4 py-3 bg-gray-900 flex justify-between items-center">
            <button
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page === 1}
              className="px-3 py-1 bg-gray-700 rounded disabled:opacity-50"
            >
              Previous
            </button>
            <span className="text-gray-400">Page {page}</span>
            <button
              onClick={() => setPage((p) => p + 1)}
              disabled={players.length < 20}
              className="px-3 py-1 bg-gray-700 rounded disabled:opacity-50"
            >
              Next
            </button>
          </div>
        </div>

        {/* Player Details */}
        {selectedPlayer && (
          <div className="bg-gray-800 rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Player Details</h2>
            <div className="space-y-4">
              <div>
                <label className="text-gray-400 text-sm">ID</label>
                <div className="font-mono text-sm">{selectedPlayer.id}</div>
              </div>
              <div>
                <label className="text-gray-400 text-sm">Username</label>
                <div>{selectedPlayer.username}</div>
              </div>
              <div>
                <label className="text-gray-400 text-sm">Display Name</label>
                <div>{selectedPlayer.display_name || '-'}</div>
              </div>
              <div>
                <label className="text-gray-400 text-sm">Email</label>
                <div>
                  {selectedPlayer.email || '-'}
                  {selectedPlayer.email_verified && (
                    <span className="ml-2 text-green-400">(verified)</span>
                  )}
                </div>
              </div>
              <div>
                <label className="text-gray-400 text-sm">Devices</label>
                <div>
                  {selectedPlayer.devices.length > 0 ? (
                    <ul className="list-disc list-inside">
                      {selectedPlayer.devices.map((d, i) => (
                        <li key={i} className="font-mono text-sm">{d}</li>
                      ))}
                    </ul>
                  ) : '-'}
                </div>
              </div>
              <div>
                <label className="text-gray-400 text-sm">Social Links</label>
                <div>
                  {selectedPlayer.social_links.length > 0 ? (
                    <ul className="list-disc list-inside">
                      {selectedPlayer.social_links.map((l, i) => (
                        <li key={i}>{l.provider}: {l.provider_id}</li>
                      ))}
                    </ul>
                  ) : '-'}
                </div>
              </div>
              <div>
                <label className="text-gray-400 text-sm">Created</label>
                <div>{formatDate(selectedPlayer.created_at)}</div>
              </div>
              <div>
                <label className="text-gray-400 text-sm">Status</label>
                <div>
                  {selectedPlayer.banned ? (
                    <span className="text-red-400">
                      Banned{selectedPlayer.ban_reason && `: ${selectedPlayer.ban_reason}`}
                    </span>
                  ) : (
                    <span className="text-green-400">Active</span>
                  )}
                </div>
              </div>

              <div className="flex gap-2 mt-6">
                {selectedPlayer.banned ? (
                  <button
                    onClick={() => handleUnban(selectedPlayer.id)}
                    className="px-4 py-2 bg-green-600 hover:bg-green-700 rounded"
                  >
                    Unban
                  </button>
                ) : (
                  <button
                    onClick={() => handleBan(selectedPlayer.id)}
                    className="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 rounded"
                  >
                    Ban
                  </button>
                )}
                <button
                  onClick={() => handleDelete(selectedPlayer.id)}
                  className="px-4 py-2 bg-red-600 hover:bg-red-700 rounded"
                >
                  Delete
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
