import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { useConfirm } from '../components/ConfirmDialog';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { ChatIcon, RoomsIcon, GroupIcon, UsersIcon, RefreshIcon } from '../components/icons';
import { formatTimestamp, formatRelativeTime } from '../utils/formatters';

interface Channel { id: string; label: string; channel_type: string; member_count: number; created_at: number; }
interface Message { id: string; sender_id: string; sender_username: string; content: string; created_at: number; }

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
  const { confirm, ConfirmDialog } = useConfirm();

  useEffect(() => { loadChannels(); }, []);
  useEffect(() => { if (selectedChannel) loadMessages(selectedChannel.id); }, [selectedChannel]);

  const loadChannels = async () => {
    try {
      setLoading(true);
      const data = await api.get<{ items: Channel[]; total: number }>('/api/chat/channels');
      const mapped = (data.items || []).map(c => ({ ...c, label: c.label || (c as { name?: string }).name || c.id }));
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

  const handleRowClick = (channel: Channel) => { setSelectedChannel(channel); setDrawerOpen(true); };

  const handleDeleteChannel = async () => {
    if (!selectedChannel) return;
    const confirmed = await confirm({ title: 'Delete Channel', message: 'Are you sure you want to delete this channel and all messages?', confirmLabel: 'Delete', variant: 'danger' });
    if (!confirmed) return;
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
      await api.post(`/api/chat/channels/${selectedChannel.id}/send`, { content: newMessage.trim(), code: 100 });
      setNewMessage('');
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
      await api.post('/api/chat/channels', { name: newChannelName.trim(), channel_type: newChannelType });
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

  const getChannelTypeVariant = (type: string): 'info' | 'success' | 'warning' => type === 'room' ? 'info' : type === 'group' ? 'success' : 'warning';
  const channelTypeCounts = { room: channels.filter(c => c.channel_type === 'room').length, group: channels.filter(c => c.channel_type === 'group').length, dm: channels.filter(c => c.channel_type === 'dm').length };
  const totalMembers = channels.reduce((sum, c) => sum + c.member_count, 0);

  const columns: Column<Channel>[] = [
    {
      key: 'label', header: 'Channel',
      render: (channel) => (
        <div className="flex items-center gap-3">
          <ChannelAvatar channel={channel} size="sm" />
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>{channel.label || channel.id.slice(0, 12)}</div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>{channel.id.slice(0, 16)}...</div>
          </div>
        </div>
      ),
    },
    { key: 'channel_type', header: 'Type', width: '100px', render: (channel) => <Badge variant={getChannelTypeVariant(channel.channel_type)}>{channel.channel_type.charAt(0).toUpperCase() + channel.channel_type.slice(1)}</Badge> },
    { key: 'member_count', header: 'Members', width: '100px', render: (channel) => <span style={{ color: 'var(--text-secondary)' }}>{channel.member_count}</span> },
    { key: 'created_at', header: 'Created', width: '140px', render: (channel) => <span style={{ color: 'var(--text-muted)' }}>{formatRelativeTime(channel.created_at)}</span> },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {ConfirmDialog}
      <PageHeader title="Chat" subtitle="Channels and messages">
        <button onClick={() => setShowCreateModal(true)} className="btn btn-primary">+ Create Channel</button>
        <button onClick={loadChannels} className="btn btn-secondary"><RefreshIcon className="w-4 h-4" /></button>
      </PageHeader>

      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="card p-6 w-full max-w-md" style={{ background: 'var(--bg-secondary)' }}>
            <h2 className="text-xl font-semibold mb-4" style={{ color: 'var(--text-primary)' }}>Create Channel</h2>
            <form onSubmit={handleCreateChannel} className="space-y-4">
              <div><label className="block text-sm font-medium mb-1" style={{ color: 'var(--text-secondary)' }}>Channel Name</label><input type="text" value={newChannelName} onChange={(e) => setNewChannelName(e.target.value)} placeholder="e.g., general, lobby" className="input w-full" autoFocus /></div>
              <div><label className="block text-sm font-medium mb-1" style={{ color: 'var(--text-secondary)' }}>Channel Type</label><select value={newChannelType} onChange={(e) => setNewChannelType(e.target.value as 'room' | 'group')} className="input w-full"><option value="room">Room (Public chat room)</option><option value="group">Group (Private group chat)</option></select></div>
              <div className="flex gap-2 justify-end"><button type="button" onClick={() => setShowCreateModal(false)} className="btn btn-secondary">Cancel</button><button type="submit" className="btn btn-primary" disabled={creatingChannel || !newChannelName.trim()}>{creatingChannel ? 'Creating...' : 'Create'}</button></div>
            </form>
          </div>
        </div>
      )}

      {error && <Alert variant="danger" onDismiss={() => setError('')}>{error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<ChatIcon className="w-5 h-5" />} label="Total Channels" value={channels.length} color="primary" />
        <StatCard icon={<RoomsIcon className="w-5 h-5" />} label="Room Channels" value={channelTypeCounts.room} color="info" />
        <StatCard icon={<GroupIcon className="w-5 h-5" />} label="Group Channels" value={channelTypeCounts.group} color="success" />
        <StatCard icon={<UsersIcon className="w-5 h-5" />} label="Total Members" value={totalMembers} color="warning" />
      </StatGrid>

      <div className="card p-0 overflow-hidden">
        <DataTable data={channels} columns={columns} keyField="id" onRowClick={handleRowClick} selectedId={selectedChannel?.id} loading={loading} searchable searchPlaceholder="Search channels..." searchFields={['label', 'id', 'channel_type']} pagination pageSize={15} emptyMessage="No channels found" />
      </div>

      <Drawer open={drawerOpen} onClose={() => setDrawerOpen(false)} title="Channel Details" width="lg" footer={selectedChannel && <button onClick={handleDeleteChannel} className="btn btn-danger flex-1">Delete Channel</button>}>
        {selectedChannel && (
          <div className="space-y-6">
            <div className="flex items-center gap-4">
              <ChannelAvatar channel={selectedChannel} size="lg" />
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>{selectedChannel.label || selectedChannel.id.slice(0, 16)}</h2>
                <Badge variant={getChannelTypeVariant(selectedChannel.channel_type)}>{selectedChannel.channel_type.charAt(0).toUpperCase() + selectedChannel.channel_type.slice(1)}</Badge>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}><div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{selectedChannel.member_count}</div><div className="text-xs" style={{ color: 'var(--text-muted)' }}>Members</div></div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}><div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>{messages.length}</div><div className="text-xs" style={{ color: 'var(--text-muted)' }}>Messages</div></div>
            </div>
            <Section title="Channel Information">
              <Field label="Channel ID" mono>{selectedChannel.id}</Field>
              <Field label="Label">{selectedChannel.label || '-'}</Field>
              <Field label="Type">{selectedChannel.channel_type}</Field>
              <Field label="Created At">{formatTimestamp(selectedChannel.created_at)}</Field>
            </Section>
            <Section title="Send System Message">
              <form onSubmit={handleSendMessage} className="flex gap-2">
                <input type="text" value={newMessage} onChange={(e) => setNewMessage(e.target.value)} placeholder="Type a system message..." className="input flex-1" disabled={sendingMessage} />
                <button type="submit" className="btn btn-primary" disabled={sendingMessage || !newMessage.trim()}>{sendingMessage ? 'Sending...' : 'Send'}</button>
              </form>
              <p className="text-xs mt-2" style={{ color: 'var(--text-muted)' }}>Messages sent from the console appear as system messages</p>
            </Section>
            <Section title="Recent Messages">
              {messages.length > 0 ? (
                <div className="space-y-3 max-h-80 overflow-y-auto">
                  {messages.slice(0, 20).map((msg) => (
                    <div key={msg.id} className="p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                      <div className="flex items-baseline gap-2 mb-1">
                        <span className="font-semibold" style={{ color: msg.sender_username === 'System' ? 'var(--color-warning)' : 'var(--color-accent)' }}>{msg.sender_username}</span>
                        <span className="text-xs" style={{ color: 'var(--text-muted)' }}>{formatRelativeTime(msg.created_at)}</span>
                      </div>
                      <p style={{ color: 'var(--text-secondary)' }}>{msg.content}</p>
                    </div>
                  ))}
                </div>
              ) : <p style={{ color: 'var(--text-muted)' }}>No messages in this channel</p>}
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}

function ChannelAvatar({ channel, size = 'sm' }: { channel: Channel; size?: 'sm' | 'lg' }) {
  const sizeClasses = size === 'lg' ? 'w-16 h-16' : 'w-9 h-9';
  const iconSize = size === 'lg' ? 'w-8 h-8' : 'w-5 h-5';
  const bg = channel.channel_type === 'room' ? 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)' : channel.channel_type === 'group' ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)' : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)';
  return <div className={`${sizeClasses} rounded-${size === 'lg' ? 'xl' : 'lg'} flex items-center justify-center`} style={{ background: bg, color: 'white' }}><ChatIcon className={iconSize} /></div>;
}
