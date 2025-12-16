import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { RoomInfo } from '../api/types';

export function RoomsPage() {
  const [rooms, setRooms] = useState<RoomInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selectedRoom, setSelectedRoom] = useState<string | null>(null);

  useEffect(() => {
    loadRooms();
  }, []);

  const loadRooms = async () => {
    setLoading(true);
    try {
      const data = await api.listRooms(1, 100);
      setRooms(data.items);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load rooms');
    } finally {
      setLoading(false);
    }
  };

  const handleTerminate = async (id: string) => {
    if (!confirm('Are you sure you want to terminate this room?')) return;
    try {
      await api.terminateRoom(id);
      loadRooms();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to terminate room');
    }
  };

  const getStateBadge = (state: string) => {
    switch (state) {
      case 'open':
        return <span className="badge badge-success">Open</span>;
      case 'running':
        return <span className="badge badge-info">Running</span>;
      case 'closed':
        return <span className="badge badge-danger">Closed</span>;
      default:
        return <span className="badge badge-warning">{state}</span>;
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Rooms</h1>
          <p className="text-gray-400">Active game rooms and matches</p>
        </div>
        <button onClick={loadRooms} className="btn btn-secondary">
          Refresh
        </button>
      </div>

      {error && (
        <div className="bg-red-900/50 border border-red-700 text-red-300 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Rooms List */}
        <div className="card p-0 overflow-hidden">
          <div className="px-4 py-3 border-b border-gray-700">
            <h2 className="font-semibold text-white">Room List</h2>
          </div>
          <div className="divide-y divide-gray-700">
            {loading ? (
              <div className="p-4 text-gray-400">Loading...</div>
            ) : rooms.length === 0 ? (
              <div className="p-4 text-gray-400">No rooms found</div>
            ) : (
              rooms.map((room) => (
                <div
                  key={room.id}
                  className={`p-4 hover:bg-gray-700/50 cursor-pointer ${selectedRoom === room.id ? 'bg-gray-700/50' : ''}`}
                  onClick={() => setSelectedRoom(room.id)}
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="font-medium text-white">
                        {room.label || room.id.slice(0, 8)}
                      </p>
                      <p className="text-xs text-gray-400 font-mono">{room.id}</p>
                    </div>
                    {getStateBadge(room.state)}
                  </div>
                  <div className="mt-2 flex items-center gap-4 text-sm text-gray-400">
                    <span>👥 {room.player_count}/{room.max_players}</span>
                    <span>⚡ {room.tick_rate} Hz</span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Room Details */}
        <div className="card">
          <h2 className="font-semibold text-white mb-4">Room Details</h2>
          {selectedRoom ? (
            <RoomDetails roomId={selectedRoom} onTerminate={handleTerminate} />
          ) : (
            <p className="text-gray-400">Select a room to view details</p>
          )}
        </div>
      </div>
    </div>
  );
}

function RoomDetails({ roomId, onTerminate }: { roomId: string; onTerminate: (id: string) => void }) {
  const [room, setRoom] = useState<RoomInfo | null>(null);
  const [players, setPlayers] = useState<number[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadRoom();
  }, [roomId]);

  const loadRoom = async () => {
    setLoading(true);
    try {
      const [roomData, playersData] = await Promise.all([
        api.getRoom(roomId),
        api.getRoomPlayers(roomId),
      ]);
      setRoom(roomData);
      setPlayers(playersData.players);
    } catch {
      // Room might have been terminated
      setRoom(null);
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return <p className="text-gray-400">Loading...</p>;
  }

  if (!room) {
    return <p className="text-gray-400">Room not found</p>;
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <p className="text-gray-400">Room ID</p>
          <p className="text-white font-mono text-xs">{room.id}</p>
        </div>
        <div>
          <p className="text-gray-400">Label</p>
          <p className="text-white">{room.label || '-'}</p>
        </div>
        <div>
          <p className="text-gray-400">State</p>
          <p className="text-white capitalize">{room.state}</p>
        </div>
        <div>
          <p className="text-gray-400">Tick Rate</p>
          <p className="text-white">{room.tick_rate} Hz</p>
        </div>
        <div>
          <p className="text-gray-400">Players</p>
          <p className="text-white">{room.player_count} / {room.max_players}</p>
        </div>
      </div>

      {players.length > 0 && (
        <div>
          <p className="text-gray-400 mb-2">Connected Players</p>
          <div className="flex flex-wrap gap-2">
            {players.map((id) => (
              <span key={id} className="badge badge-info">Session #{id}</span>
            ))}
          </div>
        </div>
      )}

      <div className="pt-4 border-t border-gray-700">
        <button
          onClick={() => onTerminate(room.id)}
          className="btn btn-danger"
        >
          Terminate Room
        </button>
      </div>
    </div>
  );
}
