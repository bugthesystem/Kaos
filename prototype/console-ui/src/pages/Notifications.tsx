import { useState } from 'react';
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
    if (code >= 0 && code < 100) return 'var(--color-accent)';
    if (code >= 100 && code < 200) return 'var(--color-success)';
    if (code >= 200 && code < 300) return 'var(--color-warning)';
    return 'var(--color-tertiary)';
  };

  if (error) {
    return <div className="p-6" style={{ color: 'var(--color-danger)' }}>Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold" style={{ color: 'var(--text-primary)' }}>Notifications</h1>
        <button
          onClick={() => setShowSend(true)}
          className="btn btn-primary"
        >
          Send Notification
        </button>
      </div>

      {/* Send Modal */}
      {showSend && (
        <div className="modal-overlay">
          <div className="modal w-[500px]">
            <h2 className="modal-title">Send Notification</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>User ID</label>
                <input
                  type="text"
                  value={newNotification.user_id}
                  onChange={(e) => setNewNotification({...newNotification, user_id: e.target.value})}
                  className="input"
                  placeholder="Target user ID"
                />
              </div>
              <div>
                <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Subject</label>
                <input
                  type="text"
                  value={newNotification.subject}
                  onChange={(e) => setNewNotification({...newNotification, subject: e.target.value})}
                  className="input"
                  placeholder="Notification subject"
                />
              </div>
              <div>
                <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Content</label>
                <textarea
                  value={newNotification.content}
                  onChange={(e) => setNewNotification({...newNotification, content: e.target.value})}
                  className="input h-24"
                  placeholder="Notification content (JSON or text)"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Code</label>
                  <input
                    type="number"
                    value={newNotification.code}
                    onChange={(e) => setNewNotification({...newNotification, code: parseInt(e.target.value) || 0})}
                    className="input"
                  />
                </div>
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Persistent</label>
                  <select
                    value={newNotification.persistent ? 'true' : 'false'}
                    onChange={(e) => setNewNotification({...newNotification, persistent: e.target.value === 'true'})}
                    className="input"
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
                className="btn btn-secondary"
              >
                Cancel
              </button>
              <button
                onClick={sendNotification}
                className="btn btn-primary"
              >
                Send
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Search */}
      <div className="card mb-6">
        <div className="flex gap-2">
          <input
            type="text"
            value={searchUserId}
            onChange={(e) => setSearchUserId(e.target.value)}
            placeholder="Enter User ID to search notifications"
            className="input flex-1"
          />
          <button
            onClick={searchNotifications}
            className="btn btn-primary"
          >
            Search
          </button>
        </div>
      </div>

      {/* Notifications List */}
      <div className="card overflow-hidden !p-0">
        <div className="px-4 py-3 font-semibold" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)' }}>
          Notifications ({notifications.length})
        </div>
        {loading && notifications.length === 0 && searchUserId ? (
          <div className="p-4" style={{ color: 'var(--text-secondary)' }}>Loading...</div>
        ) : notifications.length === 0 ? (
          <div className="p-4 text-center" style={{ color: 'var(--text-secondary)' }}>
            {searchUserId ? 'No notifications found' : 'Enter a User ID to search'}
          </div>
        ) : (
          <div style={{ borderColor: 'var(--border-primary)' }}>
            {notifications.map((notification, index) => (
              <div
                key={notification.id}
                className="p-4"
                style={{ borderTop: index > 0 ? '1px solid var(--border-primary)' : 'none' }}
              >
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <span className="font-semibold" style={{ color: 'var(--text-primary)' }}>{notification.subject}</span>
                    <span className="ml-2 text-sm" style={{ color: getCodeColor(notification.code) }}>
                      (code: {notification.code})
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    {notification.read ? (
                      <span className="text-sm" style={{ color: 'var(--text-muted)' }}>Read</span>
                    ) : (
                      <span className="text-sm" style={{ color: 'var(--color-accent)' }}>Unread</span>
                    )}
                    {notification.persistent && (
                      <span className="text-sm" style={{ color: 'var(--color-warning)' }}>Persistent</span>
                    )}
                    <button
                      onClick={() => deleteNotification(notification.id)}
                      className="text-sm hover:opacity-80"
                      style={{ color: 'var(--color-danger)' }}
                    >
                      Delete
                    </button>
                  </div>
                </div>
                <div className="text-sm mb-2" style={{ color: 'var(--text-secondary)' }}>{notification.content}</div>
                <div className="text-xs" style={{ color: 'var(--text-muted)' }}>
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
