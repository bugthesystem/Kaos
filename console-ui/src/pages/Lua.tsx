import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { LuaScriptInfo, RpcInfo } from '../api/types';

export function LuaPage() {
  const [scripts, setScripts] = useState<LuaScriptInfo[]>([]);
  const [rpcs, setRpcs] = useState<RpcInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [reloading, setReloading] = useState(false);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      const [scriptsData, rpcsData] = await Promise.all([
        api.listScripts(),
        api.listRpcs(),
      ]);
      setScripts(scriptsData.items);
      setRpcs(rpcsData.items);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load Lua data');
    } finally {
      setLoading(false);
    }
  };

  const handleReload = async () => {
    setReloading(true);
    try {
      await api.reloadScripts();
      loadData();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to reload scripts');
    } finally {
      setReloading(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-400">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Lua Scripts</h1>
          <p className="text-gray-400">Server-side Lua modules and RPCs</p>
        </div>
        <div className="flex gap-3">
          <button onClick={loadData} className="btn btn-secondary">
            Refresh
          </button>
          <button onClick={handleReload} disabled={reloading} className="btn btn-primary">
            {reloading ? 'Reloading...' : 'Reload Scripts'}
          </button>
        </div>
      </div>

      {error && (
        <div className="bg-red-900/50 border border-red-700 text-red-300 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Scripts */}
        <div className="card">
          <h2 className="text-lg font-semibold text-white mb-4">Loaded Scripts</h2>
          {scripts.length === 0 ? (
            <p className="text-gray-400">No scripts loaded</p>
          ) : (
            <div className="space-y-3">
              {scripts.map((script) => (
                <div
                  key={script.name}
                  className="flex items-center justify-between p-3 bg-gray-700/50 rounded-lg"
                >
                  <div>
                    <p className="font-medium text-white">{script.name}</p>
                    <p className="text-xs text-gray-400">{script.path}</p>
                  </div>
                  <span className={`badge ${script.loaded ? 'badge-success' : 'badge-danger'}`}>
                    {script.loaded ? 'Loaded' : 'Not Loaded'}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* RPCs */}
        <div className="card">
          <h2 className="text-lg font-semibold text-white mb-4">Registered RPCs</h2>
          {rpcs.length === 0 ? (
            <p className="text-gray-400">No RPCs registered</p>
          ) : (
            <div className="space-y-3">
              {rpcs.map((rpc) => (
                <div
                  key={rpc.name}
                  className="flex items-center justify-between p-3 bg-gray-700/50 rounded-lg"
                >
                  <div>
                    <p className="font-medium text-white">{rpc.name}</p>
                    <p className="text-xs text-gray-400">Module: {rpc.module}</p>
                  </div>
                  <button className="btn btn-secondary btn-sm">
                    Execute
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Lua Info */}
      <div className="card">
        <h2 className="text-lg font-semibold text-white mb-4">Lua Runtime</h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="stat-card">
            <span className="stat-value">{scripts.length}</span>
            <span className="stat-label">Scripts</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">{rpcs.length}</span>
            <span className="stat-label">RPCs</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">Lua 5.4</span>
            <span className="stat-label">Version</span>
          </div>
          <div className="stat-card">
            <span className="stat-value">Sandboxed</span>
            <span className="stat-label">Mode</span>
          </div>
        </div>
      </div>
    </div>
  );
}
