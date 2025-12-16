import { useState } from 'react';
import { useAuth } from '../contexts/AuthContext';

interface LayoutProps {
  children: React.ReactNode;
  currentPage: string;
  onNavigate: (page: string) => void;
}

export function Layout({ children, currentPage, onNavigate }: LayoutProps) {
  const { user, logout, isAdmin } = useAuth();
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: '📊' },
    { id: 'sessions', label: 'Sessions', icon: '👥' },
    { id: 'rooms', label: 'Rooms', icon: '🎮' },
    { id: 'lua', label: 'Lua Scripts', icon: '📜' },
    ...(isAdmin ? [
      { id: 'accounts', label: 'Accounts', icon: '👤' },
      { id: 'apikeys', label: 'API Keys', icon: '🔑' },
    ] : []),
  ];

  return (
    <div className="flex min-h-screen bg-gray-900">
      {/* Sidebar */}
      <aside className={`${sidebarOpen ? 'w-64' : 'w-20'} bg-gray-800 border-r border-gray-700 transition-all duration-300`}>
        <div className="flex flex-col h-full">
          {/* Logo */}
          <div className="flex items-center gap-3 px-4 py-5 border-b border-gray-700">
            <div className="w-10 h-10 bg-sky-600 rounded-lg flex items-center justify-center text-xl font-bold">
              K
            </div>
            {sidebarOpen && (
              <div>
                <h1 className="text-lg font-bold text-white">KaosNet</h1>
                <p className="text-xs text-gray-400">Console</p>
              </div>
            )}
          </div>

          {/* Navigation */}
          <nav className="flex-1 px-3 py-4 space-y-1">
            {navItems.map((item) => (
              <button
                key={item.id}
                onClick={() => onNavigate(item.id)}
                className={`nav-link w-full ${currentPage === item.id ? 'active' : ''}`}
              >
                <span className="text-xl">{item.icon}</span>
                {sidebarOpen && <span>{item.label}</span>}
              </button>
            ))}
          </nav>

          {/* User */}
          <div className="px-3 py-4 border-t border-gray-700">
            {sidebarOpen ? (
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium text-white">{user?.username}</p>
                  <p className="text-xs text-gray-400 capitalize">{user?.role}</p>
                </div>
                <button
                  onClick={logout}
                  className="text-gray-400 hover:text-white"
                  title="Logout"
                >
                  🚪
                </button>
              </div>
            ) : (
              <button
                onClick={logout}
                className="w-full flex justify-center text-gray-400 hover:text-white"
                title="Logout"
              >
                🚪
              </button>
            )}
          </div>

          {/* Toggle */}
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="px-3 py-2 text-gray-400 hover:text-white border-t border-gray-700"
          >
            {sidebarOpen ? '◀' : '▶'}
          </button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-auto">
        <div className="p-6">
          {children}
        </div>
      </main>
    </div>
  );
}
