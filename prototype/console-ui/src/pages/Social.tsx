import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';

interface Friend {
  user_id: string;
  username: string;
  state: number;
  updated_at: number;
}

interface Group {
  id: string;
  name: string;
  description: string;
  creator_id: string;
  open: boolean;
  member_count: number;
  max_count: number;
  created_at: number;
}

interface GroupMember {
  user_id: string;
  username: string;
  state: number;
  joined_at: number;
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

export default function Social() {
  const [activeTab, setActiveTab] = useState<'friends' | 'groups'>('groups');
  const [groups, setGroups] = useState<Group[]>([]);
  const [friends, setFriends] = useState<Friend[]>([]);
  const [members, setMembers] = useState<GroupMember[]>([]);
  const [selectedGroup, setSelectedGroup] = useState<Group | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchUserId, setSearchUserId] = useState('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newGroup, setNewGroup] = useState({ name: '', description: '', open: true, max_count: 100 });

  useEffect(() => {
    if (activeTab === 'groups') {
      loadGroups();
    }
  }, [activeTab]);

  useEffect(() => {
    if (selectedGroup) {
      loadMembers(selectedGroup.id);
    }
  }, [selectedGroup]);

  const loadGroups = async () => {
    try {
      setLoading(true);
      const data = await api.get('/api/social/groups');
      setGroups(data.groups || []);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load groups');
    } finally {
      setLoading(false);
    }
  };

  const loadMembers = async (groupId: string) => {
    try {
      const data = await api.get(`/api/social/groups/${groupId}/members`);
      setMembers(data.members || []);
    } catch (err) {
      console.error('Failed to load members:', err);
      setMembers([]);
    }
  };

  const searchFriends = async () => {
    if (!searchUserId.trim()) return;
    try {
      setLoading(true);
      const data = await api.get(`/api/social/friends?user_id=${searchUserId}`);
      setFriends(data.friends || []);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to search friends');
      setFriends([]);
    } finally {
      setLoading(false);
    }
  };

  const createGroup = async () => {
    if (!newGroup.name.trim()) return;
    try {
      await api.post('/api/social/groups', newGroup);
      setShowCreateModal(false);
      setNewGroup({ name: '', description: '', open: true, max_count: 100 });
      loadGroups();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to create group');
    }
  };

  const deleteGroup = async () => {
    if (!selectedGroup) return;
    if (!confirm('Are you sure you want to delete this group?')) return;
    try {
      await api.delete(`/api/social/groups/${selectedGroup.id}`);
      setDrawerOpen(false);
      setSelectedGroup(null);
      loadGroups();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete group');
    }
  };

  const handleRowClick = (group: Group) => {
    setSelectedGroup(group);
    setDrawerOpen(true);
  };

  const getFriendStateInfo = (state: number): { label: string; variant: 'success' | 'warning' | 'info' | 'danger' } => {
    switch (state) {
      case 0: return { label: 'Friend', variant: 'success' };
      case 1: return { label: 'Pending Sent', variant: 'warning' };
      case 2: return { label: 'Pending Received', variant: 'info' };
      case 3: return { label: 'Blocked', variant: 'danger' };
      default: return { label: 'Unknown', variant: 'info' };
    }
  };

  const getMemberStateInfo = (state: number): { label: string; variant: 'success' | 'warning' | 'info' | 'danger' } => {
    switch (state) {
      case 0: return { label: 'Superadmin', variant: 'danger' };
      case 1: return { label: 'Admin', variant: 'info' };
      case 2: return { label: 'Member', variant: 'success' };
      case 3: return { label: 'Join Request', variant: 'warning' };
      default: return { label: 'Unknown', variant: 'info' };
    }
  };

  const openGroups = groups.filter(g => g.open).length;
  const closedGroups = groups.filter(g => !g.open).length;
  const totalMembers = groups.reduce((sum, g) => sum + g.member_count, 0);

  const groupColumns: Column<Group>[] = [
    {
      key: 'name',
      header: 'Group',
      render: (group) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: group.open
                ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
              color: 'white',
            }}
          >
            <GroupIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {group.name}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {group.id.slice(0, 16)}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'open',
      header: 'Access',
      width: '100px',
      render: (group) => (
        <Badge variant={group.open ? 'success' : 'warning'}>
          {group.open ? 'Open' : 'Closed'}
        </Badge>
      ),
    },
    {
      key: 'member_count',
      header: 'Members',
      width: '120px',
      render: (group) => (
        <span style={{ color: 'var(--text-secondary)' }}>
          {group.member_count} / {group.max_count}
        </span>
      ),
    },
    {
      key: 'created_at',
      header: 'Created',
      width: '140px',
      render: (group) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(group.created_at)}
        </span>
      ),
    },
  ];

  const friendColumns: Column<Friend>[] = [
    {
      key: 'username',
      header: 'Friend',
      render: (friend) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
              color: 'white',
            }}
          >
            <FriendIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {friend.username}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {friend.user_id.slice(0, 16)}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'state',
      header: 'Status',
      width: '140px',
      render: (friend) => {
        const info = getFriendStateInfo(friend.state);
        return <Badge variant={info.variant}>{info.label}</Badge>;
      },
    },
    {
      key: 'updated_at',
      header: 'Updated',
      width: '140px',
      render: (friend) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(friend.updated_at)}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Social</h1>
          <p className="page-subtitle">
            Groups and friends management
          </p>
        </div>
        <div className="flex gap-2">
          {activeTab === 'groups' && (
            <button onClick={() => setShowCreateModal(true)} className="btn btn-primary">
              Create Group
            </button>
          )}
          <button onClick={loadGroups} className="btn btn-secondary">
            Refresh
          </button>
        </div>
      </div>

      {error && (
        <div className="alert alert-danger">
          {error}
        </div>
      )}

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="stat-card">
          <div className="stat-icon">
            <GroupIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{groups.length}</span>
          <span className="stat-label">Total Groups</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <OpenIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{openGroups}</span>
          <span className="stat-label">Open Groups</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ClosedIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{closedGroups}</span>
          <span className="stat-label">Closed Groups</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <MembersIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{totalMembers}</span>
          <span className="stat-label">Total Members</span>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2" style={{ borderBottom: '1px solid var(--border-primary)', paddingBottom: '0.5rem' }}>
        <button
          onClick={() => setActiveTab('groups')}
          className={`px-4 py-2 rounded-t font-medium transition-colors ${
            activeTab === 'groups'
              ? 'bg-[var(--color-accent)] text-white'
              : 'hover:bg-[var(--bg-tertiary)]'
          }`}
          style={{ color: activeTab === 'groups' ? 'white' : 'var(--text-secondary)' }}
        >
          Groups
        </button>
        <button
          onClick={() => setActiveTab('friends')}
          className={`px-4 py-2 rounded-t font-medium transition-colors ${
            activeTab === 'friends'
              ? 'bg-[var(--color-accent)] text-white'
              : 'hover:bg-[var(--bg-tertiary)]'
          }`}
          style={{ color: activeTab === 'friends' ? 'white' : 'var(--text-secondary)' }}
        >
          Friends
        </button>
      </div>

      {activeTab === 'groups' ? (
        <div className="card p-0 overflow-hidden">
          <DataTable
            data={groups}
            columns={groupColumns}
            keyField="id"
            onRowClick={handleRowClick}
            selectedId={selectedGroup?.id}
            loading={loading}
            searchable
            searchPlaceholder="Search groups..."
            searchFields={['name', 'id', 'description']}
            pagination
            pageSize={15}
            emptyMessage="No groups found"
          />
        </div>
      ) : (
        <div className="space-y-4">
          {/* Friends Search */}
          <div className="card">
            <h3 className="font-semibold mb-4" style={{ color: 'var(--text-primary)' }}>Search User's Friends</h3>
            <div className="flex gap-2">
              <input
                type="text"
                value={searchUserId}
                onChange={(e) => setSearchUserId(e.target.value)}
                placeholder="Enter User ID"
                className="form-input flex-1"
                onKeyDown={(e) => e.key === 'Enter' && searchFriends()}
              />
              <button onClick={searchFriends} className="btn btn-primary">
                Search
              </button>
            </div>
          </div>

          {/* Friends Table */}
          <div className="card p-0 overflow-hidden">
            <DataTable
              data={friends}
              columns={friendColumns}
              keyField="user_id"
              loading={loading}
              emptyMessage={searchUserId ? "No friends found for this user" : "Enter a User ID to search for friends"}
            />
          </div>
        </div>
      )}

      {/* Create Group Modal */}
      {showCreateModal && (
        <div className="modal-overlay">
          <div className="modal">
            <h2 className="modal-title">Create Group</h2>
            <div className="space-y-4">
              <div>
                <label className="form-label">Name</label>
                <input
                  type="text"
                  value={newGroup.name}
                  onChange={(e) => setNewGroup({ ...newGroup, name: e.target.value })}
                  className="form-input"
                  placeholder="Group name"
                />
              </div>
              <div>
                <label className="form-label">Description</label>
                <textarea
                  value={newGroup.description}
                  onChange={(e) => setNewGroup({ ...newGroup, description: e.target.value })}
                  className="form-input"
                  rows={3}
                  placeholder="Group description"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="form-label">Access</label>
                  <select
                    value={newGroup.open ? 'true' : 'false'}
                    onChange={(e) => setNewGroup({ ...newGroup, open: e.target.value === 'true' })}
                    className="form-input"
                  >
                    <option value="true">Open (anyone can join)</option>
                    <option value="false">Closed (invite only)</option>
                  </select>
                </div>
                <div>
                  <label className="form-label">Max Members</label>
                  <input
                    type="number"
                    value={newGroup.max_count}
                    onChange={(e) => setNewGroup({ ...newGroup, max_count: parseInt(e.target.value) || 100 })}
                    className="form-input"
                    min={1}
                  />
                </div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowCreateModal(false)} className="btn btn-secondary">
                Cancel
              </button>
              <button onClick={createGroup} className="btn btn-primary">
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Group Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Group Details"
        width="lg"
        footer={
          selectedGroup && (
            <button onClick={deleteGroup} className="btn btn-danger flex-1">
              Delete Group
            </button>
          )
        }
      >
        {selectedGroup && (
          <div className="space-y-6">
            {/* Group Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedGroup.open
                    ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                    : 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)',
                  color: 'white',
                }}
              >
                <GroupIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedGroup.name}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={selectedGroup.open ? 'success' : 'warning'}>
                    {selectedGroup.open ? 'Open' : 'Closed'}
                  </Badge>
                </div>
              </div>
            </div>

            {/* Stats Row */}
            <div className="grid grid-cols-2 gap-3">
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedGroup.member_count}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Members</div>
              </div>
              <div className="text-center p-3 rounded-lg" style={{ background: 'var(--bg-tertiary)' }}>
                <div className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>
                  {selectedGroup.max_count}
                </div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>Max Members</div>
              </div>
            </div>

            <Section title="Group Information">
              <Field label="Group ID" mono>
                {selectedGroup.id}
              </Field>
              <Field label="Name">
                {selectedGroup.name}
              </Field>
              <Field label="Description">
                {selectedGroup.description || '-'}
              </Field>
              <Field label="Creator ID" mono>
                {selectedGroup.creator_id}
              </Field>
              <Field label="Created At">
                {formatTimestamp(selectedGroup.created_at)}
              </Field>
            </Section>

            <Section title="Members">
              {members.length > 0 ? (
                <div className="space-y-2 max-h-80 overflow-y-auto">
                  {members.map((member) => {
                    const stateInfo = getMemberStateInfo(member.state);
                    return (
                      <div
                        key={member.user_id}
                        className="flex items-center justify-between p-3 rounded-lg"
                        style={{ background: 'var(--bg-tertiary)' }}
                      >
                        <div>
                          <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
                            {member.username}
                          </div>
                          <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
                            {member.user_id.slice(0, 16)}...
                          </div>
                        </div>
                        <div className="flex items-center gap-3">
                          <Badge variant={stateInfo.variant}>{stateInfo.label}</Badge>
                          <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
                            {formatRelativeTime(member.joined_at)}
                          </span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              ) : (
                <p style={{ color: 'var(--text-muted)' }}>No members in this group</p>
              )}
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}

// Icons
function GroupIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}

function FriendIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
    </svg>
  );
}

function OpenIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z" />
    </svg>
  );
}

function ClosedIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
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
