import { useState, useEffect, useCallback, useRef } from 'react';

interface Command {
  id: string;
  label: string;
  description?: string;
  icon?: React.ReactNode;
  shortcut?: string;
  section: string;
  action: () => void;
}

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigate: (page: string) => void;
}

export function CommandPalette({ isOpen, onClose, onNavigate }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const commands: Command[] = [
    // Navigation
    { id: 'nav-dashboard', label: 'Dashboard', description: 'Server overview', icon: <DashboardIcon />, section: 'Navigation', action: () => { onNavigate('dashboard'); onClose(); } },
    { id: 'nav-sessions', label: 'Sessions', description: 'Active connections', icon: <SessionsIcon />, shortcut: 'G S', section: 'Navigation', action: () => { onNavigate('sessions'); onClose(); } },
    { id: 'nav-rooms', label: 'Rooms', description: 'Game rooms', icon: <RoomsIcon />, shortcut: 'G R', section: 'Navigation', action: () => { onNavigate('rooms'); onClose(); } },
    { id: 'nav-lua', label: 'Lua Scripts', description: 'Server scripting', icon: <LuaIcon />, shortcut: 'G L', section: 'Navigation', action: () => { onNavigate('lua'); onClose(); } },
    { id: 'nav-players', label: 'Players', description: 'Registered players', icon: <PlayersIcon />, section: 'Navigation', action: () => { onNavigate('players'); onClose(); } },
    { id: 'nav-chat', label: 'Chat', description: 'Chat channels', icon: <ChatIcon />, section: 'Navigation', action: () => { onNavigate('chat'); onClose(); } },
    { id: 'nav-leaderboards', label: 'Leaderboards', description: 'Rankings', icon: <LeaderboardIcon />, section: 'Navigation', action: () => { onNavigate('leaderboards'); onClose(); } },
    { id: 'nav-matchmaker', label: 'Matchmaker', description: 'Match queue', icon: <MatchmakerIcon />, section: 'Navigation', action: () => { onNavigate('matchmaker'); onClose(); } },
    { id: 'nav-tournaments', label: 'Tournaments', description: 'Competitions', icon: <TournamentIcon />, section: 'Navigation', action: () => { onNavigate('tournaments'); onClose(); } },
    { id: 'nav-social', label: 'Social', description: 'Friends & groups', icon: <SocialIcon />, section: 'Navigation', action: () => { onNavigate('social'); onClose(); } },
    { id: 'nav-notifications', label: 'Notifications', description: 'Player notifications', icon: <NotificationsIcon />, section: 'Navigation', action: () => { onNavigate('notifications'); onClose(); } },
    { id: 'nav-storage', label: 'Storage', description: 'Data storage', icon: <StorageIcon />, section: 'Navigation', action: () => { onNavigate('storage'); onClose(); } },
    { id: 'nav-auth', label: 'Auth Testing', description: 'Authentication test', icon: <AuthIcon />, section: 'Navigation', action: () => { onNavigate('auth'); onClose(); } },
    { id: 'nav-accounts', label: 'Accounts', description: 'Admin accounts', icon: <AccountsIcon />, section: 'Administration', action: () => { onNavigate('accounts'); onClose(); } },
    { id: 'nav-apikeys', label: 'API Keys', description: 'Manage API keys', icon: <ApiKeysIcon />, section: 'Administration', action: () => { onNavigate('apikeys'); onClose(); } },

    // Actions
    { id: 'action-refresh', label: 'Refresh', description: 'Refresh current page', icon: <RefreshIcon />, shortcut: 'R', section: 'Actions', action: () => { window.location.reload(); } },
    { id: 'action-docs', label: 'Documentation', description: 'Open docs', icon: <DocsIcon />, section: 'Actions', action: () => { window.open('https://github.com/your-repo/kaosnet', '_blank'); onClose(); } },
  ];

  const filteredCommands = commands.filter(cmd =>
    cmd.label.toLowerCase().includes(query.toLowerCase()) ||
    cmd.description?.toLowerCase().includes(query.toLowerCase()) ||
    cmd.section.toLowerCase().includes(query.toLowerCase())
  );

  const groupedCommands = filteredCommands.reduce((acc, cmd) => {
    if (!acc[cmd.section]) acc[cmd.section] = [];
    acc[cmd.section].push(cmd);
    return acc;
  }, {} as Record<string, Command[]>);

  const flatCommands = Object.values(groupedCommands).flat();

  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [isOpen]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex(i => Math.min(i + 1, flatCommands.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex(i => Math.max(i - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (flatCommands[selectedIndex]) {
          flatCommands[selectedIndex].action();
        }
        break;
      case 'Escape':
        onClose();
        break;
    }
  }, [flatCommands, selectedIndex, onClose]);

  // Scroll selected item into view
  useEffect(() => {
    const selectedEl = listRef.current?.querySelector(`[data-index="${selectedIndex}"]`);
    selectedEl?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  if (!isOpen) return null;

  let globalIndex = -1;

  return (
    <div className="command-palette-overlay" onClick={onClose}>
      <div
        className="command-palette"
        onClick={e => e.stopPropagation()}
        style={{
          background: 'var(--bg-secondary)',
          border: '1px solid var(--border-primary)',
          borderRadius: '16px',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.5)',
          width: '100%',
          maxWidth: '560px',
          maxHeight: '70vh',
          overflow: 'hidden',
        }}
      >
        {/* Search Input */}
        <div
          className="p-4"
          style={{ borderBottom: '1px solid var(--border-primary)' }}
        >
          <div className="flex items-center gap-3">
            <SearchIcon className="w-5 h-5 flex-shrink-0" style={{ color: 'var(--text-muted)' }} />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={e => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Search commands..."
              className="flex-1 bg-transparent border-none outline-none text-base"
              style={{ color: 'var(--text-primary)' }}
            />
            <kbd
              className="px-2 py-1 rounded text-xs font-mono"
              style={{
                background: 'var(--bg-tertiary)',
                color: 'var(--text-muted)',
                border: '1px solid var(--border-primary)',
              }}
            >
              ESC
            </kbd>
          </div>
        </div>

        {/* Commands List */}
        <div
          ref={listRef}
          className="overflow-y-auto"
          style={{ maxHeight: 'calc(70vh - 72px)' }}
        >
          {Object.entries(groupedCommands).length === 0 ? (
            <div className="p-8 text-center" style={{ color: 'var(--text-muted)' }}>
              <SearchIcon className="w-10 h-10 mx-auto mb-3 opacity-30" />
              <p>No commands found</p>
            </div>
          ) : (
            Object.entries(groupedCommands).map(([section, cmds]) => (
              <div key={section}>
                <div
                  className="px-4 py-2 text-xs font-semibold uppercase tracking-wider"
                  style={{
                    color: 'var(--text-muted)',
                    background: 'var(--bg-tertiary)',
                  }}
                >
                  {section}
                </div>
                {cmds.map((cmd) => {
                  globalIndex++;
                  const isSelected = globalIndex === selectedIndex;
                  const currentIndex = globalIndex;

                  return (
                    <button
                      key={cmd.id}
                      data-index={currentIndex}
                      onClick={cmd.action}
                      className="w-full flex items-center gap-3 px-4 py-3 text-left transition-colors"
                      style={{
                        background: isSelected ? 'var(--bg-hover)' : 'transparent',
                        borderLeft: isSelected ? '2px solid var(--color-accent)' : '2px solid transparent',
                      }}
                      onMouseEnter={() => setSelectedIndex(currentIndex)}
                    >
                      <span
                        className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
                        style={{
                          background: isSelected ? 'rgba(6, 182, 212, 0.15)' : 'var(--bg-tertiary)',
                          color: isSelected ? 'var(--color-accent)' : 'var(--text-muted)',
                        }}
                      >
                        {cmd.icon}
                      </span>
                      <div className="flex-1 min-w-0">
                        <p
                          className="font-medium truncate"
                          style={{ color: isSelected ? 'var(--text-primary)' : 'var(--text-secondary)' }}
                        >
                          {cmd.label}
                        </p>
                        {cmd.description && (
                          <p className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>
                            {cmd.description}
                          </p>
                        )}
                      </div>
                      {cmd.shortcut && (
                        <kbd
                          className="px-1.5 py-0.5 rounded text-xs font-mono flex-shrink-0"
                          style={{
                            background: 'var(--bg-tertiary)',
                            color: 'var(--text-muted)',
                            border: '1px solid var(--border-primary)',
                          }}
                        >
                          {cmd.shortcut}
                        </kbd>
                      )}
                    </button>
                  );
                })}
              </div>
            ))
          )}
        </div>

        {/* Footer */}
        <div
          className="px-4 py-3 flex items-center justify-between text-xs"
          style={{
            borderTop: '1px solid var(--border-primary)',
            background: 'var(--bg-tertiary)',
            color: 'var(--text-muted)',
          }}
        >
          <div className="flex items-center gap-4">
            <span className="flex items-center gap-1">
              <kbd className="px-1 py-0.5 rounded bg-white/5 border border-white/10">Enter</kbd>
              to select
            </span>
            <span className="flex items-center gap-1">
              <kbd className="px-1 py-0.5 rounded bg-white/5 border border-white/10">Arrow</kbd>
              to navigate
            </span>
          </div>
          <span className="flex items-center gap-1">
            <kbd className="px-1 py-0.5 rounded bg-white/5 border border-white/10">Cmd</kbd>
            <kbd className="px-1 py-0.5 rounded bg-white/5 border border-white/10">K</kbd>
            to toggle
          </span>
        </div>
      </div>

      <style>{`
        .command-palette-overlay {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.6);
          backdrop-filter: blur(4px);
          display: flex;
          align-items: flex-start;
          justify-content: center;
          padding-top: 15vh;
          z-index: 100;
          animation: fadeIn 0.15s ease-out;
        }

        .command-palette {
          animation: slideDown 0.2s ease-out;
        }

        @keyframes fadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }

        @keyframes slideDown {
          from {
            opacity: 0;
            transform: translateY(-20px) scale(0.95);
          }
          to {
            opacity: 1;
            transform: translateY(0) scale(1);
          }
        }
      `}</style>
    </div>
  );
}

// Hook for keyboard shortcuts
export function useCommandPalette(onNavigate: (page: string) => void) {
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd+K or Ctrl+K to toggle
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setIsOpen(prev => !prev);
      }

      // Quick navigation shortcuts (only when palette is closed)
      if (!isOpen && !e.metaKey && !e.ctrlKey && !e.altKey) {
        // Check if we're not in an input
        if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') {
          return;
        }

        // G + key for navigation
        if (e.key === 'g') {
          const handleNext = (e2: KeyboardEvent) => {
            switch (e2.key) {
              case 'd': onNavigate('dashboard'); break;
              case 's': onNavigate('sessions'); break;
              case 'r': onNavigate('rooms'); break;
              case 'l': onNavigate('lua'); break;
              case 'p': onNavigate('players'); break;
            }
            window.removeEventListener('keydown', handleNext);
          };
          window.addEventListener('keydown', handleNext, { once: true });
          setTimeout(() => window.removeEventListener('keydown', handleNext), 500);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onNavigate]);

  return { isOpen, setIsOpen };
}

// Icons
function SearchIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
  );
}

function DashboardIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
    </svg>
  );
}

function SessionsIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}

function RoomsIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
  );
}

function LuaIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
    </svg>
  );
}

function PlayersIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
    </svg>
  );
}

function ChatIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
    </svg>
  );
}

function LeaderboardIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
    </svg>
  );
}

function MatchmakerIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  );
}

function TournamentIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z" />
    </svg>
  );
}

function SocialIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}

function NotificationsIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
    </svg>
  );
}

function StorageIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
    </svg>
  );
}

function AuthIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
    </svg>
  );
}

function AccountsIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.121 17.804A13.937 13.937 0 0112 16c2.5 0 4.847.655 6.879 1.804M15 10a3 3 0 11-6 0 3 3 0 016 0zm6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
    </svg>
  );
}

function ApiKeysIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
    </svg>
  );
}

function RefreshIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
    </svg>
  );
}

function DocsIcon({ className }: { className?: string }) {
  return (
    <svg className={className || "w-4 h-4"} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
    </svg>
  );
}

export default CommandPalette;
