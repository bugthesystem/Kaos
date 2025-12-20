import { useState } from 'react';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { ThemeProvider } from './contexts/ThemeContext';
import { ToastProvider } from './components/Toast';
import { Layout } from './components/Layout';
import { LoginPage } from './pages/Login';
import { DashboardPage } from './pages/Dashboard';
import { SessionsPage } from './pages/Sessions';
import { RoomsPage } from './pages/Rooms';
import { AccountsPage } from './pages/Accounts';
import { ApiKeysPage } from './pages/ApiKeys';
import { LuaPage } from './pages/Lua';
import Players from './pages/Players';
import Chat from './pages/Chat';
import Leaderboards from './pages/Leaderboards';
import Matchmaker from './pages/Matchmaker';
import Notifications from './pages/Notifications';
import Social from './pages/Social';
import Storage from './pages/Storage';
import Tournaments from './pages/Tournaments';
import Auth from './pages/Auth';
import ApiExplorer from './pages/ApiExplorer';
import Roles from './pages/Roles';
import AuditLogs from './pages/AuditLogs';
import { MetricsPage } from './pages/Metrics';

function AppContent() {
  const { user, isLoading } = useAuth();
  const [currentPage, setCurrentPage] = useState('dashboard');
  const [pageKey, setPageKey] = useState(0);

  // Trigger page transition animation on page change
  const handleNavigate = (page: string) => {
    setCurrentPage(page);
    setPageKey((k) => k + 1);
  };

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center" style={{ background: 'var(--bg-primary)' }}>
        <div className="spinner" />
      </div>
    );
  }

  if (!user) {
    return <LoginPage />;
  }

  const renderPage = () => {
    switch (currentPage) {
      case 'dashboard':
        return <DashboardPage onNavigate={handleNavigate} />;
      case 'sessions':
        return <SessionsPage />;
      case 'rooms':
        return <RoomsPage />;
      case 'lua':
        return <LuaPage />;
      case 'players':
        return <Players />;
      case 'chat':
        return <Chat />;
      case 'leaderboards':
        return <Leaderboards />;
      case 'matchmaker':
        return <Matchmaker />;
      case 'notifications':
        return <Notifications />;
      case 'social':
        return <Social />;
      case 'storage':
        return <Storage />;
      case 'tournaments':
        return <Tournaments />;
      case 'auth':
        return <Auth />;
      case 'accounts':
        return <AccountsPage />;
      case 'apikeys':
        return <ApiKeysPage />;
      case 'api-explorer':
        return <ApiExplorer />;
      case 'roles':
        return <Roles />;
      case 'audit-logs':
        return <AuditLogs />;
      case 'metrics':
        return <MetricsPage />;
      default:
        return <DashboardPage onNavigate={handleNavigate} />;
    }
  };

  return (
    <Layout currentPage={currentPage} onNavigate={handleNavigate}>
      <div key={pageKey} className="page-enter">
        {renderPage()}
      </div>
    </Layout>
  );
}

function App() {
  return (
    <ThemeProvider>
      <ToastProvider>
        <AuthProvider>
          <AppContent />
        </AuthProvider>
      </ToastProvider>
    </ThemeProvider>
  );
}

export default App;
