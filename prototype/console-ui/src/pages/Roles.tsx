import { useState } from 'react';
import { PageHeader, StatCard, StatGrid, Card } from '../components/ui';
import { Badge } from '../components/DataTable';
import { ShieldIcon, CodeIcon, ViewIcon, CheckIcon, XIcon } from '../components/icons';
import { ROLE_PERMISSIONS, type Role, type Permission } from '../api/types';

// Group permissions by category for display
const PERMISSION_GROUPS: { category: string; permissions: { id: Permission; label: string; description: string }[] }[] = [
  {
    category: 'Status & Monitoring',
    permissions: [
      { id: 'view:status', label: 'View Status', description: 'View server status and health' },
      { id: 'view:metrics', label: 'View Metrics', description: 'Access server metrics and statistics' },
      { id: 'view:config', label: 'View Config', description: 'View server configuration' },
    ],
  },
  {
    category: 'Sessions',
    permissions: [
      { id: 'view:sessions', label: 'View Sessions', description: 'List and view session details' },
      { id: 'kick:session', label: 'Kick Session', description: 'Disconnect active sessions' },
    ],
  },
  {
    category: 'Rooms',
    permissions: [
      { id: 'view:rooms', label: 'View Rooms', description: 'List and view room details' },
      { id: 'terminate:room', label: 'Terminate Room', description: 'Close active game rooms' },
    ],
  },
  {
    category: 'Accounts',
    permissions: [
      { id: 'view:accounts', label: 'View Accounts', description: 'List console accounts' },
      { id: 'create:account', label: 'Create Account', description: 'Create new console accounts' },
      { id: 'update:account', label: 'Update Account', description: 'Modify account settings' },
      { id: 'delete:account', label: 'Delete Account', description: 'Remove console accounts' },
      { id: 'disable:account', label: 'Disable Account', description: 'Enable/disable accounts' },
    ],
  },
  {
    category: 'API Keys',
    permissions: [
      { id: 'view:apikeys', label: 'View API Keys', description: 'List API keys' },
      { id: 'create:apikey', label: 'Create API Key', description: 'Generate new API keys' },
      { id: 'delete:apikey', label: 'Delete API Key', description: 'Revoke API keys' },
    ],
  },
  {
    category: 'Lua Scripting',
    permissions: [
      { id: 'view:scripts', label: 'View Scripts', description: 'List and view Lua scripts' },
      { id: 'reload:scripts', label: 'Reload Scripts', description: 'Hot reload Lua scripts' },
      { id: 'execute:rpc', label: 'Execute RPC', description: 'Run Lua RPC functions' },
    ],
  },
  {
    category: 'Storage',
    permissions: [
      { id: 'view:storage', label: 'View Storage', description: 'Browse storage objects' },
      { id: 'write:storage', label: 'Write Storage', description: 'Create storage objects' },
      { id: 'delete:storage', label: 'Delete Storage', description: 'Remove storage objects' },
    ],
  },
  {
    category: 'Leaderboards',
    permissions: [
      { id: 'view:leaderboards', label: 'View Leaderboards', description: 'Browse leaderboards' },
      { id: 'delete:leaderboard', label: 'Delete Leaderboard', description: 'Remove leaderboards' },
      { id: 'delete:leaderboard_record', label: 'Delete Record', description: 'Remove leaderboard records' },
    ],
  },
  {
    category: 'Matchmaker',
    permissions: [
      { id: 'view:matchmaker', label: 'View Matchmaker', description: 'View matchmaking queues' },
      { id: 'cancel:matchmaker_ticket', label: 'Cancel Ticket', description: 'Cancel matchmaking tickets' },
    ],
  },
  {
    category: 'Notifications',
    permissions: [
      { id: 'view:notifications', label: 'View Notifications', description: 'View notification history' },
      { id: 'send:notification', label: 'Send Notification', description: 'Send push notifications' },
    ],
  },
  {
    category: 'Chat',
    permissions: [
      { id: 'view:chat', label: 'View Chat', description: 'View chat messages' },
      { id: 'delete:chat_message', label: 'Delete Message', description: 'Remove chat messages' },
    ],
  },
];

