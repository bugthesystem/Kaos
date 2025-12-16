import { useEffect, useState } from 'react';
import { api } from '../api/client';
import { useAuth } from '../contexts/AuthContext';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import type { AccountInfo } from '../api/types';

export function AccountsPage() {
  const { user: currentUser } = useAuth();
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selectedAccount, setSelectedAccount] = useState<AccountInfo | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [showChangePassword, setShowChangePassword] = useState(false);

  useEffect(() => {
    loadAccounts();
  }, []);

  const loadAccounts = async () => {
    setLoading(true);
    try {
      const data = await api.listAccounts();
      setAccounts(data.items);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load accounts');
    } finally {
      setLoading(false);
    }
  };

  const handleRowClick = (account: AccountInfo) => {
    setSelectedAccount(account);
    setDrawerOpen(true);
  };

  const handleDelete = async () => {
    if (!selectedAccount) return;

    // Prevent self-delete
    if (selectedAccount.id === currentUser?.id) {
      alert('You cannot delete your own account');
      return;
    }

    if (!confirm(`Are you sure you want to delete "${selectedAccount.username}"?`)) return;

    try {
      await api.deleteAccount(selectedAccount.id);
      setDrawerOpen(false);
      setSelectedAccount(null);
      loadAccounts();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete account');
    }
  };

  const handleRoleChange = async (newRole: string) => {
    if (!selectedAccount) return;

    // Prevent self-demotion from admin
    if (selectedAccount.id === currentUser?.id && currentUser?.role === 'admin' && newRole !== 'admin') {
      alert('You cannot demote yourself from admin');
      return;
    }

    try {
      const updated = await api.updateAccount(selectedAccount.id, { role: newRole });
      setSelectedAccount(updated);
      loadAccounts();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to update role');
    }
  };

  const handleToggleDisabled = async () => {
    if (!selectedAccount) return;

    // Prevent self-disable
    if (selectedAccount.id === currentUser?.id) {
      alert('You cannot disable your own account');
      return;
    }

    try {
      const updated = await api.updateAccount(selectedAccount.id, {
        disabled: !selectedAccount.disabled
      });
      setSelectedAccount(updated);
      loadAccounts();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to update account');
    }
  };

  const isCurrentUser = (account: AccountInfo) => account.id === currentUser?.id;

  const columns: Column<AccountInfo>[] = [
    {
      key: 'username',
      header: 'User',
      render: (account) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: account.role === 'admin'
                ? 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)'
                : account.role === 'developer'
                ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)'
                : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
              color: 'white',
            }}
          >
            {account.username.charAt(0).toUpperCase()}
          </div>
          <div>
            <div className="font-medium flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
              {account.username}
              {isCurrentUser(account) && (
                <span className="text-xs px-2 py-0.5 rounded-full" style={{
                  background: 'var(--color-accent)',
                  color: 'white'
                }}>
                  You
                </span>
              )}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {account.id.slice(0, 8)}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'role',
      header: 'Role',
      width: '120px',
      render: (account) => (
        <Badge variant={
          account.role === 'admin' ? 'danger' :
          account.role === 'developer' ? 'warning' : 'info'
        }>
          {account.role.charAt(0).toUpperCase() + account.role.slice(1)}
        </Badge>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      width: '100px',
      render: (account) => (
        <Badge variant={account.disabled ? 'neutral' : 'success'}>
          {account.disabled ? 'Disabled' : 'Active'}
        </Badge>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Accounts</h1>
          <p className="page-subtitle">
            Manage console users and permissions
          </p>
        </div>
        <button onClick={() => setShowCreate(true)} className="btn btn-primary">
          + New Account
        </button>
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
            <UsersIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{accounts.length}</span>
          <span className="stat-label">Total Accounts</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <AdminIcon className="w-6 h-6" style={{ color: 'var(--color-danger)' }} />
          </div>
          <span className="stat-value">{accounts.filter(a => a.role === 'admin').length}</span>
          <span className="stat-label">Admins</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <DevIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">{accounts.filter(a => a.role === 'developer').length}</span>
          <span className="stat-label">Developers</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <ActiveIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{accounts.filter(a => !a.disabled).length}</span>
          <span className="stat-label">Active</span>
        </div>
      </div>

      {/* Accounts Table */}
      <div className="card p-0 overflow-hidden">
        <DataTable
          data={accounts}
          columns={columns}
          keyField="id"
          onRowClick={handleRowClick}
          selectedId={selectedAccount?.id}
          loading={loading}
          searchable
          searchPlaceholder="Search accounts..."
          searchFields={['username']}
          pagination
          pageSize={10}
          emptyMessage="No accounts found"
        />
      </div>

      {/* Account Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Account Details"
        width="md"
        footer={
          selectedAccount && (
            <>
              <button
                onClick={() => setShowChangePassword(true)}
                className="btn btn-secondary flex-1"
              >
                Change Password
              </button>
              {!isCurrentUser(selectedAccount) && (
                <button onClick={handleDelete} className="btn btn-danger">
                  Delete
                </button>
              )}
            </>
          )
        }
      >
        {selectedAccount && (
          <div className="space-y-6">
            {/* Account Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: selectedAccount.role === 'admin'
                    ? 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)'
                    : selectedAccount.role === 'developer'
                    ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)'
                    : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                  color: 'white',
                }}
              >
                {selectedAccount.username.charAt(0).toUpperCase()}
              </div>
              <div>
                <h2 className="text-xl font-semibold flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
                  {selectedAccount.username}
                  {isCurrentUser(selectedAccount) && (
                    <span className="text-xs px-2 py-0.5 rounded-full" style={{
                      background: 'var(--color-accent)',
                      color: 'white'
                    }}>
                      You
                    </span>
                  )}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={
                    selectedAccount.role === 'admin' ? 'danger' :
                    selectedAccount.role === 'developer' ? 'warning' : 'info'
                  }>
                    {selectedAccount.role.charAt(0).toUpperCase() + selectedAccount.role.slice(1)}
                  </Badge>
                  <Badge variant={selectedAccount.disabled ? 'neutral' : 'success'}>
                    {selectedAccount.disabled ? 'Disabled' : 'Active'}
                  </Badge>
                </div>
              </div>
            </div>

            <Section title="Account Information">
              <Field label="Account ID" mono>
                {selectedAccount.id}
              </Field>
              <Field label="Username">
                {selectedAccount.username}
              </Field>
            </Section>

            <Section title="Role & Permissions">
              <div className="space-y-3">
                <label className="block text-sm font-medium" style={{ color: 'var(--text-secondary)' }}>
                  Role
                </label>
                <select
                  value={selectedAccount.role}
                  onChange={(e) => handleRoleChange(e.target.value)}
                  className="input w-full"
                  disabled={isCurrentUser(selectedAccount) && selectedAccount.role === 'admin'}
                >
                  <option value="viewer">Viewer</option>
                  <option value="developer">Developer</option>
                  <option value="admin">Admin</option>
                </select>
                {isCurrentUser(selectedAccount) && selectedAccount.role === 'admin' && (
                  <p className="text-xs" style={{ color: 'var(--text-muted)' }}>
                    You cannot demote yourself from admin
                  </p>
                )}
              </div>

              <div className="mt-4 pt-4" style={{ borderTop: '1px solid var(--border-color)' }}>
                <div className="flex items-center justify-between">
                  <div>
                    <p className="font-medium" style={{ color: 'var(--text-primary)' }}>
                      Account Status
                    </p>
                    <p className="text-sm" style={{ color: 'var(--text-muted)' }}>
                      {selectedAccount.disabled ? 'This account is disabled and cannot log in' : 'This account is active'}
                    </p>
                  </div>
                  <button
                    onClick={handleToggleDisabled}
                    disabled={isCurrentUser(selectedAccount)}
                    className={`btn ${selectedAccount.disabled ? 'btn-success' : 'btn-secondary'}`}
                  >
                    {selectedAccount.disabled ? 'Enable' : 'Disable'}
                  </button>
                </div>
                {isCurrentUser(selectedAccount) && (
                  <p className="text-xs mt-2" style={{ color: 'var(--text-muted)' }}>
                    You cannot disable your own account
                  </p>
                )}
              </div>
            </Section>

            {isCurrentUser(selectedAccount) && (
              <div className="p-4 rounded-lg" style={{
                background: 'rgba(99, 102, 241, 0.1)',
                border: '1px solid rgba(99, 102, 241, 0.2)'
              }}>
                <p className="text-sm" style={{ color: 'var(--color-accent)' }}>
                  This is your account. Some actions are restricted to prevent accidental lockout.
                </p>
              </div>
            )}
          </div>
        )}
      </Drawer>

      {/* Create Account Modal */}
      {showCreate && (
        <CreateAccountModal
          onClose={() => setShowCreate(false)}
          onCreated={() => {
            setShowCreate(false);
            loadAccounts();
          }}
        />
      )}

      {/* Change Password Modal */}
      {showChangePassword && selectedAccount && (
        <ChangePasswordModal
          account={selectedAccount}
          onClose={() => setShowChangePassword(false)}
          onChanged={() => setShowChangePassword(false)}
        />
      )}
    </div>
  );
}

