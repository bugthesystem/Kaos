import { useEffect, useState } from 'react';
import { api } from '../api/client';
import { Badge } from '../components/DataTable';
import type { LuaScriptInfo, RpcInfo } from '../api/types';

export function LuaPage() {
  const [scripts, setScripts] = useState<LuaScriptInfo[]>([]);
  const [rpcs, setRpcs] = useState<RpcInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [reloading, setReloading] = useState(false);
  const [executeModalOpen, setExecuteModalOpen] = useState(false);
  const [selectedRpc, setSelectedRpc] = useState<RpcInfo | null>(null);
  const [rpcPayload, setRpcPayload] = useState('{}');
  const [rpcResult, setRpcResult] = useState<string | null>(null);
  const [executing, setExecuting] = useState(false);

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

  const handleExecuteRpc = async () => {
    if (!selectedRpc) return;
    setExecuting(true);
    setRpcResult(null);
    try {
      const payload = JSON.parse(rpcPayload);
      const result = await api.executeRpc(selectedRpc.name, payload);
      setRpcResult(JSON.stringify(result, null, 2));
    } catch (err) {
      setRpcResult(`Error: ${err instanceof Error ? err.message : 'Failed to execute RPC'}`);
    } finally {
      setExecuting(false);
    }
  };

  const openExecuteModal = (rpc: RpcInfo) => {
    setSelectedRpc(rpc);
    setRpcPayload('{}');
    setRpcResult(null);
    setExecuteModalOpen(true);
  };

  const loadedScripts = scripts.filter(s => s.loaded).length;

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title">Lua Scripts</h1>
          <p className="page-subtitle">
            Server-side Lua modules and RPCs
          </p>
        </div>
        <div className="flex gap-2">
          <button onClick={loadData} className="btn btn-secondary">
            Refresh
          </button>
          <button onClick={handleReload} disabled={reloading} className="btn btn-primary">
            {reloading ? 'Reloading...' : 'Reload Scripts'}
          </button>
        </div>
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
            <ScriptIcon className="w-6 h-6" style={{ color: 'var(--color-accent)' }} />
          </div>
          <span className="stat-value">{scripts.length}</span>
          <span className="stat-label">Total Scripts</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <LoadedIcon className="w-6 h-6" style={{ color: 'var(--color-success)' }} />
          </div>
          <span className="stat-value">{loadedScripts}</span>
          <span className="stat-label">Loaded</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <RpcIcon className="w-6 h-6" style={{ color: 'var(--color-info)' }} />
          </div>
          <span className="stat-value">{rpcs.length}</span>
          <span className="stat-label">RPCs</span>
        </div>
        <div className="stat-card">
          <div className="stat-icon">
            <VersionIcon className="w-6 h-6" style={{ color: 'var(--color-warning)' }} />
          </div>
          <span className="stat-value">5.4</span>
          <span className="stat-label">Lua Version</span>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-64">
          <div style={{ color: 'var(--text-muted)' }}>Loading...</div>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Scripts */}
          <div className="card">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>
                Loaded Scripts
              </h2>
              <span className="text-sm" style={{ color: 'var(--text-muted)' }}>
                {scripts.length} total
              </span>
            </div>
            {scripts.length === 0 ? (
              <p style={{ color: 'var(--text-muted)' }}>No scripts loaded</p>
            ) : (
              <div className="space-y-3 max-h-96 overflow-y-auto">
                {scripts.map((script) => (
                  <div
                    key={script.name}
                    className="flex items-center justify-between p-3 rounded-lg"
                    style={{ background: 'var(--bg-tertiary)' }}
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="w-9 h-9 rounded-lg flex items-center justify-center"
                        style={{
                          background: script.loaded
                            ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
                            : 'linear-gradient(135deg, #6b7280 0%, #4b5563 100%)',
                          color: 'white',
                        }}
                      >
                        <ScriptIcon className="w-5 h-5" />
                      </div>
                      <div>
                        <p className="font-medium" style={{ color: 'var(--text-primary)' }}>
                          {script.name}
                        </p>
                        <p className="text-xs font-mono" style={{ color: 'var(--text-muted)' }}>
                          {script.path}
                        </p>
                      </div>
                    </div>
                    <Badge variant={script.loaded ? 'success' : 'danger'}>
                      {script.loaded ? 'Loaded' : 'Failed'}
                    </Badge>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* RPCs */}
          <div className="card">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>
                Registered RPCs
              </h2>
              <span className="text-sm" style={{ color: 'var(--text-muted)' }}>
                {rpcs.length} total
              </span>
            </div>
            {rpcs.length === 0 ? (
              <p style={{ color: 'var(--text-muted)' }}>No RPCs registered</p>
            ) : (
              <div className="space-y-3 max-h-96 overflow-y-auto">
                {rpcs.map((rpc) => (
                  <div
                    key={rpc.name}
                    className="flex items-center justify-between p-3 rounded-lg"
                    style={{ background: 'var(--bg-tertiary)' }}
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className="w-9 h-9 rounded-lg flex items-center justify-center"
                        style={{
                          background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                          color: 'white',
                        }}
                      >
                        <RpcIcon className="w-5 h-5" />
                      </div>
                      <div>
                        <p className="font-medium" style={{ color: 'var(--text-primary)' }}>
                          {rpc.name}
                        </p>
                        <p className="text-xs" style={{ color: 'var(--text-muted)' }}>
                          Module: {rpc.module}
                        </p>
                      </div>
                    </div>
                    <button
                      onClick={() => openExecuteModal(rpc)}
                      className="btn btn-secondary btn-sm"
                    >
                      Execute
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Execute RPC Modal */}
      {executeModalOpen && selectedRpc && (
        <div className="modal-overlay">
          <div className="modal" style={{ maxWidth: '600px' }}>
            <h2 className="modal-title">Execute RPC: {selectedRpc.name}</h2>
            <div className="space-y-4">
              <div>
                <label className="form-label">Module</label>
                <input
                  type="text"
                  value={selectedRpc.module}
                  disabled
                  className="form-input"
                  style={{ opacity: 0.7 }}
                />
              </div>
              <div>
                <label className="form-label">Payload (JSON)</label>
                <textarea
                  value={rpcPayload}
                  onChange={(e) => setRpcPayload(e.target.value)}
                  className="form-input font-mono"
                  rows={5}
                  placeholder="{}"
                />
              </div>
              {rpcResult && (
                <div>
                  <label className="form-label">Result</label>
                  <pre
                    className="p-3 rounded-lg text-sm font-mono overflow-x-auto max-h-48"
                    style={{
                      background: 'var(--bg-tertiary)',
                      color: rpcResult.startsWith('Error') ? 'var(--color-danger)' : 'var(--text-primary)',
                    }}
                  >
                    {rpcResult}
                  </pre>
                </div>
              )}
            </div>
            <div className="flex justify-end gap-2 mt-6">
              <button
                onClick={() => {
                  setExecuteModalOpen(false);
                  setSelectedRpc(null);
                  setRpcResult(null);
                }}
                className="btn btn-secondary"
              >
                Close
              </button>
              <button
                onClick={handleExecuteRpc}
                disabled={executing}
                className="btn btn-primary"
              >
                {executing ? 'Executing...' : 'Execute'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// Icons
function ScriptIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
    </svg>
  );
}

function LoadedIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function RpcIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
    </svg>
  );
}

function VersionIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
    </svg>
  );
}

export default LuaPage;
