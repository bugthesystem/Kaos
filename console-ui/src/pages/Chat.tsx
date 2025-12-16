import { useState, useEffect } from 'react';
import { api } from '../api/client';

interface Channel {
  id: string;
  label: string;
  channel_type: string;
  member_count: number;
  created_at: number;
}

interface Message {
  id: string;
  sender_id: string;
  sender_username: string;
  content: string;
  created_at: number;
}

export default function Chat() {
  const [channels, setChannels] = useState<Channel[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadChannels();
  }, []);

  useEffect(() => {
    if (selectedChannel) {
      loadMessages(selectedChannel.id);
    }
  }, [selectedChannel]);

  const loadChannels = async () => {
    try {
      setLoading(true);
      const data = await api.get('/api/chat/channels');
      setChannels(data.channels || []);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const loadMessages = async (channelId: string) => {
    try {
      const data = await api.get(`/api/chat/channels/${channelId}/messages?limit=100`);
      setMessages(data.messages || []);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  const getChannelTypeColor = (type: string) => {
    switch (type) {
      case 'room': return 'text-blue-400';
      case 'group': return 'text-green-400';
      case 'dm': return 'text-purple-400';
      default: return 'text-gray-400';
    }
  };

  if (loading && channels.length === 0) {
    return <div className="p-6">Loading chat channels...</div>;
  }

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Chat</h1>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Channel List */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            Channels ({channels.length})
          </div>
          <div className="divide-y divide-gray-700">
            {channels.length === 0 ? (
              <div className="px-4 py-3 text-gray-400">No active channels</div>
            ) : (
              channels.map((channel) => (
                <div
                  key={channel.id}
                  className={`px-4 py-3 cursor-pointer hover:bg-gray-700 ${
                    selectedChannel?.id === channel.id ? 'bg-gray-700' : ''
                  }`}
                  onClick={() => setSelectedChannel(channel)}
                >
                  <div className="flex justify-between items-center">
                    <span className="font-medium">{channel.label || channel.id}</span>
                    <span className={`text-sm ${getChannelTypeColor(channel.channel_type)}`}>
                      {channel.channel_type}
                    </span>
                  </div>
                  <div className="text-sm text-gray-400 mt-1">
                    {channel.member_count} members
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Messages */}
        <div className="lg:col-span-2 bg-gray-800 rounded-lg overflow-hidden flex flex-col h-[600px]">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            {selectedChannel ? (
              <>Messages - {selectedChannel.label || selectedChannel.id}</>
            ) : (
              'Select a channel'
            )}
          </div>
          <div className="flex-1 overflow-y-auto p-4 space-y-4">
            {selectedChannel ? (
              messages.length === 0 ? (
                <div className="text-gray-400 text-center">No messages</div>
              ) : (
                messages.map((msg) => (
                  <div key={msg.id} className="flex flex-col">
                    <div className="flex items-baseline gap-2">
                      <span className="font-semibold text-blue-400">
                        {msg.sender_username}
                      </span>
                      <span className="text-xs text-gray-500">
                        {formatDate(msg.created_at)}
                      </span>
                    </div>
                    <div className="mt-1 text-gray-200">{msg.content}</div>
                  </div>
                ))
              )
            ) : (
              <div className="text-gray-400 text-center">
                Select a channel to view messages
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