function CreateAccountModal({ onClose, onCreated }: { onClose: () => void; onCreated: () => void }) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [role, setRole] = useState('viewer');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');

    try {
      await api.createAccount({ username, password, role });
      onCreated();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create account');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal">
        <h2 className="modal-title">Create Account</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="alert alert-danger text-sm">
              {error}
            </div>
          )}

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              Username
            </label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="input w-full"
              required
              autoFocus
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              Password
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="input w-full"
              required
              minLength={6}
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              Role
            </label>
            <select
              value={role}
              onChange={(e) => setRole(e.target.value)}
              className="input w-full"
            >
              <option value="viewer">Viewer - Read-only access</option>
              <option value="developer">Developer - Can manage resources</option>
              <option value="admin">Admin - Full access</option>
            </select>
          </div>

          <div className="flex gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">
              Cancel
            </button>
            <button type="submit" disabled={loading} className="btn btn-primary flex-1">
              {loading ? 'Creating...' : 'Create Account'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

function ChangePasswordModal({
  account,
  onClose,
  onChanged
}: {
  account: AccountInfo;
  onClose: () => void;
  onChanged: () => void;
}) {
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }

    if (password.length < 6) {
      setError('Password must be at least 6 characters');
      return;
    }

    setLoading(true);
    setError('');

    try {
      await api.changePassword(account.id, password);
      onChanged();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to change password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal">
        <h2 className="modal-title">Change Password</h2>
        <p className="text-sm mb-4" style={{ color: 'var(--text-muted)' }}>
          Set a new password for <strong>{account.username}</strong>
        </p>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="alert alert-danger text-sm">
              {error}
            </div>
          )}

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              New Password
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="input w-full"
              required
              minLength={6}
              autoFocus
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
              Confirm Password
            </label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              className="input w-full"
              required
            />
          </div>

          <div className="flex gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">
              Cancel
            </button>
            <button type="submit" disabled={loading} className="btn btn-primary flex-1">
              {loading ? 'Saving...' : 'Change Password'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// Icons
function UsersIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}

function AdminIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
    </svg>
  );
}

function DevIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
    </svg>
  );
}

function ActiveIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}
