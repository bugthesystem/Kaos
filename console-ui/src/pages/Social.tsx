import { useState, useEffect } from 'react';
import { api } from '../api/client';

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

export default function Social() {
  const [activeTab, setActiveTab] = useState<'friends' | 'groups'>('groups');
  const [groups, setGroups] = useState<Group[]>([]);
  const [friends, setFriends] = useState<Friend[]>([]);
  const [members, setMembers] = useState<GroupMember[]>([]);
  const [selectedGroup, setSelectedGroup] = useState<Group | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchUserId, setSearchUserId] = useState('');
  const [showCreateGroup, setShowCreateGroup] = useState(false);
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
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const loadMembers = async (groupId: string) => {
    try {
      const data = await api.get(`/api/social/groups/${groupId}/members`);
      setMembers(data.members || []);
    } catch (err: any) {
      setError(err.message);
    }
  };

  const searchFriends = async () => {
    if (!searchUserId) return;
    try {
      setLoading(true);
      const data = await api.get(`/api/social/friends?user_id=${searchUserId}`);
      setFriends(data.friends || []);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const createGroup = async () => {
    try {
      await api.post('/api/social/groups', newGroup);
      setShowCreateGroup(false);
      setNewGroup({ name: '', description: '', open: true, max_count: 100 });
      loadGroups();
    } catch (err: any) {
      alert('Failed to create: ' + err.message);
    }
  };

  const deleteGroup = async (id: string) => {
    if (!confirm('Delete this group?')) return;
    try {
      await api.delete(`/api/social/groups/${id}`);
      setSelectedGroup(null);
      loadGroups();
    } catch (err: any) {
      alert('Failed to delete: ' + err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  const getFriendState = (state: number) => {
    switch (state) {
      case 0: return { label: 'Friend', color: 'text-green-400' };
      case 1: return { label: 'Pending Sent', color: 'text-yellow-400' };
      case 2: return { label: 'Pending Received', color: 'text-blue-400' };
      case 3: return { label: 'Blocked', color: 'text-red-400' };
      default: return { label: 'Unknown', color: 'text-gray-400' };
    }
  };

  const getMemberState = (state: number) => {
    switch (state) {
      case 0: return { label: 'Superadmin', color: 'text-purple-400' };
      case 1: return { label: 'Admin', color: 'text-blue-400' };
      case 2: return { label: 'Member', color: 'text-green-400' };
      case 3: return { label: 'Join Request', color: 'text-yellow-400' };
      default: return { label: 'Unknown', color: 'text-gray-400' };
    }
  };

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Social</h1>
        {activeTab === 'groups' && (
          <button
            onClick={() => setShowCreateGroup(true)}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
          >
            Create Group
          </button>
        )}
      </div>

      {/* Create Group Modal */}
      {showCreateGroup && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-800 rounded-lg p-6 w-[400px]">
            <h2 className="text-xl font-semibold mb-4">Create Group</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">Name</label>
                <input
                  type="text"
                  value={newGroup.name}
                  onChange={(e) => setNewGroup({...newGroup, name: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  placeholder="Group name"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Description</label>
                <textarea
                  value={newGroup.description}
                  onChange={(e) => setNewGroup({...newGroup, description: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700 h-20"
                  placeholder="Group description"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Open</label>
                  <select
                    value={newGroup.open ? 'true' : 'false'}
                    onChange={(e) => setNewGroup({...newGroup, open: e.target.value === 'true'})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  >
                    <option value="true">Yes (anyone can join)</option>
                    <option value="false">No (invite only)</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Max Members</label>
                  <input
                    type="number"
                    value={newGroup.max_count}
                    onChange={(e) => setNewGroup({...newGroup, max_count: parseInt(e.target.value) || 100})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  />
                </div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button
                onClick={() => setShowCreateGroup(false)}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded"
              >
                Cancel
              </button>
              <button
                onClick={createGroup}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-4 mb-6">
        <button
          onClick={() => setActiveTab('groups')}
          className={`px-4 py-2 rounded ${
            activeTab === 'groups' ? 'bg-blue-600' : 'bg-gray-700 hover:bg-gray-600'
          }`}
        >
          Groups
        </button>
        <button
          onClick={() => setActiveTab('friends')}
          className={`px-4 py-2 rounded ${
            activeTab === 'friends' ? 'bg-blue-600' : 'bg-gray-700 hover:bg-gray-600'
          }`}
        >
          Friends
        </button>
      </div>

      {activeTab === 'groups' ? (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Groups List */}
          <div className="bg-gray-800 rounded-lg overflow-hidden">
            <div className="bg-gray-900 px-4 py-3 font-semibold">
              Groups ({groups.length})
            </div>
            {loading && groups.length === 0 ? (
              <div className="p-4 text-gray-400">Loading...</div>
            ) : groups.length === 0 ? (
              <div className="p-4 text-gray-400">No groups</div>
            ) : (
              <div className="divide-y divide-gray-700">
                {groups.map((group) => (
                  <div
                    key={group.id}
                    className={`px-4 py-3 cursor-pointer hover:bg-gray-700 ${
                      selectedGroup?.id === group.id ? 'bg-gray-700' : ''
                    }`}
                    onClick={() => setSelectedGroup(group)}
                  >
                    <div className="flex justify-between items-center">
                      <span className="font-medium">{group.name}</span>
                      <span className={group.open ? 'text-green-400 text-sm' : 'text-yellow-400 text-sm'}>
                        {group.open ? 'Open' : 'Closed'}
                      </span>
                    </div>
                    <div className="text-sm text-gray-400 mt-1">
                      {group.member_count}/{group.max_count} members
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Group Details & Members */}
          <div className="lg:col-span-2 space-y-6">
            {selectedGroup && (
              <>
                <div className="bg-gray-800 rounded-lg p-6">
                  <div className="flex justify-between items-start mb-4">
                    <h2 className="text-xl font-semibold">{selectedGroup.name}</h2>
                    <button
                      onClick={() => deleteGroup(selectedGroup.id)}
                      className="text-red-400 hover:text-red-300 text-sm"
                    >
                      Delete Group
                    </button>
                  </div>
                  <p className="text-gray-300 mb-4">{selectedGroup.description || 'No description'}</p>
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <span className="text-gray-400">ID:</span>
                      <span className="ml-2 font-mono">{selectedGroup.id}</span>
                    </div>
                    <div>
                      <span className="text-gray-400">Created:</span>
                      <span className="ml-2">{formatDate(selectedGroup.created_at)}</span>
                    </div>
                  </div>
                </div>

                <div className="bg-gray-800 rounded-lg overflow-hidden">
                  <div className="bg-gray-900 px-4 py-3 font-semibold">
                    Members ({members.length})
                  </div>
                  <table className="w-full">
                    <thead className="bg-gray-900/50">
                      <tr>
                        <th className="px-4 py-2 text-left">Username</th>
                        <th className="px-4 py-2 text-left">Role</th>
                        <th className="px-4 py-2 text-right">Joined</th>
                      </tr>
                    </thead>
                    <tbody>
                      {members.length === 0 ? (
                        <tr>
                          <td colSpan={3} className="px-4 py-3 text-gray-400 text-center">
                            No members
                          </td>
                        </tr>
                      ) : (
                        members.map((member) => {
                          const state = getMemberState(member.state);
                          return (
                            <tr key={member.user_id} className="border-t border-gray-700">
                              <td className="px-4 py-2">{member.username}</td>
                              <td className={`px-4 py-2 ${state.color}`}>{state.label}</td>
                              <td className="px-4 py-2 text-right text-gray-400 text-sm">
                                {formatDate(member.joined_at)}
                              </td>
                            </tr>
                          );
                        })
                      )}
                    </tbody>
                  </table>
                </div>
              </>
            )}
            {!selectedGroup && (
              <div className="bg-gray-800 rounded-lg p-6 text-gray-400 text-center">
                Select a group to view details
              </div>
            )}
          </div>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Friends Search */}
          <div className="bg-gray-800 rounded-lg p-4">
            <div className="flex gap-2">
              <input
                type="text"
                value={searchUserId}
                onChange={(e) => setSearchUserId(e.target.value)}
                placeholder="Enter User ID to view their friends"
                className="flex-1 px-3 py-2 bg-gray-900 rounded border border-gray-700"
              />
              <button
                onClick={searchFriends}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
              >
                Search
              </button>
            </div>
          </div>

          {/* Friends List */}
          <div className="bg-gray-800 rounded-lg overflow-hidden">
            <div className="bg-gray-900 px-4 py-3 font-semibold">
              Friends ({friends.length})
            </div>
            <table className="w-full">
              <thead className="bg-gray-900/50">
                <tr>
                  <th className="px-4 py-2 text-left">User ID</th>
                  <th className="px-4 py-2 text-left">Username</th>
                  <th className="px-4 py-2 text-left">Status</th>
                  <th className="px-4 py-2 text-right">Updated</th>
                </tr>
              </thead>
              <tbody>
                {friends.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="px-4 py-3 text-gray-400 text-center">
                      {searchUserId ? 'No friends found' : 'Enter a User ID to search'}
                    </td>
                  </tr>
                ) : (
                  friends.map((friend) => {
                    const state = getFriendState(friend.state);
                    return (
                      <tr key={friend.user_id} className="border-t border-gray-700">
                        <td className="px-4 py-2 font-mono text-sm">{friend.user_id}</td>
                        <td className="px-4 py-2">{friend.username}</td>
                        <td className={`px-4 py-2 ${state.color}`}>{state.label}</td>
                        <td className="px-4 py-2 text-right text-gray-400 text-sm">
                          {formatDate(friend.updated_at)}
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
