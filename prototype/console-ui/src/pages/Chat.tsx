import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';

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

function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString();
}

function formatRelativeTime(ts: number): string {
  const seconds = Math.floor((Date.now() - ts) / 1000);
  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

export default function Chat() {
  const [channels, setChannels] = useState<Channel[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [newMessage, setNewMessage] = useState('');
  const [sendingMessage, setSendingMessage] = useState(false);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newChannelName, setNewChannelName] = useState('');
  const [newChannelType, setNewChannelType] = useState<'room' | 'group'>('room');
  const [creatingChannel, setCreatingChannel] = useState(false);

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
      const data = await api.get<{ items: Channel[]; total: number }>('/api/chat/channels');
      // Map from backend format (name) to frontend (label)
      const mapped = (data.items || []).map(c => ({
        ...c,
        label: c.label || (c as { name?: string }).name || c.id,
      }));
      setChannels(mapped);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load channels');
    } finally {
      setLoading(false);
    }
  };

  const loadMessages = async (channelId: string) => {
    try {
      const data = await api.get(`/api/chat/channels/${channelId}/messages?limit=100`);
      setMessages(data.messages || []);
    } catch (err) {
      console.error('Failed to load messages:', err);
      setMessages([]);
    }
  };

  const handleRowClick = (channel: Channel) => {
    setSelectedChannel(channel);
    setDrawerOpen(true);
  };

  const handleDeleteChannel = async () => {
    if (!selectedChannel) return;
    if (!confirm('Are you sure you want to delete this channel and all messages?')) return;
    try {
      await api.delete(`/api/chat/channels/${selectedChannel.id}`);
      setDrawerOpen(false);
      setSelectedChannel(null);
      loadChannels();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete channel');
    }
  };

  const handleSendMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedChannel || !newMessage.trim()) return;

    setSendingMessage(true);
    try {
      await api.post(`/api/chat/channels/${selectedChannel.id}/send`, {
        content: newMessage.trim(),
        code: 100, // System message code
      });
      setNewMessage('');
      // Reload messages to show the new one
      await loadMessages(selectedChannel.id);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to send message');
    } finally {
      setSendingMessage(false);
    }
  };

  const handleCreateChannel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newChannelName.trim()) return;

    setCreatingChannel(true);
    try {
      await api.post('/api/chat/channels', {
        name: newChannelName.trim(),
        channel_type: newChannelType,
      });
      setNewChannelName('');
      setNewChannelType('room');
      setShowCreateModal(false);
      await loadChannels();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to create channel');
    } finally {
      setCreatingChannel(false);
    }
  };

  const getChannelTypeVariant = (type: string): 'info' | 'success' | 'warning' | 'danger' => {
    switch (type) {
      case 'room': return 'info';
      case 'group': return 'success';
      case 'dm': return 'warning';
      default: return 'info';
    }
  };

  const channelTypeCounts = {
    room: channels.filter(c => c.channel_type === 'room').length,
    group: channels.filter(c => c.channel_type === 'group').length,
    dm: channels.filter(c => c.channel_type === 'dm').length,
  };

  const totalMembers = channels.reduce((sum, c) => sum + c.member_count, 0);

  const columns: Column<Channel>[] = [
    {
      key: 'label',
      header: 'Channel',
      render: (channel) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: channel.channel_type === 'room'
                ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
                : channel.channel_type === 'group'
                ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
              color: 'white',
            }}
          >
            <ChatIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {channel.label || channel.id.slice(0, 12)}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {channel.id.slice(0, 16)}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'channel_type',
      header: 'Type',
      width: '100px',
      render: (channel) => (
        <Badge variant={getChannelTypeVariant(channel.channel_type)}>
          {channel.channel_type.charAt(0).toUpperCase() + channel.channel_type.slice(1)}
        </Badge>
      ),
    },
    {
      key: 'member_count',
      header: 'Members',
      width: '100px',
      render: (channel) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {channel.member_count}
        </span>
      ),
    },
    {
      key: 'created_at',
      header: 'Created',
      width: '140px',
      render: (channel) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(channel.created_at)}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Chat</h1>
          <p className="page-subtitle">
            Channels and messages
          </p>
        </div>
        <div className="flex gap-2">
          <button onClick={() => setShowCreateModal(true)} className="btn btn-primary">
            + Create Channel
          </button>
          <button onClick={loadChannels} className="btn btn-secondary">
            Refresh
          </button>
        </div>
      </div>

      {/* Create Channel Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="card p-6 w-full max-w-md" style={{ background: 'var(--bg-secondary)' }}>
            <h2 className="text-xl font-semibold mb-4" style={{ color: 'var(--text-primary)' }}>
              Create Channel
            </h2>
            <form onSubmit={handleCreateChannel} className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-1" style={{ color: 'var(--text-secondary)' }}>
                  Channel Name
                </label>
                <input
                  type="text"
                  value={newChannelName}
                  onChange={(e) => setNewChannelName(e.target.value)}
                  placeholder="e.g., general, lobby, announcements"
                  className="input w-full"
                  autoFocus
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1" style={{ color: 'var(--text-secondary)' }}>
                  Channel Type
                </label>
                <select
                  value={newChannelType}
                  onChange={(e) => setNewChannelType(e.target.value as 'room' | 'group')}
                  className="input w-full"
                >
                  <option value="room">Room (Public chat room)</option>
                  <option value="group">Group (Private group chat)</option>
                </select>
              </div>
              <div className="flex gap-2 justify-end">
                <button
                  type="button"
                  onClick={() => setShowCreateModal(false)}
                  className="btn btn-secondary"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={creatingChannel || !newChannelName.trim()}
                >
                  {creatingChannel ? 'Creating...' : 'Create'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {error && (
        <div className="alert alert-danger">
          {error}
        </div>
      )}

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="stat-card">
          <div className="stat-icon">
            <ChatIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{channels.length}</span>
          <span className="stat-label">Total Channels</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <RoomIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{channelTypeCounts.room}</span>
          <span className="stat-label">Room Channels</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <GroupIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{channelTypeCounts.group}</span>
          <span className="stat-label">Group Channels</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <MembersIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{totalMembers}</span>
          <span className="stat-label">Total Members</span>
        </div>
      </div>

      {/* Channels Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={channels}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedChannel?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search channels..."
          searchFields={['label', 'id', 'channel_type']}
          pagination
          pageSize={15}
          emptyMessage="No channels found"
        />
      </div>

      {/* Channel Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Channel Details"
        width="lg"
        footer={
          selectedChannel && (
            <button onClick={handleDeleteChannel} className="btn btn-danger flex-1">
              Delete Channel
            </button>
          )
        }
      >
        {selectedChannel && (
          <div className="space-y-6">
            {/* Channel Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedChannel.channel_type === 'room'
                    ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)'
                    : selectedChannel.channel_type === 'group'
                    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                    : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
                  color: 'white',
                }}
              >
                <ChatIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedChannel.label || selectedChannel.id.slice(0, 16)}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={getChannelTypeVariant(selectedChannel.channel_type)}>
                    {selectedChannel.channel_type.charAt(0).toUpperCase() + selectedChannel.channel_type.slice(1)}
                  </Badge>
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-2 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedChannel.member_count}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Members</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {messages.length}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Messages</div>
              </div>
            </div>

            <Section title="Channel Information">
              <Field label="Channel ID" mono>
                {selectedChannel.id}
              </Field>
              <Field label="Label">
                {selectedChannel.label || '-'}
              </Field>
              <Field label="Type">
                {selectedChannel.channel_type}
              </Field>
              <Field label="Created At">
                {formatTimestamp(selectedChannel.created_at)}
              </Field>
            </Section>

            <Section title="Send System Message">
              <form onSubmit={handleSendMessage} className="flex gap-2">
                <input
                  type="text"
                  value={newMessage}
                  onChange={(e) => setNewMessage(e.target.value)}
                  placeholder="Type a system message..."
                  className="input flex-1"
                  disabled={sendingMessage}
                />
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={sendingMessage || !newMessage.trim()}
                >
                  {sendingMessage ? 'Sending...' : 'Send'}
                </button>
              </form>
              <p className="text-xs mt-2" style={{ color: 'var(--text-muted)' }}>
                Messages sent from the console appear as system messages
              </p>
            </Section>

            <Section title="Recent Messages">
              {messages.length > 0 ? (
                <div className="space-y-3 max-h-80 overflow-y-auto">
                  {messages.slice(0, 20).map((msg) => (
                    <div
                      key={msg.id}
                      className="p-3 rounded-lg"
                      style={{ background: 'var(--bg-tertiary)' }}
                    >
                      <div className="flex items-baseline gap-2 mb-1">
                        <span className="font-semibold" style={{ color: msg.sender_username === 'System' ? 'var(--color-warning)' : 'var(--color-accent)' }}>
                          {msg.sender_username}
                        </span>
                        <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
                          {formatRelativeTime(msg.created_at)}
                        </span>
                      </div>
                      <p style={{ color: 'var(--text-secondary)' }}>{msg.content}</p>
                    </div>
                  ))}
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)' }}>No messages in this channel</p>
              )}
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}

// Icons
function ChatIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
    </svg>
  );
}

function RoomIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
  );
}

function GroupIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}

function MembersIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}
