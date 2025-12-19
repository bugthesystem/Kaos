import { useEffect, useState, useCallback, useRef } from 'react';
import { api } from '../api/client';
import { Badge } from '../components/DataTable';
import type { LuaScriptInfo, RpcInfo } from '../api/types';

type TabType = 'scripts' | 'rpcs' | 'editor';

interface ExecutionHistory {
  id: string;
  rpc: string;
  payload: string;
  result: string;
  success: boolean;
  duration: number;
  timestamp: Date;
}

export function LuaPage() {
  const [scripts, setScripts] = useState<LuaScriptInfo[]>([]);
  const [rpcs, setRpcs] = useState<RpcInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [reloading, setReloading] = useState(false);
  const [activeTab, setActiveTab] = useState<TabType>('scripts');

  // Script viewer state
  const [selectedScript, setSelectedScript] = useState<LuaScriptInfo | null>(null);
  const [scriptContent, setScriptContent] = useState<string>('');
  const [loadingContent, setLoadingContent] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // RPC execution state
  const [selectedRpc, setSelectedRpc] = useState<RpcInfo | null>(null);
  const [rpcPayload, setRpcPayload] = useState('{\n  \n}');
  const [rpcResult, setRpcResult] = useState<string | null>(null);
  const [executing, setExecuting] = useState(false);
  const [executionHistory, setExecutionHistory] = useState<ExecutionHistory[]>([]);

  // Editor features
  const [lineNumbers, setLineNumbers] = useState(true);
  const [wordWrap, setWordWrap] = useState(false);
  const [fontSize, setFontSize] = useState(13);
  const editorRef = useRef<HTMLTextAreaElement>(null);

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
      await loadData();
      // Show success feedback
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reload scripts');
    } finally {
      setReloading(false);
    }
  };

  const loadScriptContent = useCallback(async (script: LuaScriptInfo) => {
    setSelectedScript(script);
    setLoadingContent(true);
    setActiveTab('editor');
    try {
      const data = await api.getScriptContent(script.name);
      setScriptContent(data.content);
    } catch (err) {
      // If API doesn't support content, show placeholder
      setScriptContent(`-- Script: ${script.name}\n-- Path: ${script.path}\n-- Size: ${formatBytes(script.size)}\n\n-- Content not available from API.\n-- Enable content endpoint in server to view script source.`);
    } finally {
      setLoadingContent(false);
    }
  }, []);

  const handleExecuteRpc = async () => {
    if (!selectedRpc) return;
    setExecuting(true);
    setRpcResult(null);

    const startTime = Date.now();
    try {
      const payload = JSON.parse(rpcPayload);
      const result = await api.executeRpc(selectedRpc.name, payload);
      const duration = Date.now() - startTime;
      const resultStr = JSON.stringify(result, null, 2);
      setRpcResult(resultStr);

      // Add to history
      setExecutionHistory(prev => [{
        id: crypto.randomUUID(),
        rpc: selectedRpc.name,
        payload: rpcPayload,
        result: resultStr,
        success: true,
        duration: result.duration_ms || duration,
        timestamp: new Date(),
      }, ...prev.slice(0, 19)]);
    } catch (err) {
      const duration = Date.now() - startTime;
      const errorStr = `Error: ${err instanceof Error ? err.message : 'Failed to execute RPC'}`;
      setRpcResult(errorStr);

      setExecutionHistory(prev => [{
        id: crypto.randomUUID(),
        rpc: selectedRpc.name,
        payload: rpcPayload,
        result: errorStr,
        success: false,
        duration,
        timestamp: new Date(),
      }, ...prev.slice(0, 19)]);
    } finally {
      setExecuting(false);
    }
  };

  const filteredScripts = scripts.filter(s =>
    s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    s.path.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const filteredRpcs = rpcs.filter(r =>
    r.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    r.module.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const loadedScripts = scripts.filter(s => s.loaded).length;
  const failedScripts = scripts.filter(s => !s.loaded).length;

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Tab key inserts spaces
    if (e.key === 'Tab') {
      e.preventDefault();
      const target = e.target as HTMLTextAreaElement;
      const start = target.selectionStart;
      const end = target.selectionEnd;
      const value = target.value;
      target.value = value.substring(0, start) + '  ' + value.substring(end);
      target.selectionStart = target.selectionEnd = start + 2;
      setRpcPayload(target.value);
    }
    // Cmd/Ctrl + Enter to execute
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      handleExecuteRpc();
    }
  };

  return (
    <div className="space-y-6 animate-fade-in">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div className="page-header" style={{ marginBottom: 0 }}>
          <h1 className="page-title flex items-center gap-3">
            <LuaLogo className="w-8 h-8" />
            Lua Runtime
          </h1>
          <p className="page-subtitle">
            Server-side scripting, RPCs, and hooks
          </p>
        </div>
        <div className="flex gap-2">
          <button onClick={loadData} className="btn btn-secondary" disabled={loading}>
            <RefreshIcon className="w-4 h-4" />
            Refresh
          </button>
          <button onClick={handleReload} disabled={reloading} className="btn btn-primary">
            {reloading ? (
              <>
                <SpinnerIcon className="w-4 h-4 animate-spin" />
                Reloading...
              </>
            ) : (
              <>
                <ReloadIcon className="w-4 h-4" />
                Hot Reload
              </>
            )}
          </button>
        </div>
      </div>

      {error && (
        <div className="alert alert-danger flex items-center gap-2">
          <AlertIcon className="w-5 h-5 flex-shrink-0" />
          {error}
        </div>
      )}

      {/* Stats Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <StatCard
          icon={<ScriptIcon className="w-6 h-6" />}
          value={scripts.length}
          label="Total Scripts"
          color="var(--color-accent)"
        />
        <StatCard
          icon={<LoadedIcon className="w-6 h-6" />}
          value={loadedScripts}
          label="Loaded"
          color="var(--color-success)"
          subtext={failedScripts > 0 ? `${failedScripts} failed` : undefined}
        />
        <StatCard
          icon={<RpcIcon className="w-6 h-6" />}
          value={rpcs.length}
          label="RPCs"
          color="var(--color-info)"
        />
        <StatCard
          icon={<HistoryIcon className="w-6 h-6" />}
          value={executionHistory.length}
          label="Executions"
          color="var(--color-warning)"
          subtext="this session"
        />
      </div>

      {/* Tab Navigation */}
      <div className="flex items-center gap-4 border-b" style={{ borderColor: 'var(--border-primary)' }}>
        <TabButton active={activeTab === 'scripts'} onClick={() => setActiveTab('scripts')}>
          <ScriptIcon className="w-4 h-4" />
          Scripts
          <span className="tab-count">{scripts.length}</span>
        </TabButton>
        <TabButton active={activeTab === 'rpcs'} onClick={() => setActiveTab('rpcs')}>
          <RpcIcon className="w-4 h-4" />
          RPCs
          <span className="tab-count">{rpcs.length}</span>
        </TabButton>
        <TabButton active={activeTab === 'editor'} onClick={() => setActiveTab('editor')}>
          <CodeIcon className="w-4 h-4" />
          Editor
          {selectedScript && <span className="tab-badge">{selectedScript.name}</span>}
        </TabButton>

        {/* Search - right aligned */}
        <div className="ml-auto pb-2">
          <div className="relative">
            <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4" style={{ color: 'var(--text-muted)' }} />
            <input
              type="text"
              placeholder="Search scripts or RPCs..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="form-input pl-9 py-1.5 text-sm"
              style={{ width: '240px' }}
            />
          </div>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-64">
          <div className="flex items-center gap-3" style={{ color: 'var(--text-muted)' }}>
            <SpinnerIcon className="w-5 h-5 animate-spin" />
            Loading Lua runtime...
          </div>
        </div>
      ) : (
        <>
          {/* Scripts Tab */}
          {activeTab === 'scripts' && (
            <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
              {filteredScripts.length === 0 ? (
                <div className="col-span-full text-center py-12" style={{ color: 'var(--text-muted)' }}>
                  {searchQuery ? 'No scripts match your search' : 'No scripts loaded'}
                </div>
              ) : (
                filteredScripts.map((script) => (
                  <ScriptCard
                    key={script.name}
                    script={script}
                    onView={() => loadScriptContent(script)}
                  />
                ))
              )}
            </div>
          )}

          {/* RPCs Tab */}
          {activeTab === 'rpcs' && (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              {/* RPC List */}
              <div className="card">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>
                    Registered RPCs
                  </h2>
                </div>
                {filteredRpcs.length === 0 ? (
                  <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>
                    {searchQuery ? 'No RPCs match your search' : 'No RPCs registered'}
                  </div>
                ) : (
                  <div className="space-y-2 max-h-[500px] overflow-y-auto">
                    {filteredRpcs.map((rpc) => (
                      <button
                        key={rpc.name}
                        onClick={() => {
                          setSelectedRpc(rpc);
                          setRpcResult(null);
                        }}
                        className={`w-full flex items-center gap-3 p-3 rounded-lg transition-all text-left ${
                          selectedRpc?.name === rpc.name ? 'ring-2 ring-cyan-500/50' : ''
                        }`}
                        style={{
                          background: selectedRpc?.name === rpc.name
                            ? 'var(--bg-hover)'
                            : 'var(--bg-tertiary)',
                        }}
                      >
                        <div
                          className="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0"
                          style={{
                            background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)',
                            color: 'white',
                          }}
                        >
                          <RpcIcon className="w-5 h-5" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="font-medium truncate" style={{ color: 'var(--text-primary)' }}>
                            {rpc.name}
                          </p>
                          <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>
                            {rpc.module}
                          </p>
                        </div>
                        <ChevronRightIcon className="w-4 h-4 flex-shrink-0" style={{ color: 'var(--text-muted)' }} />
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* RPC Executor */}
              <div className="card">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>
                    {selectedRpc ? `Execute: ${selectedRpc.name}` : 'RPC Executor'}
                  </h2>
                  {selectedRpc && (
                    <Badge variant="info">{selectedRpc.module}</Badge>
                  )}
                </div>

                {!selectedRpc ? (
                  <div className="flex flex-col items-center justify-center py-12 text-center" style={{ color: 'var(--text-muted)' }}>
                    <RpcIcon className="w-12 h-12 mb-4 opacity-30" />
                    <p>Select an RPC from the list to execute</p>
                  </div>
                ) : (
                  <div className="space-y-4">
                    <div>
                      <label className="form-label flex items-center justify-between">
                        <span>Payload (JSON)</span>
                        <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
                          Cmd+Enter to execute
                        </span>
                      </label>
                      <div className="relative">
                        <textarea
                          ref={editorRef}
                          value={rpcPayload}
                          onChange={(e) => setRpcPayload(e.target.value)}
                          onKeyDown={handleKeyDown}
                          className="form-input font-mono"
                          rows={6}
                          placeholder="{}"
                          style={{ fontSize: `${fontSize}px` }}
                        />
                      </div>
                    </div>

                    <button
                      onClick={handleExecuteRpc}
                      disabled={executing}
                      className="btn btn-primary w-full flex items-center justify-center gap-2"
                    >
                      {executing ? (
                        <>
                          <SpinnerIcon className="w-4 h-4 animate-spin" />
                          Executing...
                        </>
                      ) : (
                        <>
                          <PlayIcon className="w-4 h-4" />
                          Execute RPC
                        </>
                      )}
                    </button>

                    {rpcResult && (
                      <div>
                        <label className="form-label flex items-center gap-2">
                          Result
                          {!rpcResult.startsWith('Error') && (
                            <Badge variant="success">Success</Badge>
                          )}
                          {rpcResult.startsWith('Error') && (
                            <Badge variant="danger">Error</Badge>
                          )}
                        </label>
                        <pre
                          className="p-4 rounded-lg text-sm font-mono overflow-auto max-h-64"
                          style={{
                            background: 'var(--bg-tertiary)',
                            color: rpcResult.startsWith('Error') ? 'var(--color-danger)' : 'var(--text-primary)',
                            fontSize: `${fontSize}px`,
                          }}
                        >
                          {rpcResult}
                        </pre>
                      </div>
                    )}
                  </div>
                )}

                {/* Execution History */}
                {executionHistory.length > 0 && (
                  <div className="mt-6 pt-6" style={{ borderTop: '1px solid var(--border-primary)' }}>
                    <h3 className="text-sm font-medium mb-3" style={{ color: 'var(--text-secondary)' }}>
                      Recent Executions
                    </h3>
                    <div className="space-y-2 max-h-48 overflow-y-auto">
                      {executionHistory.slice(0, 5).map((exec) => (
                        <div
                          key={exec.id}
                          className="flex items-center gap-3 p-2 rounded text-sm"
                          style={{ background: 'var(--bg-tertiary)' }}
                        >
                          <div
                            className="w-2 h-2 rounded-full flex-shrink-0"
                            style={{ background: exec.success ? 'var(--color-success)' : 'var(--color-danger)' }}
                          />
                          <span className="font-medium truncate" style={{ color: 'var(--text-primary)' }}>
                            {exec.rpc}
                          </span>
                          <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
                            {exec.duration}ms
                          </span>
                          <span className="ml-auto text-xs" style={{ color: 'var(--text-muted)' }}>
                            {formatTime(exec.timestamp)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Editor Tab */}
          {activeTab === 'editor' && (
            <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
              {/* Editor Toolbar */}
              <div
                className="flex items-center justify-between px-4 py-2"
                style={{
                  background: 'var(--bg-tertiary)',
                  borderBottom: '1px solid var(--border-primary)',
                }}
              >
                <div className="flex items-center gap-4">
                  {selectedScript ? (
                    <>
                      <div className="flex items-center gap-2">
                        <FileIcon className="w-4 h-4" style={{ color: 'var(--color-accent)' }} />
                        <span className="font-medium" style={{ color: 'var(--text-primary)' }}>
                          {selectedScript.name}
                        </span>
                      </div>
                      <span className="text-xs" style={{ color: 'var(--text-muted)' }}>
                        {selectedScript.path}
                      </span>
                      <Badge variant={selectedScript.loaded ? 'success' : 'danger'}>
                        {selectedScript.loaded ? 'Loaded' : 'Failed'}
                      </Badge>
                    </>
                  ) : (
                    <span style={{ color: 'var(--text-muted)' }}>No script selected</span>
                  )}
                </div>
                <div className="flex items-center gap-3">
                  <button
                    onClick={() => setLineNumbers(!lineNumbers)}
                    className={`p-1.5 rounded transition-colors ${lineNumbers ? 'bg-cyan-500/20' : ''}`}
                    title="Toggle line numbers"
                  >
                    <HashIcon className="w-4 h-4" style={{ color: lineNumbers ? 'var(--color-accent)' : 'var(--text-muted)' }} />
                  </button>
                  <button
                    onClick={() => setWordWrap(!wordWrap)}
                    className={`p-1.5 rounded transition-colors ${wordWrap ? 'bg-cyan-500/20' : ''}`}
                    title="Toggle word wrap"
                  >
                    <WrapIcon className="w-4 h-4" style={{ color: wordWrap ? 'var(--color-accent)' : 'var(--text-muted)' }} />
                  </button>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => setFontSize(Math.max(10, fontSize - 1))}
                      className="p-1 rounded hover:bg-white/10"
                      title="Decrease font size"
                    >
                      <MinusIcon className="w-3 h-3" style={{ color: 'var(--text-muted)' }} />
                    </button>
                    <span className="text-xs w-8 text-center" style={{ color: 'var(--text-muted)' }}>
                      {fontSize}px
                    </span>
                    <button
                      onClick={() => setFontSize(Math.min(24, fontSize + 1))}
                      className="p-1 rounded hover:bg-white/10"
                      title="Increase font size"
                    >
                      <PlusIcon className="w-3 h-3" style={{ color: 'var(--text-muted)' }} />
                    </button>
                  </div>
                </div>
              </div>

              {/* Editor Content */}
              {loadingContent ? (
                <div className="flex items-center justify-center h-96">
                  <SpinnerIcon className="w-6 h-6 animate-spin" style={{ color: 'var(--text-muted)' }} />
                </div>
              ) : selectedScript ? (
                <div className="relative overflow-auto" style={{ maxHeight: 'calc(100vh - 400px)', minHeight: '400px' }}>
                  <CodeViewer
                    content={scriptContent}
                    showLineNumbers={lineNumbers}
                    wordWrap={wordWrap}
                    fontSize={fontSize}
                  />
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center h-96" style={{ color: 'var(--text-muted)' }}>
                  <CodeIcon className="w-16 h-16 mb-4 opacity-20" />
                  <p className="text-lg mb-2">No script selected</p>
                  <p className="text-sm">Select a script from the Scripts tab to view its source</p>
                </div>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}

// Components

function StatCard({ icon, value, label, color, subtext }: {
  icon: React.ReactNode;
  value: number;
  label: string;
  color: string;
  subtext?: string;
}) {
  return (
    <div className="stat-card">
      <div className="stat-icon" style={{ color }}>
        {icon}
      </div>
      <span className="stat-value">{value}</span>
      <span className="stat-label">{label}</span>
      {subtext && (
        <span className="text-xs mt-1" style={{ color: 'var(--text-muted)' }}>{subtext}</span>
      )}
    </div>
  );
}

function TabButton({ children, active, onClick }: {
  children: React.ReactNode;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 px-4 py-3 text-sm font-medium transition-colors relative ${
        active ? '' : 'hover:text-cyan-400'
      }`}
      style={{
        color: active ? 'var(--color-accent)' : 'var(--text-secondary)',
      }}
    >
      {children}
      {active && (
        <div
          className="absolute bottom-0 left-0 right-0 h-0.5"
          style={{ background: 'var(--color-accent)' }}
        />
      )}
    </button>
  );
}

function ScriptCard({ script, onView }: { script: LuaScriptInfo; onView: () => void }) {
  return (
    <div
      className="card p-4 hover:ring-1 hover:ring-cyan-500/30 transition-all cursor-pointer group"
      onClick={onView}
    >
      <div className="flex items-start gap-3">
        <div
          className="w-12 h-12 rounded-xl flex items-center justify-center flex-shrink-0 transition-transform group-hover:scale-105"
          style={{
            background: script.loaded
              ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)'
              : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)',
            color: 'white',
          }}
        >
          <ScriptIcon className="w-6 h-6" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h3 className="font-semibold truncate" style={{ color: 'var(--text-primary)' }}>
              {script.name}
            </h3>
            <Badge variant={script.loaded ? 'success' : 'danger'}>
              {script.loaded ? 'OK' : 'Error'}
            </Badge>
          </div>
          <p className="text-xs font-mono truncate mt-1" style={{ color: 'var(--text-muted)' }}>
            {script.path}
          </p>
          <div className="flex items-center gap-3 mt-2 text-xs" style={{ color: 'var(--text-muted)' }}>
            <span>{formatBytes(script.size)}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function CodeViewer({ content, showLineNumbers, wordWrap, fontSize }: {
  content: string;
  showLineNumbers: boolean;
  wordWrap: boolean;
  fontSize: number;
}) {
  const lines = content.split('\n');

  return (
    <div className="flex font-mono" style={{ fontSize: `${fontSize}px` }}>
      {showLineNumbers && (
        <div
          className="select-none text-right pr-4 py-4 flex-shrink-0"
          style={{
            color: 'var(--text-muted)',
            background: 'var(--bg-tertiary)',
            borderRight: '1px solid var(--border-primary)',
            minWidth: `${Math.max(3, lines.length.toString().length + 1)}ch`,
          }}
        >
          {lines.map((_, i) => (
            <div key={i} className="leading-6">{i + 1}</div>
          ))}
        </div>
      )}
      <pre
        className="flex-1 p-4 m-0 overflow-x-auto"
        style={{
          whiteSpace: wordWrap ? 'pre-wrap' : 'pre',
          wordBreak: wordWrap ? 'break-all' : 'normal',
          color: 'var(--text-primary)',
          lineHeight: '1.5',
        }}
      >
        {highlightLua(content)}
      </pre>
    </div>
  );
}

// Lua syntax highlighting (basic)
function highlightLua(code: string): React.ReactNode {
  // Keywords
  const keywords = /\b(local|function|end|if|then|else|elseif|for|while|do|repeat|until|return|break|in|and|or|not|nil|true|false|self)\b/g;
  // Strings
  const strings = /("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')/g;
  // Comments
  const comments = /(--\[\[[\s\S]*?\]\]|--.*$)/gm;
  // Numbers
  const numbers = /\b(\d+\.?\d*)\b/g;
  // Functions
  const functions = /\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/g;

  const parts: { text: string; type: string; index: number }[] = [];

  // Find all matches
  let match;

  // Comments (highest priority)
  while ((match = comments.exec(code)) !== null) {
    parts.push({ text: match[0], type: 'comment', index: match.index });
  }

  // Strings
  while ((match = strings.exec(code)) !== null) {
    parts.push({ text: match[0], type: 'string', index: match.index });
  }

  // Keywords
  while ((match = keywords.exec(code)) !== null) {
    parts.push({ text: match[0], type: 'keyword', index: match.index });
  }

  // Numbers
  while ((match = numbers.exec(code)) !== null) {
    parts.push({ text: match[0], type: 'number', index: match.index });
  }

  // Functions
  while ((match = functions.exec(code)) !== null) {
    parts.push({ text: match[1], type: 'function', index: match.index });
  }

  // Sort by index
  parts.sort((a, b) => a.index - b.index);

  // Build result with colors
  const result: React.ReactNode[] = [];
  let lastIndex = 0;

  const colors: Record<string, string> = {
    keyword: '#c678dd',
    string: '#98c379',
    comment: '#5c6370',
    number: '#d19a66',
    function: '#61afef',
  };

  // Remove overlapping matches
  const filteredParts: typeof parts = [];
  for (const part of parts) {
    const overlaps = filteredParts.some(
      p => part.index >= p.index && part.index < p.index + p.text.length
    );
    if (!overlaps) {
      filteredParts.push(part);
    }
  }

  for (const part of filteredParts) {
    // Add text before this match
    if (part.index > lastIndex) {
      result.push(code.substring(lastIndex, part.index));
    }

    // Add highlighted match
    result.push(
      <span key={part.index} style={{ color: colors[part.type] }}>
        {part.text}
      </span>
    );

    lastIndex = part.index + part.text.length;
  }

  // Add remaining text
  if (lastIndex < code.length) {
    result.push(code.substring(lastIndex));
  }

  return result.length > 0 ? result : code;
}

// Utilities
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatTime(date: Date): string {
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

// Icons
function LuaLogo({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <circle cx="12" cy="12" r="10" fill="url(#lua-gradient)" />
      <circle cx="16" cy="8" r="2.5" fill="white" />
      <defs>
        <linearGradient id="lua-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#000080" />
          <stop offset="100%" stopColor="#00008B" />
        </linearGradient>
      </defs>
    </svg>
  );
}

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

function HistoryIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function RefreshIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
    </svg>
  );
}

function ReloadIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
    </svg>
  );
}

function SpinnerIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
    </svg>
  );
}

function AlertIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function SearchIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
  );
}

function CodeIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
    </svg>
  );
}

function ChevronRightIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
    </svg>
  );
}

function PlayIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function FileIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
    </svg>
  );
}

function HashIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 20l4-16m2 16l4-16M6 9h14M4 15h14" />
    </svg>
  );
}

function WrapIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h10m-4 6h10M4 18h4" />
    </svg>
  );
}

function MinusIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 12H4" />
    </svg>
  );
}

function PlusIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
    </svg>
  );
}

export default LuaPage;
