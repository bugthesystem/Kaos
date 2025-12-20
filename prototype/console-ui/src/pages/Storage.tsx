import { useState, useEffect } from 'react';
import { api } from '../api/client';
import { DataTable, Badge, type Column } from '../components/DataTable';
import { Drawer, Field, Section } from '../components/Drawer';
import { useConfirm } from '../components/ConfirmDialog';
import { useAuth } from '../contexts/AuthContext';
import { PageHeader, StatCard, StatGrid, Alert } from '../components/ui';
import { DatabaseIcon, FolderIcon, FilterIcon } from '../components/icons';
import { formatTimestamp, formatRelativeTime } from '../utils/formatters';

interface StorageObject {
  collection: string;
  key: string;
  user_id: string;
  value: Record<string, unknown>;
  version: number;
  permission: string;
  created_at: number;
  updated_at: number;
}

export default function Storage() {
  const { hasPermission } = useAuth();
  const canWrite = hasPermission('write:storage');
  const canDelete = hasPermission('delete:storage');

  const [objects, setObjects] = useState<StorageObject[]>([]);
  const [collections, setCollections] = useState<string[]>([]);
  const [selectedCollection, setSelectedCollection] = useState<string>('');
  const [selectedObject, setSelectedObject] = useState<StorageObject | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [searchUserId, setSearchUserId] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [newObject, setNewObject] = useState({ collection: '', key: '', user_id: '', value: '{}', permission_read: 1, permission_write: 1 });
  const { confirm, ConfirmDialog } = useConfirm();

  useEffect(() => { loadCollections(); }, []);
  useEffect(() => { if (selectedCollection) loadObjects(); }, [selectedCollection]);

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
      if (searchUserId) url += `&user_id=${searchUserId}`;
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
      try { value = JSON.parse(newObject.value); } catch { alert('Invalid JSON value'); return; }
      let permission = 'public_read';
      if (newObject.permission_write === 2) permission = 'public_read_write';
      else if (newObject.permission_read === 1) permission = 'owner_only';
      await api.post('/api/storage', { user_id: newObject.user_id, collection: newObject.collection, key: newObject.key, value, permission });
      setShowCreate(false);
      setNewObject({ collection: '', key: '', user_id: '', value: '{}', permission_read: 1, permission_write: 1 });
      loadCollections();
      if (newObject.collection === selectedCollection) loadObjects();
    } catch (err) {
      alert('Failed to create: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const deleteObject = async () => {
    if (!selectedObject) return;
    const confirmed = await confirm({ title: 'Delete Object', message: 'Delete this storage object? This cannot be undone.', confirmLabel: 'Delete', variant: 'danger' });
    if (!confirmed) return;
    try {
      await api.delete(`/api/storage/${selectedObject.user_id}/${selectedObject.collection}/${selectedObject.key}`);
      setDrawerOpen(false);
      setSelectedObject(null);
      loadObjects();
    } catch (err) {
      alert('Failed to delete: ' + (err instanceof Error ? err.message : 'Unknown error'));
    }
  };

  const handleRowClick = (obj: StorageObject) => { setSelectedObject(obj); setDrawerOpen(true); };
  const getPermissionLabel = (perm: string) => perm === 'owner_only' ? 'Owner Only' : perm === 'public_read' ? 'Public Read' : perm === 'public_read_write' ? 'Public R/W' : perm;
  const getPermissionVariant = (perm: string): 'warning' | 'info' | 'success' => perm === 'owner_only' ? 'warning' : perm === 'public_read_write' ? 'success' : 'info';

  const columns: Column<StorageObject>[] = [
    {
      key: 'key', header: 'Object',
      render: (obj) => (
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg flex items-center justify-center text-sm font-semibold" style={{ background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)', color: 'white' }}><DatabaseIcon className="w-5 h-5" /></div>
          <div>
            <div className="font-medium" style={{ color: 'var(--text-primary)' }}>{obj.key}</div>
            <div className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>{obj.user_id.slice(0, 12)}...</div>
          </div>
        </div>
      ),
    },
    { key: 'permission', header: 'Permission', width: '120px', render: (obj) => <Badge variant={getPermissionVariant(obj.permission)}>{getPermissionLabel(obj.permission)}</Badge> },
    { key: 'updated_at', header: 'Updated', width: '120px', render: (obj) => <span style={{ color: 'var(--text-muted)' }}>{formatRelativeTime(obj.updated_at)}</span> },
  ];

  return (
    <div className="space-y-6 animate-fade-in">
      {ConfirmDialog}
      <PageHeader title="Storage" subtitle="Persistent data storage">
        {canWrite && <button onClick={() => setShowCreate(true)} className="btn btn-primary">Create Object</button>}
      </PageHeader>

      {error && <Alert variant="danger" onDismiss={() => setError('')}>{error}</Alert>}

      <StatGrid columns={3}>
        <StatCard icon={<FolderIcon className="w-5 h-5" />} label="Collections" value={collections.length} color="primary" />
        <StatCard icon={<DatabaseIcon className="w-5 h-5" />} label="Objects in View" value={objects.length} color="success" />
        <StatCard icon={<FilterIcon className="w-5 h-5" />} label="Current Collection" value={selectedCollection || 'All'} color="info" />
      </StatGrid>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <div className="card p-0 overflow-hidden">
          <div className="px-4 py-3 border-b" style={{ borderColor: 'var(--border-primary)', background: 'var(--bg-tertiary)' }}><h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>Collections ({collections.length})</h3></div>
          <div className="divide-y" style={{ borderColor: 'var(--border-primary)' }}>
            {collections.length === 0 ? <div className="px-4 py-3" style={{ color: 'var(--text-muted)' }}>No collections</div> : collections.map((collection) => (
              <div key={collection} className="px-4 py-3 cursor-pointer transition-colors" style={{ background: selectedCollection === collection ? 'var(--bg-tertiary)' : 'transparent', color: 'var(--text-primary)' }} onClick={() => setSelectedCollection(collection)}>
                <div className="flex items-center gap-2"><FolderIcon className="w-4 h-4" style={{ color: 'var(--color-accent)' }} /><span className="font-medium">{collection}</span></div>
              </div>
            ))}
          </div>
        </div>

        <div className="lg:col-span-3 space-y-4">
          {selectedCollection && (
            <div className="card">
              <div className="flex gap-2">
                <input type="text" value={searchUserId} onChange={(e) => setSearchUserId(e.target.value)} placeholder="Filter by User ID (optional)" className="form-input flex-1" onKeyDown={(e) => e.key === 'Enter' && loadObjects()} />
                <button onClick={loadObjects} className="btn btn-primary">Filter</button>
              </div>
            </div>
          )}
          <div className="card p-0 overflow-hidden">
            <div className="px-4 py-3 border-b" style={{ borderColor: 'var(--border-primary)', background: 'var(--bg-tertiary)' }}><h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>{selectedCollection ? `Objects in ${selectedCollection}` : 'Select a collection'}</h3></div>
            {selectedCollection ? <DataTable data={objects} columns={columns} keyField="key" onRowClick={handleRowClick} selectedId={selectedObject?.key} loading={loading} searchable searchPlaceholder="Search objects..." searchFields={['key', 'user_id']} pagination pageSize={10} emptyMessage="No objects in this collection" /> : <div className="p-8 text-center" style={{ color: 'var(--text-muted)' }}>Select a collection to view objects</div>}
          </div>
        </div>
      </div>

      {showCreate && (
        <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: 'rgba(0, 0, 0, 0.5)' }} onClick={() => setShowCreate(false)}>
          <div className="modal" style={{ maxWidth: '500px' }} onClick={(e) => e.stopPropagation()}>
            <h2 className="modal-title">Create Storage Object</h2>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div><label className="form-label">Collection</label><input type="text" value={newObject.collection} onChange={(e) => setNewObject({ ...newObject, collection: e.target.value })} className="form-input" placeholder="e.g., player_data" /></div>
                <div><label className="form-label">Key</label><input type="text" value={newObject.key} onChange={(e) => setNewObject({ ...newObject, key: e.target.value })} className="form-input" placeholder="e.g., profile" /></div>
              </div>
              <div><label className="form-label">User ID</label><input type="text" value={newObject.user_id} onChange={(e) => setNewObject({ ...newObject, user_id: e.target.value })} className="form-input" placeholder="Owner user ID" /></div>
              <div><label className="form-label">Value (JSON)</label><textarea value={newObject.value} onChange={(e) => setNewObject({ ...newObject, value: e.target.value })} className="form-input h-32 font-mono text-sm" placeholder="{}" /></div>
              <div className="grid grid-cols-2 gap-4">
                <div><label className="form-label">Read Permission</label><select value={newObject.permission_read} onChange={(e) => setNewObject({ ...newObject, permission_read: parseInt(e.target.value) })} className="form-input"><option value={0}>No Access</option><option value={1}>Owner Only</option><option value={2}>Public</option></select></div>
                <div><label className="form-label">Write Permission</label><select value={newObject.permission_write} onChange={(e) => setNewObject({ ...newObject, permission_write: parseInt(e.target.value) })} className="form-input"><option value={0}>No Access</option><option value={1}>Owner Only</option></select></div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6"><button onClick={() => setShowCreate(false)} className="btn btn-secondary">Cancel</button><button onClick={createObject} className="btn btn-primary">Create</button></div>
          </div>
        </div>
      )}

      <Drawer open={drawerOpen} onClose={() => setDrawerOpen(false)} title="Object Details" width="lg" footer={selectedObject && canDelete && <button onClick={deleteObject} className="btn btn-danger flex-1">Delete Object</button>}>
        {selectedObject && (
          <div className="space-y-6">
            <div className="flex items-center gap-4">
              <div className="w-16 h-16 rounded-xl flex items-center justify-center text-2xl font-bold" style={{ background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)', color: 'white' }}><DatabaseIcon className="w-8 h-8" /></div>
              <div>
                <h2 className="text-xl font-semibold" style={{ color: 'var(--text-primary)' }}>{selectedObject.key}</h2>
                <Badge variant={getPermissionVariant(selectedObject.permission)}>{getPermissionLabel(selectedObject.permission)}</Badge>
              </div>
            </div>
            <Section title="Object Information">
              <Field label="Collection" mono>{selectedObject.collection}</Field>
              <Field label="Key" mono>{selectedObject.key}</Field>
              <Field label="User ID" mono>{selectedObject.user_id}</Field>
              <Field label="Version" mono>{selectedObject.version}</Field>
              <Field label="Permission">{getPermissionLabel(selectedObject.permission)}</Field>
              <Field label="Created At">{formatTimestamp(selectedObject.created_at)}</Field>
              <Field label="Updated At">{formatTimestamp(selectedObject.updated_at)}</Field>
            </Section>
            <Section title="Value">
              <pre className="p-4 rounded-lg overflow-x-auto text-sm font-mono" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-secondary)' }}>{JSON.stringify(selectedObject.value, null, 2)}</pre>
            </Section>
          </div>
        )}
      </Drawer>
    </div>
  );
}
