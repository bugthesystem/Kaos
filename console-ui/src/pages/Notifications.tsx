import { useState, useEffect } from 'react';
import { api } from '../api/client';

interface Notification {
  id: string;
  user_id: string;
  subject: string;
  content: string;
  code: number;
  persistent: boolean;
  read: boolean;
  created_at: number;
}

export default function Notifications() {
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchUserId, setSearchUserId] = useState('');
  const [showSend, setShowSend] = useState(false);
  const [newNotification, setNewNotification] = useState({
    user_id: '',
    subject: '',
    content: '',
    code: 0,
    persistent: true,
  });

  const searchNotifications = async () => {
    if (!searchUserId) return;
    try {
      setLoading(true);
      const data = await api.get(`/api/notifications?user_id=${searchUserId}`);
      setNotifications(data.notifications || []);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const sendNotification = async () => {
    try {
      await api.post('/api/notifications', newNotification);
      setShowSend(false);
      setNewNotification({ user_id: '', subject: '', content: '', code: 0, persistent: true });
      if (searchUserId === newNotification.user_id) {
        searchNotifications();
      }
    } catch (err: any) {
      alert('Failed to send: ' + err.message);
    }
  };

  const deleteNotification = async (id: string) => {
    try {
      await api.delete(`/api/notifications/${id}`);
      setNotifications(notifications.filter(n => n.id !== id));
    } catch (err: any) {
      alert('Failed to delete: ' + err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  const getCodeColor = (code: number) => {
    if (code >= 0 && code < 100) return 'text-blue-400';
    if (code >= 100 && code < 200) return 'text-green-400';
    if (code >= 200 && code < 300) return 'text-yellow-400';
    return 'text-purple-400';
  };

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Notifications</h1>
        <button
          onClick={() => setShowSend(true)}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
        >
          Send Notification
        </button>
      </div>

      {/* Send Modal */}
      {showSend && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-800 rounded-lg p-6 w-[500px]">
            <h2 className="text-xl font-semibold mb-4">Send Notification</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">User ID</label>
                <input
                  type="text"
                  value={newNotification.user_id}
                  onChange={(e) => setNewNotification({...newNotification, user_id: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  placeholder="Target user ID"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Subject</label>
                <input
                  type="text"
                  value={newNotification.subject}
                  onChange={(e) => setNewNotification({...newNotification, subject: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  placeholder="Notification subject"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Content</label>
                <textarea
                  value={newNotification.content}
                  onChange={(e) => setNewNotification({...newNotification, content: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700 h-24"
                  placeholder="Notification content (JSON or text)"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Code</label>
                  <input
                    type="number"
                    value={newNotification.code}
                    onChange={(e) => setNewNotification({...newNotification, code: parseInt(e.target.value) || 0})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Persistent</label>
                  <select
                    value={newNotification.persistent ? 'true' : 'false'}
                    onChange={(e) => setNewNotification({...newNotification, persistent: e.target.value === 'true'})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  >
                    <option value="true">Yes</option>
                    <option value="false">No</option>
                  </select>
                </div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button
                onClick={() => setShowSend(false)}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded"
              >
                Cancel
              </button>
              <button
                onClick={sendNotification}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
              >
                Send
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Search */}
      <div className="bg-gray-800 rounded-lg p-4 mb-6">
        <div className="flex gap-2">
          <input
            type="text"
            value={searchUserId}
            onChange={(e) => setSearchUserId(e.target.value)}
            placeholder="Enter User ID to search notifications"
            className="flex-1 px-3 py-2 bg-gray-900 rounded border border-gray-700"
          />
          <button
            onClick={searchNotifications}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
          >
            Search
          </button>
        </div>
      </div>

      {/* Notifications List */}
      <div className="bg-gray-800 rounded-lg overflow-hidden">
        <div className="bg-gray-900 px-4 py-3 font-semibold">
          Notifications ({notifications.length})
        </div>
        {loading && notifications.length === 0 && searchUserId ? (
          <div className="p-4 text-gray-400">Loading...</div>
        ) : notifications.length === 0 ? (
          <div className="p-4 text-gray-400 text-center">
            {searchUserId ? 'No notifications found' : 'Enter a User ID to search'}
          </div>
        ) : (
          <div className="divide-y divide-gray-700">
            {notifications.map((notification) => (
              <div key={notification.id} className="p-4">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <span className="font-semibold">{notification.subject}</span>
                    <span className={`ml-2 text-sm ${getCodeColor(notification.code)}`}>
                      (code: {notification.code})
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    {notification.read ? (
                      <span className="text-gray-500 text-sm">Read</span>
                    ) : (
                      <span className="text-blue-400 text-sm">Unread</span>
                    )}
                    {notification.persistent && (
                      <span className="text-yellow-400 text-sm">Persistent</span>
                    )}
                    <button
                      onClick={() => deleteNotification(notification.id)}
                      className="text-red-400 hover:text-red-300 text-sm"
                    >
                      Delete
                    </button>
                  </div>
                </div>
                <div className="text-gray-300 text-sm mb-2">{notification.content}</div>
                <div className="text-gray-500 text-xs">
                  {formatDate(notification.created_at)}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
