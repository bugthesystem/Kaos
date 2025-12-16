import { useState, useEffect } from 'react';
import { api } from '../api/client';

interface Queue {
  name: string;
  tickets: number;
  players: number;
}

interface Ticket {
  id: string;
  queue: string;
  players: { user_id: string; username: string; skill: number }[];
  created_at: number;
}

export default function Matchmaker() {
  const [queues, setQueues] = useState<Queue[]>([]);
  const [tickets, setTickets] = useState<Ticket[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchUserId, setSearchUserId] = useState('');

  useEffect(() => {
    loadQueues();
  }, []);

  const loadQueues = async () => {
    try {
      setLoading(true);
      const data = await api.get('/api/matchmaker/queues');
      setQueues(data.queues || []);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const searchTickets = async () => {
    if (!searchUserId) return;
    try {
      const data = await api.get(`/api/matchmaker/tickets?user_id=${searchUserId}`);
      setTickets(data.tickets || []);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  if (loading && queues.length === 0) {
    return <div className="p-6">Loading matchmaker...</div>;
  }

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Matchmaker</h1>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Queue Stats */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            Active Queues
          </div>
          <table className="w-full">
            <thead className="bg-gray-900/50">
              <tr>
                <th className="px-4 py-2 text-left">Queue</th>
                <th className="px-4 py-2 text-right">Tickets</th>
                <th className="px-4 py-2 text-right">Players</th>
              </tr>
            </thead>
            <tbody>
              {queues.length === 0 ? (
                <tr>
                  <td colSpan={3} className="px-4 py-3 text-gray-400 text-center">
                    No active queues
                  </td>
                </tr>
              ) : (
                queues.map((queue) => (
                  <tr key={queue.name} className="border-t border-gray-700">
                    <td className="px-4 py-2 font-medium">{queue.name}</td>
                    <td className="px-4 py-2 text-right">{queue.tickets}</td>
                    <td className="px-4 py-2 text-right">{queue.players}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* Ticket Search */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            Search Tickets
          </div>
          <div className="p-4">
            <div className="flex gap-2 mb-4">
              <input
                type="text"
                value={searchUserId}
                onChange={(e) => setSearchUserId(e.target.value)}
                placeholder="Enter User ID"
                className="flex-1 px-3 py-2 bg-gray-900 rounded border border-gray-700"
              />
              <button
                onClick={searchTickets}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
              >
                Search
              </button>
            </div>

            {tickets.length > 0 ? (
              <div className="space-y-4">
                {tickets.map((ticket) => (
                  <div key={ticket.id} className="bg-gray-900 rounded p-4">
                    <div className="flex justify-between items-start mb-2">
                      <div>
                        <span className="font-medium">Queue: </span>
                        <span className="text-blue-400">{ticket.queue}</span>
                      </div>
                      <span className="text-gray-400 text-sm">
                        {formatDate(ticket.created_at)}
                      </span>
                    </div>
                    <div className="text-sm text-gray-400 mb-2">
                      Ticket ID: {ticket.id}
                    </div>
                    <div className="text-sm">
                      <span className="text-gray-400">Players:</span>
                      <ul className="mt-1 space-y-1">
                        {ticket.players.map((player) => (
                          <li key={player.user_id} className="flex justify-between">
                            <span>{player.username}</span>
                            <span className="text-gray-400">Skill: {player.skill}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  </div>
                ))}
              </div>
            ) : searchUserId ? (
              <div className="text-gray-400 text-center">
                No tickets found for this user
              </div>
            ) : (
              <div className="text-gray-400 text-center">
                Enter a User ID to search for their ticket
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Summary Stats */}
      <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-gray-800 rounded-lg p-6 text-center">
          <div className="text-4xl font-bold text-blue-400">
            {queues.length}
          </div>
          <div className="text-gray-400 mt-2">Active Queues</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-6 text-center">
          <div className="text-4xl font-bold text-green-400">
            {queues.reduce((sum, q) => sum + q.tickets, 0)}
          </div>
          <div className="text-gray-400 mt-2">Total Tickets</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-6 text-center">
          <div className="text-4xl font-bold text-purple-400">
            {queues.reduce((sum, q) => sum + q.players, 0)}
          </div>
          <div className="text-gray-400 mt-2">Players Waiting</div>
        </div>
      </div>
    </div>
  );
}
