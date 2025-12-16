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
      <h1 className="text-2xl font-bold mb-6">Authentication Testing</h1>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Auth Methods */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          {/* Tabs */}
          <div className="flex border-b border-gray-700">
            <button
              onClick={() => { setActiveTab('device'); setError(null); setResult(null); }}
              className={`flex-1 px-4 py-3 text-sm font-medium ${
                activeTab === 'device' ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-white'
              }`}
            >
              Device
            </button>
            <button
              onClick={() => { setActiveTab('email'); setError(null); setResult(null); }}
              className={`flex-1 px-4 py-3 text-sm font-medium ${
                activeTab === 'email' ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-white'
              }`}
            >
              Email
            </button>
            <button
              onClick={() => { setActiveTab('social'); setError(null); setResult(null); }}
              className={`flex-1 px-4 py-3 text-sm font-medium ${
                activeTab === 'social' ? 'bg-gray-700 text-white' : 'text-gray-400 hover:text-white'
              }`}
            >
              Social
            </button>
          </div>

          <div className="p-6">
            {/* Device Auth */}
            {activeTab === 'device' && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold mb-2">Device Authentication</h3>
                <p className="text-gray-400 text-sm mb-4">
                  Authenticate using a unique device identifier. Creates a new user if the device ID is not registered.
                </p>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Device ID</label>
                  <input
                    type="text"
                    value={deviceId}
                    onChange={(e) => setDeviceId(e.target.value)}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                    placeholder="e.g., device-abc123"
                  />
                </div>
                <button
                  onClick={authenticateDevice}
                  disabled={loading}
                  className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded disabled:opacity-50"
                >
                  {loading ? 'Authenticating...' : 'Authenticate Device'}
                </button>
              </div>
            )}

            {/* Email Auth */}
            {activeTab === 'email' && (
              <div className="space-y-4">
                <div className="flex justify-between items-center mb-2">
                  <h3 className="text-lg font-semibold">Email Authentication</h3>
                  <button
                    onClick={() => setIsRegister(!isRegister)}
                    className="text-sm text-blue-400 hover:text-blue-300"
                  >
                    {isRegister ? 'Switch to Login' : 'Switch to Register'}
                  </button>
                </div>
                <p className="text-gray-400 text-sm mb-4">
                  {isRegister
                    ? 'Register a new account with email and password.'
                    : 'Login with existing email and password credentials.'}
                </p>
                {isRegister && (
                  <div>
                    <label className="block text-sm text-gray-400 mb-1">Username</label>
                    <input
                      type="text"
                      value={username}
                      onChange={(e) => setUsername(e.target.value)}
                      className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                      placeholder="Choose a username"
                    />
                  </div>
                )}
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Email</label>
                  <input
                    type="email"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                    placeholder="user@example.com"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Password</label>
                  <input
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                    placeholder="Enter password"
                  />
                </div>
                <button
                  onClick={authenticateEmail}
                  disabled={loading}
                  className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded disabled:opacity-50"
                >
                  {loading ? 'Processing...' : isRegister ? 'Register' : 'Login'}
                </button>
              </div>
            )}

            {/* Social Auth */}
            {activeTab === 'social' && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold mb-2">Social Authentication</h3>
                <p className="text-gray-400 text-sm mb-4">
                  Authenticate using a social provider. Simulates OAuth flow by providing provider details directly.
                </p>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Provider</label>
                  <select
                    value={provider}
                    onChange={(e) => setProvider(e.target.value)}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
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
                  <label className="block text-sm text-gray-400 mb-1">Provider User ID</label>
                  <input
                    type="text"
                    value={providerId}
                    onChange={(e) => setProviderId(e.target.value)}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                    placeholder="e.g., 123456789"
                  />
                </div>
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Access Token</label>
                  <input
                    type="text"
                    value={accessToken}
                    onChange={(e) => setAccessToken(e.target.value)}
                    className="w-full px-3 py-2 bg-gray-900 rounded border border-gray-700"
                    placeholder="OAuth access token"
                  />
                </div>
                <button
                  onClick={authenticateSocial}
                  disabled={loading}
                  className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded disabled:opacity-50"
                >
                  {loading ? 'Authenticating...' : 'Authenticate with ' + provider}
                </button>
              </div>
            )}

            {/* Error Display */}
            {error && (
              <div className="mt-4 p-3 bg-red-900/50 border border-red-700 rounded text-red-300 text-sm">
                {error}
              </div>
            )}
          </div>
        </div>

        {/* Result */}
        <div className="bg-gray-800 rounded-lg overflow-hidden">
          <div className="bg-gray-900 px-4 py-3 font-semibold">
            Authentication Result
          </div>
          <div className="p-6">
            {result ? (
              <div className="space-y-4">
                <div className="p-3 bg-green-900/50 border border-green-700 rounded text-green-300 text-sm mb-4">
                  Authentication successful!
                </div>
                <div>
                  <label className="text-gray-400 text-sm">User ID</label>
                  <div className="font-mono text-sm bg-gray-900 p-2 rounded mt-1 break-all">
                    {result.user_id}
                  </div>
                </div>
                <div>
                  <label className="text-gray-400 text-sm">Username</label>
                  <div className="bg-gray-900 p-2 rounded mt-1">
                    {result.username}
                  </div>
                </div>
                <div>
                  <label className="text-gray-400 text-sm">Expires At</label>
                  <div className="bg-gray-900 p-2 rounded mt-1">
                    {formatDate(result.expires_at * 1000)}
                  </div>
                </div>
                <div>
                  <label className="text-gray-400 text-sm">Access Token</label>
                  <textarea
                    readOnly
                    value={result.access_token}
                    className="w-full h-24 font-mono text-xs bg-gray-900 p-2 rounded mt-1 resize-none"
                  />
                </div>
                <div>
                  <label className="text-gray-400 text-sm">Refresh Token</label>
                  <textarea
                    readOnly
                    value={result.refresh_token}
                    className="w-full h-24 font-mono text-xs bg-gray-900 p-2 rounded mt-1 resize-none"
                  />
                </div>
              </div>
            ) : (
              <div className="text-gray-400 text-center py-8">
                Authenticate to see the result
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Info Cards */}
      <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-gray-800 rounded-lg p-4">
          <h4 className="font-semibold text-blue-400 mb-2">Device Auth</h4>
          <p className="text-gray-400 text-sm">
            Best for mobile games and anonymous play. Device ID is generated once and stored locally.
          </p>
        </div>
        <div className="bg-gray-800 rounded-lg p-4">
          <h4 className="font-semibold text-green-400 mb-2">Email Auth</h4>
          <p className="text-gray-400 text-sm">
            Traditional email/password login. Supports registration and password verification.
          </p>
        </div>
        <div className="bg-gray-800 rounded-lg p-4">
          <h4 className="font-semibold text-purple-400 mb-2">Social Auth</h4>
          <p className="text-gray-400 text-sm">
            OAuth-based login with third-party providers like Google, Facebook, Apple, Steam, and Discord.
          </p>
        </div>
      </div>
    </div>
  );
}
