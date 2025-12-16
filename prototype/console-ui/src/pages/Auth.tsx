import { useState } from 'react';
import { api } from '../api/client';

interface AuthResult {
  user_id: string;
  username: string;
  access_token: string;
  refresh_token: string;
  expires_at: number;
}

export default function Auth() {
  const [activeTab, setActiveTab] = useState<'device' | 'email' | 'social'>('device');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<AuthResult | null>(null);

  // Device auth state
  const [deviceId, setDeviceId] = useState('');

  // Email auth state
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [username, setUsername] = useState('');
  const [isRegister, setIsRegister] = useState(false);

  // Social auth state
  const [provider, setProvider] = useState('google');
  const [providerId, setProviderId] = useState('');
  const [accessToken, setAccessToken] = useState('');

  const authenticateDevice = async () => {
    if (!deviceId) {
      setError('Device ID is required');
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const data = await api.post('/api/auth/device', { device_id: deviceId });
      setResult(data);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const authenticateEmail = async () => {
    if (!email || !password) {
      setError('Email and password are required');
      return;
    }
    if (isRegister && !username) {
      setError('Username is required for registration');
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const endpoint = isRegister ? '/api/auth/email/register' : '/api/auth/email/login';
      const payload = isRegister
        ? { email, password, username }
        : { email, password };
      const data = await api.post(endpoint, payload);
      setResult(data);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const authenticateSocial = async () => {
    if (!providerId || !accessToken) {
      setError('Provider ID and access token are required');
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const data = await api.post('/api/auth/social', {
        provider,
        provider_id: providerId,
        access_token: accessToken,
      });
      setResult(data);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (ts: number) => new Date(ts).toLocaleString();

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6" style={{ color: 'var(--text-primary)' }}>Authentication Testing</h1>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Auth Methods */}
        <div className="card overflow-hidden !p-0">
          {/* Tabs */}
          <div className="tabs !rounded-none !border-0 !p-0" style={{ borderBottom: '1px solid var(--border-primary)' }}>
            <button
              onClick={() => { setActiveTab('device'); setError(null); setResult(null); }}
              className={`tab flex-1 !rounded-none ${activeTab === 'device' ? 'active' : ''}`}
            >
              Device
            </button>
            <button
              onClick={() => { setActiveTab('email'); setError(null); setResult(null); }}
              className={`tab flex-1 !rounded-none ${activeTab === 'email' ? 'active' : ''}`}
            >
              Email
            </button>
            <button
              onClick={() => { setActiveTab('social'); setError(null); setResult(null); }}
              className={`tab flex-1 !rounded-none ${activeTab === 'social' ? 'active' : ''}`}
            >
              Social
            </button>
          </div>

          <div className="p-6">
            {/* Device Auth */}
            {activeTab === 'device' && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold mb-2" style={{ color: 'var(--text-primary)' }}>Device Authentication</h3>
                <p className="text-sm mb-4" style={{ color: 'var(--text-secondary)' }}>
                  Authenticate using a unique device identifier. Creates a new user if the device ID is not registered.
                </p>
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Device ID</label>
                  <input
                    type="text"
                    value={deviceId}
                    onChange={(e) => setDeviceId(e.target.value)}
                    className="input"
                    placeholder="e.g., device-abc123"
                  />
                </div>
                <button
                  onClick={authenticateDevice}
                  disabled={loading}
                  className="btn btn-primary w-full"
                >
                  {loading ? 'Authenticating...' : 'Authenticate Device'}
                </button>
              </div>
            )}

            {/* Email Auth */}
            {activeTab === 'email' && (
              <div className="space-y-4">
                <div className="flex justify-between items-center mb-2">
                  <h3 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>Email Authentication</h3>
                  <button
                    onClick={() => setIsRegister(!isRegister)}
                    className="text-sm hover:opacity-80"
                    style={{ color: 'var(--color-accent)' }}
                  >
                    {isRegister ? 'Switch to Login' : 'Switch to Register'}
                  </button>
                </div>
                <p className="text-sm mb-4" style={{ color: 'var(--text-secondary)' }}>
                  {isRegister
                    ? 'Register a new account with email and password.'
                    : 'Login with existing email and password credentials.'}
                </p>
                {isRegister && (
                  <div>
                    <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Username</label>
                    <input
                      type="text"
                      value={username}
                      onChange={(e) => setUsername(e.target.value)}
                      className="input"
                      placeholder="Choose a username"
                    />
                  </div>
                )}
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Email</label>
                  <input
                    type="email"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    className="input"
                    placeholder="user@example.com"
                  />
                </div>
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Password</label>
                  <input
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className="input"
                    placeholder="Enter password"
                  />
                </div>
                <button
                  onClick={authenticateEmail}
                  disabled={loading}
                  className="btn btn-primary w-full"
                >
                  {loading ? 'Processing...' : isRegister ? 'Register' : 'Login'}
                </button>
              </div>
            )}

            {/* Social Auth */}
            {activeTab === 'social' && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold mb-2" style={{ color: 'var(--text-primary)' }}>Social Authentication</h3>
                <p className="text-sm mb-4" style={{ color: 'var(--text-secondary)' }}>
                  Authenticate using a social provider. Simulates OAuth flow by providing provider details directly.
                </p>
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Provider</label>
                  <select
                    value={provider}
                    onChange={(e) => setProvider(e.target.value)}
                    className="input"
                  >
                    <option value="google">Google</option>
                    <option value="facebook">Facebook</option>
                    <option value="apple">Apple</option>
                    <option value="steam">Steam</option>
                    <option value="discord">Discord</option>
                    <option value="custom">Custom</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Provider User ID</label>
                  <input
                    type="text"
                    value={providerId}
                    onChange={(e) => setProviderId(e.target.value)}
                    className="input"
                    placeholder="e.g., 123456789"
                  />
                </div>
                <div>
                  <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Access Token</label>
                  <input
                    type="text"
                    value={accessToken}
                    onChange={(e) => setAccessToken(e.target.value)}
                    className="input"
                    placeholder="OAuth access token"
                  />
                </div>
                <button
                  onClick={authenticateSocial}
                  disabled={loading}
                  className="btn btn-primary w-full"
                >
                  {loading ? 'Authenticating...' : 'Authenticate with ' + provider}
                </button>
              </div>
            )}

            {/* Error Display */}
            {error && (
              <div className="alert alert-error mt-4">
                {error}
              </div>
            )}
          </div>
        </div>

        {/* Result */}
        <div className="card overflow-hidden !p-0">
          <div className="px-4 py-3 font-semibold" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)' }}>
            Authentication Result
          </div>
          <div className="p-6">
            {result ? (
              <div className="space-y-4">
                <div className="alert alert-success mb-4">
                  Authentication successful!
                </div>
                <div>
                  <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>User ID</label>
                  <div className="code-block mt-1 break-all">
                    {result.user_id}
                  </div>
                </div>
                <div>
                  <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Username</label>
                  <div className="code-block mt-1">
                    {result.username}
                  </div>
                </div>
                <div>
                  <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Expires At</label>
                  <div className="code-block mt-1">
                    {formatDate(result.expires_at * 1000)}
                  </div>
                </div>
                <div>
                  <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Access Token</label>
                  <textarea
                    readOnly
                    value={result.access_token}
                    className="input w-full h-24 font-mono text-xs mt-1 resize-none"
                  />
                </div>
                <div>
                  <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Refresh Token</label>
                  <textarea
                    readOnly
                    value={result.refresh_token}
                    className="input w-full h-24 font-mono text-xs mt-1 resize-none"
                  />
                </div>
              </div>
            ) : (
              <div className="text-center py-8" style={{ color: 'var(--text-secondary)' }}>
                Authenticate to see the result
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Info Cards */}
      <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="card">
          <h4 className="font-semibold mb-2" style={{ color: 'var(--color-accent)' }}>Device Auth</h4>
          <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            Best for mobile games and anonymous play. Device ID is generated once and stored locally.
          </p>
        </div>
        <div className="card">
          <h4 className="font-semibold mb-2" style={{ color: 'var(--color-success)' }}>Email Auth</h4>
          <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            Traditional email/password login. Supports registration and password verification.
          </p>
        </div>
        <div className="card">
          <h4 className="font-semibold mb-2" style={{ color: 'var(--color-tertiary)' }}>Social Auth</h4>
          <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            OAuth-based login with third-party providers like Google, Facebook, Apple, Steam, and Discord.
          </p>
        </div>
      </div>
    </div>
  );
}
