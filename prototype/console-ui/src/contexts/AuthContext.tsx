import { createContext, useContext, useEffect, useState, useRef, useCallback, type ReactNode } from 'react';
import { api } from '../api/client';
import type { AccountInfo } from '../api/types';

interface AuthContextType {
  user: AccountInfo | null;
  token: string | null;
  isLoading: boolean;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  isAdmin: boolean;
}

const AuthContext = createContext<AuthContextType | null>(null);

// Refresh token 5 minutes before expiry
const REFRESH_BUFFER_SECS = 5 * 60;

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AccountInfo | null>(null);
  const [token, setToken] = useState<string | null>(() => localStorage.getItem('token'));
  const [isLoading, setIsLoading] = useState(true);
  const refreshTimerRef = useRef<number | null>(null);

  const clearRefreshTimer = useCallback(() => {
    if (refreshTimerRef.current) {
      clearTimeout(refreshTimerRef.current);
      refreshTimerRef.current = null;
    }
  }, []);

  const scheduleRefresh = useCallback((expiresAt: number) => {
    clearRefreshTimer();

    const now = Math.floor(Date.now() / 1000);
    const timeUntilRefresh = (expiresAt - now - REFRESH_BUFFER_SECS) * 1000;

    if (timeUntilRefresh > 0) {
      refreshTimerRef.current = window.setTimeout(async () => {
        try {
          const response = await api.refresh();
          setToken(response.token);
          setUser(response.user);
          localStorage.setItem('token', response.token);
          localStorage.setItem('token_expires_at', response.expires_at.toString());
          api.setToken(response.token);
          scheduleRefresh(response.expires_at);
        } catch {
          // Refresh failed - token might be invalid, force re-login
          setToken(null);
          setUser(null);
          localStorage.removeItem('token');
          localStorage.removeItem('token_expires_at');
          api.setToken(null);
        }
      }, timeUntilRefresh);
    }
  }, [clearRefreshTimer]);

  useEffect(() => {
    if (token) {
      api.setToken(token);
      api.me()
        .then((userData) => {
          setUser(userData);
          // Schedule refresh based on stored expiry or try to refresh now
          const storedExpiry = localStorage.getItem('token_expires_at');
          if (storedExpiry) {
            const expiresAt = parseInt(storedExpiry, 10);
            const now = Math.floor(Date.now() / 1000);
            if (expiresAt > now + REFRESH_BUFFER_SECS) {
              // Token still valid, schedule refresh
              scheduleRefresh(expiresAt);
            } else {
              // Token about to expire or expired, refresh now
              api.refresh()
                .then((response) => {
                  setToken(response.token);
                  setUser(response.user);
                  localStorage.setItem('token', response.token);
                  localStorage.setItem('token_expires_at', response.expires_at.toString());
                  api.setToken(response.token);
                  scheduleRefresh(response.expires_at);
                })
                .catch(() => {
                  // Couldn't refresh, user stays logged in until next action fails
                });
            }
          } else {
            // No stored expiry, try to refresh to get fresh token
            api.refresh()
              .then((response) => {
                setToken(response.token);
                setUser(response.user);
                localStorage.setItem('token', response.token);
                localStorage.setItem('token_expires_at', response.expires_at.toString());
                api.setToken(response.token);
                scheduleRefresh(response.expires_at);
              })
              .catch(() => {
                // Old token still works but we couldn't refresh
              });
          }
        })
        .catch(() => {
          setToken(null);
          localStorage.removeItem('token');
          localStorage.removeItem('token_expires_at');
        })
        .finally(() => setIsLoading(false));
    } else {
      setIsLoading(false);
    }

    return () => {
      clearRefreshTimer();
    };
  }, [token, scheduleRefresh, clearRefreshTimer]);

  const login = async (username: string, password: string) => {
    const response = await api.login({ username, password });
    setToken(response.token);
    setUser(response.user);
    localStorage.setItem('token', response.token);
    localStorage.setItem('token_expires_at', response.expires_at.toString());
    api.setToken(response.token);
    scheduleRefresh(response.expires_at);
  };

  const logout = () => {
    clearRefreshTimer();
    setToken(null);
    setUser(null);
    localStorage.removeItem('token');
    localStorage.removeItem('token_expires_at');
    api.setToken(null);
  };

  const isAdmin = user?.role === 'admin';

  return (
    <AuthContext.Provider value={{ user, token, isLoading, login, logout, isAdmin }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
