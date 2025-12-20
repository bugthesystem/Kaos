import { useState } from 'react';
import { api } from '../api/client';
import { Badge } from '../components/DataTable';
import { PageHeader, Alert } from '../components/ui';
import { BellIcon, RefreshIcon } from '../components/icons';
import { formatTimestamp } from '../utils/formatters';

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
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchUserId, setSearchUserId] = useState('');
  const [showSend, setShowSend] = useState(false);
  const [newNotification, setNewNotification] = useState({ user_id: '', subject: '', content: '', code: 0, persistent: true });

  const searchNotifications = async () => {
    if (!searchUserId) return;
    try {
      setLoading(true);
      setError(null);
      const data = await api.get(`/api/notifications?user_id=${searchUserId}`);
      setNotifications(data.notifications || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to search notifications');
    } finally {
      setLoading(false);
    }
  };

  const sendNotification = async () => {
    try {
      await api.post('/api/notifications', newNotification);
      setShowSend(false);
      setNewNotification({ user_id: '', subject: '', content: '', code: 0, persistent: true });
      if (searchUserId === newNotification.user_id) searchNotifications();
    } catch (err) {
      alert('Failed to send: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const deleteNotification = async (id: string) => {
    try {
      await api.delete(`/api/notifications/${id}`);
      setNotifications(notifications.filter(n => n.id !== id));
    } catch (err) {
      alert('Failed to delete: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const getCodeVariant = (code: number): 'info' | 'success' | 'warning' | 'default' => {
    if (code >= 0 && code < 100) return 'info';
    if (code >= 100 && code < 200) return 'success';
    if (code >= 200 && code < 300) return 'warning';
    return 'default';
  };

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader title="Notifications" subtitle="Send and manage user notifications">
        <button onClick={() => setShowSend(true)} className="btn btn-primary">Send Notification</button>
      </PageHeader>

      {error && <Alert variant="danger" onDismiss={() => setError(null)}>{error}</Alert>}

      {showSend && (
        <div className="modal-overlay"><div className="modal w-[500px]">
          <h2 className="modal-title">Send Notification</h2>
          <div className="space-y-4">
            <div><label className="form-label">User ID</label><input type="text" value={newNotification.user_id} onChange={(e) => setNewNotification({ ...newNotification, user_id: e.target.value })} className="form-input" placeholder="Target user ID" /></div>
            <div><label className="form-label">Subject</label><input type="text" value={newNotification.subject} onChange={(e) => setNewNotification({ ...newNotification, subject: e.target.value })} className="form-input" placeholder="Notification subject" /></div>
            <div><label className="form-label">Content</label><textarea value={newNotification.content} onChange={(e) => setNewNotification({ ...newNotification, content: e.target.value })} className="form-input h-24" placeholder="Notification content (JSON or text)" /></div>
            <div className="grid grid-cols-2 gap-4">
              <div><label className="form-label">Code</label><input type="number" value={newNotification.code} onChange={(e) => setNewNotification({ ...newNotification, code: parseInt(e.target.value) || 0 })} className="form-input" /></div>
              <div><label className="form-label">Persistent</label><select value={newNotification.persistent ? 'true' : 'false'} onChange={(e) => setNewNotification({ ...newNotification, persistent: e.target.value === 'true' })} className="form-input"><option value="true">Yes</option><option value="false">No</option></select></div>
            </div>
          </div>
          <div className="flex justify-end gap-2 mt-6"><button onClick={() => setShowSend(false)} className="btn btn-secondary">Cancel</button><button onClick={sendNotification} className="btn btn-primary">Send</button></div>
        </div></div>
      )}

      <div className="card">
        <div className="flex gap-2">
          <input type="text" value={searchUserId} onChange={(e) => setSearchUserId(e.target.value)} placeholder="Enter User ID to search notifications" className="form-input flex-1" onKeyDown={(e) => e.key === 'Enter' && searchNotifications()} />
          <button onClick={searchNotifications} className="btn btn-primary">Search</button>
        </div>
      </div>

      <div className="card p-0 overflow-hidden">
        <div className="px-4 py-3 border-b flex items-center justify-between" style={{ borderColor: 'var(--border-primary)', background: 'var(--bg-tertiary)' }}>
          <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Notifications ({notifications.length})</h3>
          {searchUserId && <button onClick={searchNotifications} className="btn btn-secondary btn-sm"><RefreshIcon className="w-4 h-4" /></button>}
        </div>
        {loading && notifications.length === 0 && searchUserId ? (
          <div className="p-4" style={{ color: 'var(--text-secondary)' }}>Loading...</div>
        ) : notifications.length === 0 ? (
          <div className="p-8 text-center" style={{ color: 'var(--text-muted)' }}>{searchUserId ? 'No notifications found' : 'Enter a User ID to search'}</div>
        ) : (
          <div>
            {notifications.map((notification, index) => (
              <div key={notification.id} className="p-4" style={{ borderTop: index > 0 ? '1px solid var(--border-primary)' : 'none' }}>
                <div className="flex justify-between items-start mb-2">
                  <div className="flex items-center gap-3">
                    <div className="w-9 h-9 rounded-lg flex items-center justify-center" style={{ background: notification.read ? 'var(--bg-tertiary)' : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)', color: notification.read ? 'var(--text-muted)' : 'white' }}><BellIcon className="w-5 h-5" /></div>
                    <div>
                      <span className="font-semibold" style={{ color: 'var(--text-primary)' }}>{notification.subject}</span>
                      <Badge variant={getCodeVariant(notification.code)} className="ml-2">code: {notification.code}</Badge>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant={notification.read ? 'default' : 'info'}>{notification.read ? 'Read' : 'Unread'}</Badge>
                    {notification.persistent && <Badge variant="warning">Persistent</Badge>}
                    <button onClick={() => deleteNotification(notification.id)} className="btn btn-danger btn-sm">Delete</button>
                  </div>
                </div>
                <div className="text-sm mb-2 ml-12" style={{ color: 'var(--text-secondary)' }}>{notification.content}</div>
                <div className="text-xs ml-12" style={{ color: 'var(--text-muted)' }}>{formatTimestamp(notification.created_at)}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
