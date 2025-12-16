import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';

interface StorageObject {
  collection: string;
  key: string;
  user_id: string;
  value: Record<string, unknown>;
  version: string;
  permission_read: number;
  permission_write: number;
  created_at: number;
  updated_at: number;
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

export default function Storage() {
  const [objects, setObjects] = useState<StorageObject[]>([]);
  const [collections, setCollections] = useState<string[]>([]);
  const [selectedCollection, setSelectedCollection] = useState<string>('');
  const [selectedObject, setSelectedObject] = useState<StorageObject | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [searchUserId, setSearchUserId] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [newObject, setNewObject] = useState({
    collection: '',
    key: '',
    user_id: '',
    value: '{}',
    permission_read: 1,
    permission_write: 1,
  });

  useEffect(() => {
    loadCollections();
  }, []);

  useEffect(() => {
    if (selectedCollection) {
      loadObjects();
    }
  }, [selectedCollection]);

  const loadCollections = async () => {
    try {
      const data = await api.get('/api/storage/collections');
      setCollections(data.collections || []);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load collections');
    }
  };

  const loadObjects = async () => {
    try {
      setLoading(true);
      let url = `/api/storage/objects?collection=${selectedCollection}`;
      if (searchUserId) {
        url += `&user_id=${searchUserId}`;
      }
      const data = await api.get(url);
      setObjects(data.objects || []);
    } catch (err) {
      console.error('Failed to load objects:', err);
      setObjects([]);
    } finally {
      setLoading(false);
    }
  };

  const createObject = async () => {
    try {
      let value;
      try {
        value = JSON.parse(newObject.value);
      } catch {
        alert('Invalid JSON value');
        return;
      }

      await api.post('/api/storage/objects', {
        ...newObject,
        value,
      });
      setShowCreate(false);
      setNewObject({
        collection: '',
        key: '',
        user_id: '',
        value: '{}',
        permission_read: 1,
        permission_write: 1,
      });
      loadCollections();
      if (newObject.collection === selectedCollection) {
        loadObjects();
      }
    } catch (err) {
      alert('Failed to create: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const deleteObject = async () => {
    if (!selectedObject) return;
    if (!confirm('Delete this storage object?')) return;
    try {
      await api.delete(`/api/storage/objects/${selectedObject.collection}/${selectedObject.key}?user_id=${selectedObject.user_id}`);
      setDrawerOpen(false);
      setSelectedObject(null);
      loadObjects();
    } catch (err) {
      alert('Failed to delete: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const handleRowClick = (obj: StorageObject) => {
    setSelectedObject(obj);
    setDrawerOpen(true);
  };

  const getPermissionLabel = (perm: number) => {
    switch (perm) {
      case 0: return 'No Access';
      case 1: return 'Owner Only';
      case 2: return 'Public';
      default: return `Unknown (${perm})`;
    }
  };

  const getPermissionVariant = (perm: number): 'danger' | 'warning' | 'success' => {
    switch (perm) {
      case 0: return 'danger';
      case 1: return 'warning';
      case 2: return 'success';
      default: return 'warning';
    }
  };

  const columns: Column<StorageObject>[] = [
    {
      key: 'key',
      header: 'Object',
      render: (obj) => (
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold"
            style={{
              background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
              color: 'white',
            }}
          >
            <StorageIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {obj.key}
            </div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
              {obj.user_id.slice(0, 12)}...
            </div>
          </div>
        </div>
      ),
    },
    {
      key: 'permission_read',
      header: 'Read',
      width: '100px',
      render: (obj) => (
        <Badge variant={getPermissionVariant(obj.permission_read)}>
          {getPermissionLabel(obj.permission_read)}
        </Badge>
      ),
    },
    {
      key: 'permission_write',
      header: 'Write',
      width: '100px',
      render: (obj) => (
        <Badge variant={getPermissionVariant(obj.permission_write)}>
          {getPermissionLabel(obj.permission_write)}
        </Badge>
      ),
    },
    {
      key: 'updated_at',
      header: 'Updated',
      width: '120px',
      render: (obj) => (
        <span style={{ color: 'var(--text-muted)' }}>
          {formatRelativeTime(obj.updated_at)}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Storage</h1>
          <p className="page-subtitle">
            Persistent data storage
          </p>
        </div>
        <button onClick={() => setShowCreate(true)} className="btn btn-primary">
          Create Object
        </button>
      </div>

      {error && (
        <div className="alert alert-danger">
          {error}
        </div>
      )}

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="stat-card">
          <div className="stat-icon">
            <CollectionIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{collections.length}</span>
          <span className="stat-label">Collections</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <StorageIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{objects.length}</span>
          <span className="stat-label">Objects in View</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <FilterIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{selectedCollection || 'All'}</span>
          <span className="stat-label">Current Collection</span>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Collections Sidebar */}
        <div className="card p-0 overflow-hidden">
          <div className="px-4 py-3 border-b" style={{ borderColor: 'var(--border-primary)', background: 'var(--bg-tertiary)' }}>
            <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>
              Collections ({collections.length})
            </h3>
          </div>
          <div className="divide-y" style={{ borderColor: 'var(--border-primary)' }}>
            {collections.length === 0 ? (
              <div className="px-4 py-3" style={{ color: 'var(--text-muted)' }}>No collections</div>
            ) : (
              collections.map((collection) => (
                <div
                  key={collection}
                  className="px-4 py-3 cursor-pointer transition-colors"
                  style={{
                    background: selectedCollection === collection ? 'var(--bg-tertiary)' : 'transparent',
                    color: 'var(--text-primary)',
                  }}
                  onClick={() => setSelectedCollection(collection)}
                  onMouseEnter={(e) => {
                    if (selectedCollection !== collection) {
                      e.currentTarget.style.background = 'var(--bg-secondary)';
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (selectedCollection !== collection) {
                      e.currentTarget.style.background = 'transparent';
                    }
                  }}
                >
                  <div className="flex items-center gap-2">
                    <CollectionIcon className="w-4 h-4" style={{ color: 'var(--color-accent)' }} />
                    <span className="font-medium">{collection}</span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Objects Area */}
        <div className="lg:col-span-3 space-y-4">
          {/* Filter */}
          {selectedCollection && (
            <div className="card">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={searchUserId}
                  onChange={(e) => setSearchUserId(e.target.value)}
                  placeholder="Filter by User ID (optional)"
                  className="form-input flex-1"
                  onKeyDown={(e) => e.key === 'Enter' && loadObjects()}
                />
                <button onClick={loadObjects} className="btn btn-primary">
                  Filter
                </button>
              </div>
            </div>
          )}

          {/* Objects Table */}
          <div className="card p-0 overflow-hidden">
            <div className="px-4 py-3 border-b" style={{ borderColor: 'var(--border-primary)', background: 'var(--bg-tertiary)' }}>
              <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>
                {selectedCollection ? `Objects in ${selectedCollection}` : 'Select a collection'}
              </h3>
            </div>
            {selectedCollection ? (
              <DataTable
                data={objects}
                columns={columns}
                keyField="key"
                onRowClick={handleRowClick}
                selectedId={selectedObject?.key}
                loading={loading}
                searchable
                searchPlaceholder="Search objects..."
                searchFields={['key', 'user_id']}
                pagination
                pageSize={10}
                emptyMessage="No objects in this collection"
              />
            ) : (
              <div className="p-8 text-center" style={{ color: 'var(--text-muted)' }}>
                Select a collection to view objects
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Create Modal */}
      {showCreate && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center"
          style={{ background: 'rgba(0, 0, 0, 0.5)' }}
          onClick={() => setShowCreate(false)}
        >
          <div
            className="modal"
            style={{ maxWidth: '500px' }}
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="modal-title">Create Storage Object</h2>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="form-label">Collection</label>
                  <input
                    type="text"
                    value={newObject.collection}
                    onChange={(e) => setNewObject({ ...newObject, collection: e.target.value })}
                    className="form-input"
                    placeholder="e.g., player_data"
                  />
                </div>
                <div>
                  <label className="form-label">Key</label>
                  <input
                    type="text"
                    value={newObject.key}
                    onChange={(e) => setNewObject({ ...newObject, key: e.target.value })}
                    className="form-input"
                    placeholder="e.g., profile"
                  />
                </div>
              </div>
              <div>
                <label className="form-label">User ID</label>
                <input
                  type="text"
                  value={newObject.user_id}
                  onChange={(e) => setNewObject({ ...newObject, user_id: e.target.value })}
                  className="form-input"
                  placeholder="Owner user ID"
                />
              </div>
              <div>
                <label className="form-label">Value (JSON)</label>
                <textarea
                  value={newObject.value}
                  onChange={(e) => setNewObject({ ...newObject, value: e.target.value })}
                  className="form-input h-32 font-mono text-sm"
                  placeholder="{}"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="form-label">Read Permission</label>
                  <select
                    value={newObject.permission_read}
                    onChange={(e) => setNewObject({ ...newObject, permission_read: parseInt(e.target.value) })}
                    className="form-input"
                  >
                    <option value={0}>No Access</option>
                    <option value={1}>Owner Only</option>
                    <option value={2}>Public</option>
                  </select>
                </div>
                <div>
                  <label className="form-label">Write Permission</label>
                  <select
                    value={newObject.permission_write}
                    onChange={(e) => setNewObject({ ...newObject, permission_write: parseInt(e.target.value) })}
                    className="form-input"
                  >
                    <option value={0}>No Access</option>
                    <option value={1}>Owner Only</option>
                  </select>
                </div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button onClick={() => setShowCreate(false)} className="btn btn-secondary">
                Cancel
              </button>
              <button onClick={createObject} className="btn btn-primary">
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Object Detail Drawer */}
      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title="Object Details"
        width="lg"
        footer={
          selectedObject && (
            <button onClick={deleteObject} className="btn btn-danger flex-1">
              Delete Object
            </button>
          )
        }
      >
        {selectedObject && (
          <div className="space-y-6">
            {/* Object Header */}
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold"
                style={{
                  background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                  color: 'white',
                }}
              >
                <StorageIcon className="w-8 h-8" />
              </div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>
                  {selectedObject.key}
                </h2>
                <div className="flex items-center gap-2 mt-1">
                  <Badge variant={getPermissionVariant(selectedObject.permission_read)}>
                    Read: {getPermissionLabel(selectedObject.permission_read)}
                  </Badge>
                  <Badge variant={getPermissionVariant(selectedObject.permission_write)}>
                    Write: {getPermissionLabel(selectedObject.permission_write)}
                  </Badge>
                </div>
              </div>
            </div>

            <Section title="Object Information">
              <Field label="Collection" mono>
                {selectedObject.collection}
              </Field>
              <Field label="Key" mono>
                {selectedObject.key}
              </Field>
              <Field label="User ID" mono>
                {selectedObject.user_id}
              </Field>
              <Field label="Version" mono>
                {selectedObject.version}
              </Field>
              <Field label="Created At">
                {formatTimestamp(selectedObject.created_at)}
              </Field>
              <Field label="Updated At">
                {formatTimestamp(selectedObject.updated_at)}
              </Field>
            </Section>

            <Section title="Value">
              <pre
                className="p-4 rounded-lg overflow-x-auto text-sm font-mono"
                style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}
              >
                {JSON.stringify(selectedObject.value, null, 2)}
              </pre>
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}

// Icons
function StorageIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
    </svg>
  );
}

function CollectionIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
  );
}

function FilterIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
    </svg>
  );
}
