import { useState, useEffect } from 'react';
import { api } from '../api/client';

interface StorageObject {
  collection: string;
  key: string;
  user_id: string;
  value: any;
  version: string;
  permission_read: number;
  permission_write: number;
  created_at: number;
  updated_at: number;
}

export default function Storage() {
  const [objects, setObjects] = useState<StorageObject[]>([]);
  const [collections, setCollections] = useState<string[]>([]);
  const [selectedCollection, setSelectedCollection] = useState<string>('');
  const [selectedObject, setSelectedObject] = useState<StorageObject | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
    } catch (err: any) {
      setError(err.message);
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
    } catch (err: any) {
      setError(err.message);
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
    } catch (err: any) {
      alert('Failed to create: ' + err.message);
    }
  };

  const deleteObject = async (obj: StorageObject) => {
    if (!confirm('Delete this storage object?')) return;
    try {
      await api.delete(`/api/storage/objects/${obj.collection}/${obj.key}?user_id=${obj.user_id}`);
      setSelectedObject(null);
      loadObjects();
    } catch (err: any) {
      alert('Failed to delete: ' + err.message);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  const getPermissionLabel = (perm: number) => {
    switch (perm) {
      case 0: return 'No Access';
      case 1: return 'Owner Only';
      case 2: return 'Public';
      default: return `Unknown (${perm})`;
    }
  };

  if (error) {
    return <div className="p-6 text-red-400">Error: {error}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Storage</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
        >
          Create Object
        </button>
      </div>

      {/* Create Modal */}
      {showCreate && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-800 rounded-lg p-6 w-[500px]">
            <h2 className="text-xl font-semibold mb-4">Create Storage Object</h2>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Collection</label>
                  <input
                    type="text"
                    value={newObject.collection}
                    onChange={(e) => setNewObject({...newObject, collection: e.target.value})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                    placeholder="e.g., player_data"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Key</label>
                  <input
                    type="text"
                    value={newObject.key}
                    onChange={(e) => setNewObject({...newObject, key: e.target.value})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                    placeholder="e.g., profile"
                  />
                </div>
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">User ID</label>
                <input
                  type="text"
                  value={newObject.user_id}
                  onChange={(e) => setNewObject({...newObject, user_id: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  placeholder="Owner user ID"
                />
              </div>
              <div>
                <label className="block text-sm text-gray-400 mb-1">Value (JSON)</label>
                <textarea
                  value={newObject.value}
                  onChange={(e) => setNewObject({...newObject, value: e.target.value})}
                  className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700 h-32 font-mono text-sm"
                  placeholder="{}"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Read Permission</label>
                  <select
                    value={newObject.permission_read}
                    onChange={(e) => setNewObject({...newObject, permission_read: parseInt(e.target.value)})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  >
                    <option value={0}>No Access</option>
                    <option value={1}>Owner Only</option>
                    <option value={2}>Public</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Write Permission</label>
                  <select
                    value={newObject.permission_write}
                    onChange={(e) => setNewObject({...newObject, permission_write: parseInt(e.target.value)})}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                  >
                    <option value={0}>No Access</option>
                    <option value={1}>Owner Only</option>
                  </select>
                </div>
              </div>
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button
                onClick={() => setShowCreate(false)}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded"
              >
                Cancel
              </button>
              <button
                onClick={createObject}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Collections Sidebar */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            Collections ({collections.length})
          </div>
          <div className="divide-y divide-gray-700">
            {collections.length === 0 ? (
              <div className="px-4 py-3 text-gray-400">No collections</div>
            ) : (
              collections.map((collection) => (
                <div
                  key={collection}
                  className={`px-4 py-3 cursor-pointer hover:bg-gray-700 ${
                    selectedCollection === collection ? 'bg-gray-700' : ''
                  }`}
                  onClick={() => setSelectedCollection(collection)}
                >
                  {collection}
                </div>
              ))
            )}
          </div>
        </div>

        {/* Objects */}
        <div className="lg:col-span-3 space-y-6">
          {/* Filter */}
          {selectedCollection && (
            <div className="bg-gray-800 rounded-lg p-4">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={searchUserId}
                  onChange={(e) => setSearchUserId(e.target.value)}
                  placeholder="Filter by User ID (optional)"
                  className="flex-1 px-3 py-2 bg-gray-900 rounded border border-gray-700"
                />
                <button
                  onClick={loadObjects}
                  className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded"
                >
                  Filter
                </button>
              </div>
            </div>
          )}

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Objects List */}
            <div className="bg-gray-800 rounded-lg overflow-hidden">
              <div className="bg-gray-900 px-4 py-3 font-semibold">
                {selectedCollection ? `Objects in ${selectedCollection}` : 'Select a collection'}
              </div>
              {loading ? (
                <div className="p-4 text-gray-400">Loading...</div>
              ) : !selectedCollection ? (
                <div className="p-4 text-gray-400 text-center">
                  Select a collection to view objects
                </div>
              ) : objects.length === 0 ? (
                <div className="p-4 text-gray-400 text-center">No objects</div>
              ) : (
                <div className="divide-y divide-gray-700 max-h-[500px] overflow-y-auto">
                  {objects.map((obj) => (
                    <div
                      key={`${obj.collection}:${obj.key}:${obj.user_id}`}
                      className={`px-4 py-3 cursor-pointer hover:bg-gray-700 ${
                        selectedObject?.key === obj.key && selectedObject?.user_id === obj.user_id
                          ? 'bg-gray-700'
                          : ''
                      }`}
                      onClick={() => setSelectedObject(obj)}
                    >
                      <div className="font-medium">{obj.key}</div>
                      <div className="text-sm text-gray-400 mt-1 font-mono truncate">
                        {obj.user_id}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Object Details */}
            {selectedObject && (
              <div className="bg-gray-800 rounded-lg p-6">
                <div className="flex justify-between items-start mb-4">
                  <h2 className="text-xl font-semibold">{selectedObject.key}</h2>
                  <button
                    onClick={() => deleteObject(selectedObject)}
                    className="text-red-400 hover:text-red-300 text-sm"
                  >
                    Delete
                  </button>
                </div>
                <div className="space-y-4">
                  <div>
                    <label className="text-gray-400 text-sm">Collection</label>
                    <div>{selectedObject.collection}</div>
                  </div>
                  <div>
                    <label className="text-gray-400 text-sm">User ID</label>
                    <div className="font-mono text-sm">{selectedObject.user_id}</div>
                  </div>
                  <div>
                    <label className="text-gray-400 text-sm">Version</label>
                    <div className="font-mono text-sm">{selectedObject.version}</div>
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <label className="text-gray-400 text-sm">Read</label>
                      <div>{getPermissionLabel(selectedObject.permission_read)}</div>
                    </div>
                    <div>
                      <label className="text-gray-400 text-sm">Write</label>
                      <div>{getPermissionLabel(selectedObject.permission_write)}</div>
                    </div>
                  </div>
                  <div>
                    <label className="text-gray-400 text-sm">Value</label>
                    <pre className="mt-1 p-3 bg-gray-900 rounded text-sm overflow-x-auto">
                      {JSON.stringify(selectedObject.value, null, 2)}
                    </pre>
                  </div>
                  <div className="grid grid-cols-2 gap-4 text-sm text-gray-400">
                    <div>Created: {formatDate(selectedObject.created_at)}</div>
                    <div>Updated: {formatDate(selectedObject.updated_at)}</div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