const ROLE_INFO: { role: Role; label: string; description: string; icon: typeof ShieldIcon; variant: 'danger' | 'warning' | 'info' }[] = [
  { role: 'admin', label: 'Admin', description: 'Full access to everything including account management', icon: ShieldIcon, variant: 'danger' },
  { role: 'developer', label: 'Developer', description: 'Can view all data and execute RPCs, but cannot manage accounts', icon: CodeIcon, variant: 'warning' },
  { role: 'viewer', label: 'Viewer', description: 'Read-only access to non-sensitive data', icon: ViewIcon, variant: 'info' },
];

export default function Roles() {
  const [selectedRole, setSelectedRole] = useState<Role>('admin');
  const selectedRoleInfo = ROLE_INFO.find(r => r.role === selectedRole)!;
  const rolePermissions = ROLE_PERMISSIONS[selectedRole];

  const adminCount = ROLE_PERMISSIONS.admin.length;
  const developerCount = ROLE_PERMISSIONS.developer.length;
  const viewerCount = ROLE_PERMISSIONS.viewer.length;

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader title="Roles & Permissions" subtitle="Role-based access control configuration" />

      <StatGrid columns={3}>
        <StatCard
          icon={<ShieldIcon className="w-5 h-5" />}
          label="Admin Permissions"
          value={adminCount}
          color="danger"
        />
        <StatCard
          icon={<CodeIcon className="w-5 h-5" />}
          label="Developer Permissions"
          value={developerCount}
          color="warning"
        />
        <StatCard
          icon={<ViewIcon className="w-5 h-5" />}
          label="Viewer Permissions"
          value={viewerCount}
          color="info"
        />
      </StatGrid>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Role Selector */}
        <div className="card p-0 overflow-hidden">
          <div className="px-4 py-3 border-b" style={{ borderColor: 'var(--border-primary)', background: 'var(--bg-tertiary)' }}>
            <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Roles</h3>
          </div>
          <div className="divide-y" style={{ borderColor: 'var(--border-primary)' }}>
            {ROLE_INFO.map((role) => {
              const Icon = role.icon;
              const isSelected = selectedRole === role.role;
              return (
                <div
                  key={role.role}
                  className="px-4 py-3 cursor-pointer transition-colors"
                  style={{
                    background: isSelected ? 'var(--bg-tertiary)' : 'transparent',
                  }}
                  onClick={() => setSelectedRole(role.role)}
                >
                  <div className="flex items-center gap-3">
                    <div
                      className="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0"
                      style={{
                        background: role.variant === 'danger' ? 'rgba(239, 68, 68, 0.15)' :
                          role.variant === 'warning' ? 'rgba(245, 158, 11, 0.15)' : 'rgba(59, 130, 246, 0.15)',
                        color: role.variant === 'danger' ? '#ef4444' :
                          role.variant === 'warning' ? '#f59e0b' : '#3b82f6',
                      }}
                    >
                      <Icon className="w-5 h-5" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium" style={{ color: 'var(--text-primary)' }}>{role.label}</span>
                        <Badge variant={role.variant}>{ROLE_PERMISSIONS[role.role].length}</Badge>
                      </div>
                      <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>{role.description}</p>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Permissions Matrix */}
        <div className="lg:col-span-3 space-y-4">
          <Card>
            <div className="flex items-center gap-4 mb-6">
              {(() => {
                const Icon = selectedRoleInfo.icon;
                return (
                  <div
                    className="w-14 h-14 rounded-xl flex items-center justify-center"
                    style={{
                      background: selectedRoleInfo.variant === 'danger' ? 'rgba(239, 68, 68, 0.15)' :
                        selectedRoleInfo.variant === 'warning' ? 'rgba(245, 158, 11, 0.15)' : 'rgba(59, 130, 246, 0.15)',
                      color: selectedRoleInfo.variant === 'danger' ? '#ef4444' :
                        selectedRoleInfo.variant === 'warning' ? '#f59e0b' : '#3b82f6',
                    }}
                  >
                    <Icon className="w-7 h-7" />
                  </div>
                );
              })()}
              <div>
                <h2 className="text-xl font-semibold flex items-center gap-3" style={{ color: 'var(--text-primary)' }}>
                  {selectedRoleInfo.label} Role
                  <Badge variant={selectedRoleInfo.variant}>{rolePermissions.length} permissions</Badge>
                </h2>
                <p className="text-sm" style={{ color: 'var(--text-muted)' }}>{selectedRoleInfo.description}</p>
              </div>
            </div>

            <div className="space-y-6">
              {PERMISSION_GROUPS.map((group) => {
                const groupPermissions = group.permissions.filter(p =>
                  (ROLE_PERMISSIONS.admin as string[]).includes(p.id)
                );
                if (groupPermissions.length === 0) return null;

                return (
                  <div key={group.category}>
                    <h3 className="text-sm font-medium mb-3 flex items-center gap-2" style={{ color: 'var(--text-secondary)' }}>
                      {group.category}
                      <span className="text-xs px-2 py-0.5 rounded-full" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-muted)' }}>
                        {groupPermissions.filter(p => rolePermissions.includes(p.id)).length}/{groupPermissions.length}
                      </span>
                    </h3>
                    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-2">
                      {groupPermissions.map((permission) => {
                        const hasPermission = rolePermissions.includes(permission.id);
                        return (
                          <div
                            key={permission.id}
                            className="flex items-center gap-3 p-3 rounded-lg transition-colors"
                            style={{
                              background: hasPermission ? 'rgba(34, 197, 94, 0.08)' : 'var(--bg-tertiary)',
                              border: `1px solid ${hasPermission ? 'rgba(34, 197, 94, 0.2)' : 'var(--border-primary)'}`,
                            }}
                          >
                            <div
                              className="w-6 h-6 rounded-full flex items-center justify-center flex-shrink-0"
                              style={{
                                background: hasPermission ? 'rgba(34, 197, 94, 0.2)' : 'rgba(107, 114, 128, 0.2)',
                                color: hasPermission ? '#22c55e' : '#6b7280',
                              }}
                            >
                              {hasPermission ? <CheckIcon className="w-3.5 h-3.5" /> : <XIcon className="w-3.5 h-3.5" />}
                            </div>
                            <div className="flex-1 min-w-0">
                              <p className="text-sm font-medium" style={{ color: hasPermission ? 'var(--text-primary)' : 'var(--text-muted)' }}>
                                {permission.label}
                              </p>
                              <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>
                                {permission.description}
                              </p>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                );
              })}
            </div>
          </Card>

          {/* Role Comparison */}
          <Card title="Role Comparison">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border-primary)' }}>
                    <th className="text-left py-3 px-4 font-medium" style={{ color: 'var(--text-secondary)' }}>Permission</th>
                    {ROLE_INFO.map((role) => (
                      <th key={role.role} className="text-center py-3 px-4 font-medium" style={{ color: 'var(--text-secondary)' }}>
                        {role.label}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {PERMISSION_GROUPS.flatMap(group =>
                    group.permissions.map((permission) => (
                      <tr key={permission.id} style={{ borderBottom: '1px solid var(--border-primary)' }}>
                        <td className="py-2 px-4">
                          <span className="text-sm" style={{ color: 'var(--text-primary)' }}>{permission.label}</span>
                        </td>
                        {ROLE_INFO.map((role) => {
                          const hasPermission = ROLE_PERMISSIONS[role.role].includes(permission.id);
                          return (
                            <td key={role.role} className="text-center py-2 px-4">
                              {hasPermission ? (
                                <CheckIcon className="w-4 h-4 mx-auto" style={{ color: '#22c55e' }} />
                              ) : (
                                <XIcon className="w-4 h-4 mx-auto" style={{ color: '#6b7280' }} />
                              )}
                            </td>
                          );
                        })}
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
