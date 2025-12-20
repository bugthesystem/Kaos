import { useEffect, useState, useCallback, useRef } from 'react';
import { api } from '../api/client';
import { Badge } from '../components/DataTable';
import { useAuth } from '../contexts/AuthContext';
import { PageHeader, StatCard, StatGrid, Alert, Spinner } from '../components/ui';
import { TerminalIcon, CheckIcon, CodeIcon, ClockIcon, RefreshIcon, SearchIcon, PlayIcon, PlusIcon } from '../components/icons';
import { formatBytes } from '../utils/formatters';
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
  const { hasPermission } = useAuth();
  const canReload = hasPermission('reload:scripts');
  const canExecuteRpc = hasPermission('execute:rpc');

  const [scripts, setScripts] = useState<LuaScriptInfo[]>([]);
  const [rpcs, setRpcs] = useState<RpcInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [reloading, setReloading] = useState(false);
  const [activeTab, setActiveTab] = useState<TabType>('scripts');
  const [selectedScript, setSelectedScript] = useState<LuaScriptInfo | null>(null);
  const [scriptContent, setScriptContent] = useState<string>('');
  const [loadingContent, setLoadingContent] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedRpc, setSelectedRpc] = useState<RpcInfo | null>(null);
  const [rpcPayload, setRpcPayload] = useState('{\n  \n}');
  const [rpcResult, setRpcResult] = useState<string | null>(null);
  const [executing, setExecuting] = useState(false);
  const [executionHistory, setExecutionHistory] = useState<ExecutionHistory[]>([]);
  const [lineNumbers, setLineNumbers] = useState(true);
  const [wordWrap, setWordWrap] = useState(false);
  const [fontSize, setFontSize] = useState(13);
  const editorRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => { loadData(); }, []);

  const loadData = async () => {
    setLoading(true);
    try {
      const [scriptsData, rpcsData] = await Promise.all([api.listScripts(), api.listRpcs()]);
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
    } catch {
      setScriptContent(`-- Script: ${script.name}\n-- Path: ${script.path}\n-- Size: ${formatBytes(script.size)}\n\n-- Content not available from API.`);
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
      setExecutionHistory(prev => [{ id: crypto.randomUUID(), rpc: selectedRpc.name, payload: rpcPayload, result: resultStr, success: true, duration: result.duration_ms || duration, timestamp: new Date() }, ...prev.slice(0, 19)]);
    } catch (err) {
      const duration = Date.now() - startTime;
      const errorStr = `Error: ${err instanceof Error ? err.message : 'Failed to execute RPC'}`;
      setRpcResult(errorStr);
      setExecutionHistory(prev => [{ id: crypto.randomUUID(), rpc: selectedRpc.name, payload: rpcPayload, result: errorStr, success: false, duration, timestamp: new Date() }, ...prev.slice(0, 19)]);
    } finally {
      setExecuting(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const target = e.target as HTMLTextAreaElement;
      const start = target.selectionStart;
      const end = target.selectionEnd;
      target.value = target.value.substring(0, start) + '  ' + target.value.substring(end);
      target.selectionStart = target.selectionEnd = start + 2;
      setRpcPayload(target.value);
    }
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') handleExecuteRpc();
  };

  const filteredScripts = scripts.filter(s => s.name.toLowerCase().includes(searchQuery.toLowerCase()) || s.path.toLowerCase().includes(searchQuery.toLowerCase()));
  const filteredRpcs = rpcs.filter(r => r.name.toLowerCase().includes(searchQuery.toLowerCase()) || r.module.toLowerCase().includes(searchQuery.toLowerCase()));
  const loadedScripts = scripts.filter(s => s.loaded).length;
  const failedScripts = scripts.filter(s => !s.loaded).length;

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader title={<span className="flex items-center gap-3"><LuaLogo className="w-8 h-8" />Lua Runtime</span>} subtitle="Server-side scripting, RPCs, and hooks">
        <button onClick={loadData} className="btn btn-secondary" disabled={loading}><RefreshIcon className="w-4 h-4" /></button>
        {canReload && (
          <button onClick={handleReload} disabled={reloading} className="btn btn-primary">
            {reloading ? <><Spinner className="w-4 h-4" /> Reloading...</> : <><RefreshIcon className="w-4 h-4" /> Hot Reload</>}
          </button>
        )}
      </PageHeader>

      {error && <Alert variant="danger" onDismiss={() => setError('')}>{error}</Alert>}

      <StatGrid columns={4}>
        <StatCard icon={<TerminalIcon className="w-5 h-5" />} label="Total Scripts" value={scripts.length} color="primary" />
        <StatCard icon={<CheckIcon className="w-5 h-5" />} label="Loaded" value={loadedScripts} color="success" subtitle={failedScripts > 0 ? `${failedScripts} failed` : undefined} />
        <StatCard icon={<CodeIcon className="w-5 h-5" />} label="RPCs" value={rpcs.length} color="info" />
        <StatCard icon={<ClockIcon className="w-5 h-5" />} label="Executions" value={executionHistory.length} color="warning" subtitle="this session" />
      </StatGrid>

      <div className="flex items-center gap-4 border-b" style={{ borderColor: 'var(--border-primary)' }}>
        <TabButton active={activeTab === 'scripts'} onClick={() => setActiveTab('scripts')}><TerminalIcon className="w-4 h-4" />Scripts<span className="tab-count">{scripts.length}</span></TabButton>
        <TabButton active={activeTab === 'rpcs'} onClick={() => setActiveTab('rpcs')}><CodeIcon className="w-4 h-4" />RPCs<span className="tab-count">{rpcs.length}</span></TabButton>
        <TabButton active={activeTab === 'editor'} onClick={() => setActiveTab('editor')}><CodeIcon className="w-4 h-4" />Editor{selectedScript && <span className="tab-badge">{selectedScript.name}</span>}</TabButton>
        <div className="ml-auto pb-2">
          <div className="relative">
            <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4" style={{ color: 'var(--text-muted)' }} />
            <input type="text" placeholder="Search scripts or RPCs..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="form-input pl-9 py-1.5 text-sm" style={{ width: '240px' }} />
          </div>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-64"><Spinner /> <span className="ml-3" style={{ color: 'var(--text-muted)' }}>Loading Lua runtime...</span></div>
      ) : (
        <>
          {activeTab === 'scripts' && (
            <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
              {filteredScripts.length === 0 ? (
                <div className="col-span-full text-center py-12" style={{ color: 'var(--text-muted)' }}>{searchQuery ? 'No scripts match your search' : 'No scripts loaded'}</div>
              ) : filteredScripts.map((script) => <ScriptCard key={script.name} script={script} onView={() => loadScriptContent(script)} />)}
            </div>
          )}

          {activeTab === 'rpcs' && (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="card">
                <h2 className="text-lg font-semibold mb-4" style={{ color: 'var(--text-primary)' }}>Registered RPCs</h2>
                {filteredRpcs.length === 0 ? (
                  <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>{searchQuery ? 'No RPCs match your search' : 'No RPCs registered'}</div>
                ) : (
                  <div className="space-y-2 max-h-[500px] overflow-y-auto">
                    {filteredRpcs.map((rpc) => (
                      <button key={rpc.name} onClick={() => { setSelectedRpc(rpc); setRpcResult(null); }} className={`w-full flex items-center gap-3 p-3 rounded-lg transition-all text-left ${selectedRpc?.name === rpc.name ? 'ring-2 ring-cyan-500/50' : ''}`} style={{ background: selectedRpc?.name === rpc.name ? 'var(--bg-hover)' : 'var(--bg-tertiary)' }}>
                        <div className="w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0" style={{ background: 'linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)', color: 'white' }}><CodeIcon className="w-5 h-5" /></div>
                        <div className="flex-1 min-w-0">
                          <p className="font-medium truncate" style={{ color: 'var(--text-primary)' }}>{rpc.name}</p>
                          <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>{rpc.module}</p>
                        </div>
                        <ChevronRightIcon className="w-4 h-4 flex-shrink-0" style={{ color: 'var(--text-muted)' }} />
                      </button>
                    ))}
                  </div>
                )}
              </div>

              <div className="card">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>{selectedRpc ? `Execute: ${selectedRpc.name}` : 'RPC Executor'}</h2>
                  {selectedRpc && <Badge variant="info">{selectedRpc.module}</Badge>}
                </div>
                {!selectedRpc ? (
                  <div className="flex flex-col items-center justify-center py-12 text-center" style={{ color: 'var(--text-muted)' }}><CodeIcon className="w-12 h-12 mb-4 opacity-30" /><p>Select an RPC from the list to execute</p></div>
                ) : (
                  <div className="space-y-4">
                    <div>
                      <label className="form-label flex items-center justify-between"><span>Payload (JSON)</span><span className="text-xs" style={{ color: 'var(--text-muted)' }}>Cmd+Enter to execute</span></label>
                      <textarea ref={editorRef} value={rpcPayload} onChange={(e) => setRpcPayload(e.target.value)} onKeyDown={handleKeyDown} className="form-input font-mono" rows={6} placeholder="{}" style={{ fontSize: `${fontSize}px` }} disabled={!canExecuteRpc} />
                    </div>
                    <button onClick={handleExecuteRpc} disabled={executing || !canExecuteRpc} className="btn btn-primary w-full flex items-center justify-center gap-2">
                      {!canExecuteRpc ? 'No permission to execute RPCs' : executing ? <><Spinner className="w-4 h-4" /> Executing...</> : <><PlayIcon className="w-4 h-4" /> Execute RPC</>}
                    </button>
                    {rpcResult && (
                      <div>
                        <label className="form-label flex items-center gap-2">Result {!rpcResult.startsWith('Error') && <Badge variant="success">Success</Badge>}{rpcResult.startsWith('Error') && <Badge variant="danger">Error</Badge>}</label>
                        <pre className="p-4 rounded-lg text-sm font-mono overflow-auto max-h-64" style={{ background: 'var(--bg-tertiary)', color: rpcResult.startsWith('Error') ? 'var(--color-danger)' : 'var(--text-primary)', fontSize: `${fontSize}px` }}>{rpcResult}</pre>
                      </div>
                    )}
                  </div>
                )}
                {executionHistory.length > 0 && (
                  <div className="mt-6 pt-6" style={{ borderTop: '1px solid var(--border-primary)' }}>
                    <h3 className="text-sm font-medium mb-3" style={{ color: 'var(--text-secondary)' }}>Recent Executions</h3>
                    <div className="space-y-2 max-h-48 overflow-y-auto">
                      {executionHistory.slice(0, 5).map((exec) => (
                        <div key={exec.id} className="flex items-center gap-3 p-2 rounded text-sm" style={{ background: 'var(--bg-tertiary)' }}>
                          <div className="w-2 h-2 rounded-full flex-shrink-0" style={{ background: exec.success ? 'var(--color-success)' : 'var(--color-danger)' }} />
                          <span className="font-medium truncate" style={{ color: 'var(--text-primary)' }}>{exec.rpc}</span>
                          <span className="text-xs" style={{ color: 'var(--text-muted)' }}>{exec.duration}ms</span>
                          <span className="ml-auto text-xs" style={{ color: 'var(--text-muted)' }}>{formatTime(exec.timestamp)}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {activeTab === 'editor' && (
            <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
              <div className="flex items-center justify-between px-4 py-2" style={{ background: 'var(--bg-tertiary)', borderBottom: '1px solid var(--border-primary)' }}>
                <div className="flex items-center gap-4">
                  {selectedScript ? (
                    <>
                      <div className="flex items-center gap-2"><FileIcon className="w-4 h-4" style={{ color: 'var(--color-accent)' }} /><span className="font-medium" style={{ color: 'var(--text-primary)' }}>{selectedScript.name}</span></div>
                      <span className="text-xs" style={{ color: 'var(--text-muted)' }}>{selectedScript.path}</span>
                      <Badge variant={selectedScript.loaded ? 'success' : 'danger'}>{selectedScript.loaded ? 'Loaded' : 'Failed'}</Badge>
                    </>
                  ) : <span style={{ color: 'var(--text-muted)' }}>No script selected</span>}
                </div>
                <div className="flex items-center gap-3">
                  <button onClick={() => setLineNumbers(!lineNumbers)} className={`p-1.5 rounded transition-colors ${lineNumbers ? 'bg-cyan-500/20' : ''}`} title="Toggle line numbers"><HashIcon className="w-4 h-4" style={{ color: lineNumbers ? 'var(--color-accent)' : 'var(--text-muted)' }} /></button>
                  <button onClick={() => setWordWrap(!wordWrap)} className={`p-1.5 rounded transition-colors ${wordWrap ? 'bg-cyan-500/20' : ''}`} title="Toggle word wrap"><WrapIcon className="w-4 h-4" style={{ color: wordWrap ? 'var(--color-accent)' : 'var(--text-muted)' }} /></button>
                  <div className="flex items-center gap-1">
                    <button onClick={() => setFontSize(Math.max(10, fontSize - 1))} className="p-1 rounded hover:bg-white/10" title="Decrease font size"><MinusIcon className="w-3 h-3" style={{ color: 'var(--text-muted)' }} /></button>
                    <span className="text-xs w-8 text-center" style={{ color: 'var(--text-muted)' }}>{fontSize}px</span>
                    <button onClick={() => setFontSize(Math.min(24, fontSize + 1))} className="p-1 rounded hover:bg-white/10" title="Increase font size"><PlusIcon className="w-3 h-3" style={{ color: 'var(--text-muted)' }} /></button>
                  </div>
                </div>
              </div>
              {loadingContent ? (
                <div className="flex items-center justify-center h-96"><Spinner /></div>
              ) : selectedScript ? (
                <div className="relative overflow-auto" style={{ maxHeight: 'calc(100vh - 400px)', minHeight: '400px' }}><CodeViewer content={scriptContent} showLineNumbers={lineNumbers} wordWrap={wordWrap} fontSize={fontSize} /></div>
              ) : (
                <div className="flex flex-col items-center justify-center h-96" style={{ color: 'var(--text-muted)' }}><CodeIcon className="w-16 h-16 mb-4 opacity-20" /><p className="text-lg mb-2">No script selected</p><p className="text-sm">Select a script from the Scripts tab to view its source</p></div>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}

function TabButton({ children, active, onClick }: { children: React.ReactNode; active: boolean; onClick: () => void }) {
  return (
    <button onClick={onClick} className={`flex items-center gap-2 px-4 py-3 text-sm font-medium transition-colors relative ${active ? '' : 'hover:text-cyan-400'}`} style={{ color: active ? 'var(--color-accent)' : 'var(--text-secondary)' }}>
      {children}
      {active && <div className="absolute bottom-0 left-0 right-0 h-0.5" style={{ background: 'var(--color-accent)' }} />}
    </button>
  );
}

function ScriptCard({ script, onView }: { script: LuaScriptInfo; onView: () => void }) {
  return (
    <div className="card p-4 hover:ring-1 hover:ring-cyan-500/30 transition-all cursor-pointer group" onClick={onView}>
      <div className="flex items-start gap-3">
        <div className="w-12 h-12 rounded-xl flex items-center justify-center flex-shrink-0 transition-transform group-hover:scale-105" style={{ background: script.loaded ? 'linear-gradient(135deg, #22c55e 0%, #16a34a 100%)' : 'linear-gradient(135deg, #ef4444 0%, #dc2626 100%)', color: 'white' }}><TerminalIcon className="w-6 h-6" /></div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2"><h3 className="font-semibold truncate" style={{ color: 'var(--text-primary)' }}>{script.name}</h3><Badge variant={script.loaded ? 'success' : 'danger'}>{script.loaded ? 'OK' : 'Error'}</Badge></div>
          <p className="text-xs font-mono truncate mt-1" style={{ color: 'var(--text-muted)' }}>{script.path}</p>
          <div className="flex items-center gap-3 mt-2 text-xs" style={{ color: 'var(--text-muted)' }}><span>{formatBytes(script.size)}</span></div>
        </div>
      </div>
    </div>
  );
}

function CodeViewer({ content, showLineNumbers, wordWrap, fontSize }: { content: string; showLineNumbers: boolean; wordWrap: boolean; fontSize: number }) {
  const lines = content.split('\n');
  return (
    <div className="flex font-mono" style={{ fontSize: `${fontSize}px` }}>
      {showLineNumbers && (
        <div className="select-none text-right pr-4 py-4 flex-shrink-0" style={{ color: 'var(--text-muted)', background: 'var(--bg-tertiary)', borderRight: '1px solid var(--border-primary)', minWidth: `${Math.max(3, lines.length.toString().length + 1)}ch` }}>
          {lines.map((_, i) => <div key={i} className="leading-6">{i + 1}</div>)}
        </div>
      )}
      <pre className="flex-1 p-4 m-0 overflow-x-auto" style={{ whiteSpace: wordWrap ? 'pre-wrap' : 'pre', wordBreak: wordWrap ? 'break-all' : 'normal', color: 'var(--text-primary)', lineHeight: '1.5' }}>{highlightLua(content)}</pre>
    </div>
  );
}

function highlightLua(code: string): React.ReactNode {
  const keywords = /\b(local|function|end|if|then|else|elseif|for|while|do|repeat|until|return|break|in|and|or|not|nil|true|false|self)\b/g;
  const strings = /("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')/g;
  const comments = /(--\[\[[\s\S]*?\]\]|--.*$)/gm;
  const numbers = /\b(\d+\.?\d*)\b/g;
  const functions = /\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/g;
  const parts: { text: string; type: string; index: number }[] = [];
  let match;
  while ((match = comments.exec(code)) !== null) parts.push({ text: match[0], type: 'comment', index: match.index });
  while ((match = strings.exec(code)) !== null) parts.push({ text: match[0], type: 'string', index: match.index });
  while ((match = keywords.exec(code)) !== null) parts.push({ text: match[0], type: 'keyword', index: match.index });
  while ((match = numbers.exec(code)) !== null) parts.push({ text: match[0], type: 'number', index: match.index });
  while ((match = functions.exec(code)) !== null) parts.push({ text: match[1], type: 'function', index: match.index });
  parts.sort((a, b) => a.index - b.index);
  const colors: Record<string, string> = { keyword: '#c678dd', string: '#98c379', comment: '#5c6370', number: '#d19a66', function: '#61afef' };
  const filteredParts: typeof parts = [];
  for (const part of parts) { if (!filteredParts.some(p => part.index >= p.index && part.index < p.index + p.text.length)) filteredParts.push(part); }
  const result: React.ReactNode[] = [];
  let lastIndex = 0;
  for (const part of filteredParts) {
    if (part.index > lastIndex) result.push(code.substring(lastIndex, part.index));
    result.push(<span key={part.index} style={{ color: colors[part.type] }}>{part.text}</span>);
    lastIndex = part.index + part.text.length;
  }
  if (lastIndex < code.length) result.push(code.substring(lastIndex));
  return result.length > 0 ? result : code;
}

function formatTime(date: Date): string { return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }); }

// Icons specific to Lua page
function LuaLogo({ className }: { className?: string }) {
  return (<svg className={className} viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="10" fill="url(#lua-gradient)" /><circle cx="16" cy="8" r="2.5" fill="white" /><defs><linearGradient id="lua-gradient" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stopColor="#000080" /><stop offset="100%" stopColor="#00008B" /></linearGradient></defs></svg>);
}
function FileIcon({ className, style }: { className?: string; style?: React.CSSProperties }) { return (<svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" /></svg>); }
function HashIcon({ className, style }: { className?: string; style?: React.CSSProperties }) { return (<svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 20l4-16m2 16l4-16M6 9h14M4 15h14" /></svg>); }
function WrapIcon({ className, style }: { className?: string; style?: React.CSSProperties }) { return (<svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h10m-4 6h10M4 18h4" /></svg>); }
function MinusIcon({ className, style }: { className?: string; style?: React.CSSProperties }) { return (<svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 12H4" /></svg>); }
function ChevronRightIcon({ className, style }: { className?: string; style?: React.CSSProperties }) { return (<svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" /></svg>); }

export default LuaPage;
