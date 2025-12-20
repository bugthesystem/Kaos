import { useEffect, useState } from 'react';
import { api } from '../api/client';
import { useAuth } from '../contexts/AuthContext';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { useConfirm } from '../components/ConfirmDialog';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { UsersIcon, ShieldIcon, CodeIcon, CheckIcon } from '../components/icons';
import type { AccountInfo } from '../api/types';

export function AccountsPage() {
  const { user: currentUser, hasPermission } = useAuth();
  const canCreate = hasPermission('create:account');
  const canUpdate = hasPermission('update:account');
  const canDelete = hasPermission('delete:account');
  const canDisable = hasPermission('disable:account');

  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [selectedAccount, setSelectedAccount] = useState<AccountInfo | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [showChangePassword, setShowChangePassword] = useState(false);
  const { confirm, ConfirmDialog } = useConfirm();

  useEffect(() => { loadAccounts(); }, []);

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
    if (selectedAccount.id === currentUser?.id) {
      alert('You cannot delete your own account');
      return;
    }
    const confirmed = await confirm({
      title: 'Delete Account',
      message: `Are you sure you want to delete "${selectedAccount.username}"? This action cannot be undone.`,
      confirmLabel: 'Delete',
      variant: 'danger',
    });
    if (!confirmed) return;
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
    if (selectedAccount.id === currentUser?.id) {
      alert('You cannot disable your own account');
      return;
    }
    try {
      const updated = await api.updateAccount(selectedAccount.id, { disabled: !selectedAccount.disabled });
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
          <AccountAvatar account={account} size="sm" />
          <div>
            <div className="font-medium flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
              {account.username}
              {isCurrentUser(account) && (
                <span className="text-xs px-2 py-0.5 rounded-full" style={{ background: 'var(--color-accent)', color: 'white' }}>You</span>
              )}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>{account.id.slice(0, 8)}...</div>
          </div>
        </div>
      ),
    },
    {
      key: 'role',
      header: 'Role',
      width: '120px',
      render: (account) => (
        <Badge variant={account.role === 'admin' ? 'danger' : account.role === 'developer' ? 'warning' : 'info'}>
          {account.role.charAt(0).toUpperCase() + account.role.slice(1)}
        </Badge>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      width: '100px',
      render: (account) => <Badge variant={account.disabled ? 'neutral' : 'success'}>{account.disabled ? 'Disabled' : 'Active'}</Badge>,
    },
  ];

  const adminCount = accounts.filter(a => a.role === 'admin').length;
  const devCount = accounts.filter(a => a.role === 'developer').length;
  const activeCount = accounts.filter(a => !a.disabled).length;

  return (
    <div className="space-y-6 animate-fade-in">
      {ConfirmDialog}
      <PageHeader title="Accounts" subtitle="Manage console users and permissions">
        {canCreate && <button onClick={() => setShowCreate(true)} className="btn btn-primary">+ New Account</button>}
      </PageHeader>

      {error && <Alert variant="danger" onDismiss={() => setError('')}>{error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<UsersIcon className="w-5 h-5" />} label="Total Accounts" value={accounts.length} color="primary" />
        <StatCard icon={<ShieldIcon className="w-5 h-5" />} label="Admins" value={adminCount} color="danger" />
        <StatCard icon={<CodeIcon className="w-5 h-5" />} label="Developers" value={devCount} color="warning" />
        <StatCard icon={<CheckIcon className="w-5 h-5" />} label="Active" value={activeCount} color="success" />
      </StatGrid>

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

      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Account Details"
        width="md"
        footer={selectedAccount && (
          <>
            {canUpdate && <button onClick={() => setShowChangePassword(true)} className="btn btn-secondary flex-1">Change Password</button>}
            {canDelete && !isCurrentUser(selectedAccount) && <button onClick={handleDelete} className="btn btn-danger">Delete</button>}
          </>
        )}
      >
        {selectedAccount && <AccountDetails account={selectedAccount} isCurrentUser={isCurrentUser(selectedAccount)} onRoleChange={handleRoleChange} onToggleDisabled={handleToggleDisabled} canUpdate={canUpdate} canDisable={canDisable} />}
      </Drawer>

      {showCreate && <CreateAccountModal onClose={() => setShowCreate(false)} onCreated={() => { setShowCreate(false); loadAccounts(); }} />}
      {showChangePassword && selectedAccount && <ChangePasswordModal account={selectedAccount} onClose={() => setShowChangePassword(false)} onChanged={() => setShowChangePassword(false)} />}
    </div>
  );
}

function AccountAvatar({ account, size = 'sm' }: { account: AccountInfo; size?: 'sm' | 'lg' }) {
  const sizeClasses = size === 'lg' ? 'w-16 h-16 text-2xl' : 'w-9 h-9 text-sm';
  const bg = account.role === 'admin' ? 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)'
    : account.role === 'developer' ? 'linear-gradient(135deg, #f59e0b 0%, #d97706 100%)'
    : 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)';
  return (
    <div className={`${sizeClasses} rounded-${size === 'lg' ? 'xl' : 'lg'} flex items-center justify-center font-${size === 'lg' ? 'bold' : 'semibold'}`} style={{ background: bg, color: 'white' }}>
      {account.username.charAt(0).toUpperCase()}
    </div>
  );
}

function AccountDetails({ account, isCurrentUser, onRoleChange, onToggleDisabled, canUpdate, canDisable }: { account: AccountInfo; isCurrentUser: boolean; onRoleChange: (role: string) => void; onToggleDisabled: () => void; canUpdate: boolean; canDisable: boolean }) {
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <AccountAvatar account={account} size="lg" />
        <div>
          <h2 className="text-xl font-semibold flex items-center gap-2" style={{ color: 'var(--text-primary)' }}>
            {account.username}
            {isCurrentUser && <span className="text-xs px-2 py-0.5 rounded-full" style={{ background: 'var(--color-accent)', color: 'white' }}>You</span>}
          </h2>
          <div className="flex items-center gap-2 mt-1">
            <Badge variant={account.role === 'admin' ? 'danger' : account.role === 'developer' ? 'warning' : 'info'}>
              {account.role.charAt(0).toUpperCase() + account.role.slice(1)}
            </Badge>
            <Badge variant={account.disabled ? 'neutral' : 'success'}>{account.disabled ? 'Disabled' : 'Active'}</Badge>
          </div>
        </div>
      </div>

      <Section title="Account Information">
        <Field label="Account ID" mono>{account.id}</Field>
        <Field label="Username">{account.username}</Field>
      </Section>

      <Section title="Role & Permissions">
        <div className="space-y-3">
          <label className="block text-sm font-medium" style={{ color: 'var(--text-secondary)' }}>Role</label>
          <select value={account.role} onChange={(e) => onRoleChange(e.target.value)} className="input w-full" disabled={!canUpdate || (isCurrentUser && account.role === 'admin')}>
            <option value="viewer">Viewer</option>
            <option value="developer">Developer</option>
            <option value="admin">Admin</option>
          </select>
          {isCurrentUser && account.role === 'admin' && <p className="text-xs" style={{ color: 'var(--text-muted)' }}>You cannot demote yourself from admin</p>}
          {!canUpdate && <p className="text-xs" style={{ color: 'var(--text-muted)' }}>You don't have permission to change roles</p>}
        </div>
        {canDisable && (
          <div className="mt-4 pt-4" style={{ borderTop: '1px solid var(--border-color)' }}>
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium" style={{ color: 'var(--text-primary)' }}>Account Status</p>
                <p className="text-sm" style={{ color: 'var(--text-muted)' }}>{account.disabled ? 'This account is disabled and cannot log in' : 'This account is active'}</p>
              </div>
              <button onClick={onToggleDisabled} disabled={isCurrentUser} className={`btn ${account.disabled ? 'btn-success' : 'btn-secondary'}`}>
                {account.disabled ? 'Enable' : 'Disable'}
              </button>
            </div>
            {isCurrentUser && <p className="text-xs mt-2" style={{ color: 'var(--text-muted)' }}>You cannot disable your own account</p>}
          </div>
        )}
      </Section>

      {isCurrentUser && (
        <div className="p-4 rounded-lg" style={{ background: 'rgba(99, 102, 241, 0.1)', border: '1px solid rgba(99, 102, 241, 0.2)' }}>
          <p className="text-sm" style={{ color: 'var(--color-accent)' }}>This is your account. Some actions are restricted to prevent accidental lockout.</p>
        </div>
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
          {error && <div className="alert alert-danger text-sm">{error}</div>}
          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>Username</label>
            <input type="text" value={username} onChange={(e) => setUsername(e.target.value)} className="input w-full" required autoFocus />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>Password</label>
            <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} className="input w-full" required minLength={6} />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>Role</label>
            <select value={role} onChange={(e) => setRole(e.target.value)} className="input w-full">
              <option value="viewer">Viewer - Read-only access</option>
              <option value="developer">Developer - Can manage resources</option>
              <option value="admin">Admin - Full access</option>
            </select>
          </div>
          <div className="flex gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">Cancel</button>
            <button type="submit" disabled={loading} className="btn btn-primary flex-1">{loading ? 'Creating...' : 'Create Account'}</button>
          </div>
        </form>
      </div>
    </div>
  );
}

function ChangePasswordModal({ account, onClose, onChanged }: { account: AccountInfo; onClose: () => void; onChanged: () => void }) {
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (password !== confirmPassword) { setError('Passwords do not match'); return; }
    if (password.length < 6) { setError('Password must be at least 6 characters'); return; }
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
        <p className="text-sm mb-4" style={{ color: 'var(--text-muted)' }}>Set a new password for <strong>{account.username}</strong></p>
        <form onSubmit={handleSubmit} className="space-y-4">
          {error && <div className="alert alert-danger text-sm">{error}</div>}
          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>New Password</label>
            <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} className="input w-full" required minLength={6} autoFocus />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>Confirm Password</label>
            <input type="password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} className="input w-full" required />
          </div>
          <div className="flex gap-3 pt-2">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">Cancel</button>
            <button type="submit" disabled={loading} className="btn btn-primary flex-1">{loading ? 'Saving...' : 'Change Password'}</button>
          </div>
        </form>
      </div>
    </div>
  );
}
