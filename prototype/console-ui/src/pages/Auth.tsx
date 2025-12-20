import { useState } from 'react';
import { api } from '../api/client';
import { PageHeader, Alert } from '../components/ui';
import { formatTimestamp } from '../utils/formatters';

interface AuthResult { user_id: string; username: string; access_token: string; refresh_token: string; expires_at: number; }

export default function Auth() {
  const [activeTab, setActiveTab] = useState<'device' | 'email' | 'social'>('device');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<AuthResult | null>(null);
  const [deviceId, setDeviceId] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [username, setUsername] = useState('');
  const [isRegister, setIsRegister] = useState(false);
  const [provider, setProvider] = useState('google');
  const [providerId, setProviderId] = useState('');
  const [accessToken, setAccessToken] = useState('');

  const authenticateDevice = async () => {
    if (!deviceId) { setError('Device ID is required'); return; }
    try { setLoading(true); setError(null); const data = await api.post('/api/auth/device', { device_id: deviceId }); setResult(data); }
    catch (err) { setError(err instanceof Error ? err.message : 'Authentication failed'); }
    finally { setLoading(false); }
  };

  const authenticateEmail = async () => {
    if (!email || !password) { setError('Email and password are required'); return; }
    if (isRegister && !username) { setError('Username is required for registration'); return; }
    try {
      setLoading(true); setError(null);
      const endpoint = isRegister ? '/api/auth/email/register' : '/api/auth/email/login';
      const payload = isRegister ? { email, password, username } : { email, password };
      const data = await api.post(endpoint, payload);
      setResult(data);
    } catch (err) { setError(err instanceof Error ? err.message : 'Authentication failed'); }
    finally { setLoading(false); }
  };

  const authenticateSocial = async () => {
    if (!providerId || !accessToken) { setError('Provider ID and access token are required'); return; }
    try { setLoading(true); setError(null); const data = await api.post('/api/auth/social', { provider, provider_id: providerId, access_token: accessToken }); setResult(data); }
    catch (err) { setError(err instanceof Error ? err.message : 'Authentication failed'); }
    finally { setLoading(false); }
  };

  const switchTab = (tab: 'device' | 'email' | 'social') => { setActiveTab(tab); setError(null); setResult(null); };

  return (
    <div className="space-y-6 animate-fade-in">
      <PageHeader title="Authentication Testing" subtitle="Test client authentication methods" />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="card p-0 overflow-hidden">
          <div className="flex border-b" style={{ borderColor: 'var(--border-primary)' }}>
            <button onClick={() => switchTab('device')} className={`flex-1 px-4 py-3 text-sm font-medium transition-colors ${activeTab === 'device' ? 'bg-[var(--color-accent)] text-white' : ''}`} style={{ color: activeTab === 'device' ? 'white' : 'var(--text-secondary)' }}>Device</button>
            <button onClick={() => switchTab('email')} className={`flex-1 px-4 py-3 text-sm font-medium transition-colors ${activeTab === 'email' ? 'bg-[var(--color-accent)] text-white' : ''}`} style={{ color: activeTab === 'email' ? 'white' : 'var(--text-secondary)' }}>Email</button>
            <button onClick={() => switchTab('social')} className={`flex-1 px-4 py-3 text-sm font-medium transition-colors ${activeTab === 'social' ? 'bg-[var(--color-accent)] text-white' : ''}`} style={{ color: activeTab === 'social' ? 'white' : 'var(--text-secondary)' }}>Social</button>
          </div>

          <div className="p-6">
            {activeTab === 'device' && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>Device Authentication</h3>
                <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>Authenticate using a unique device identifier. Creates a new user if the device ID is not registered.</p>
                <div><label className="form-label">Device ID</label><input type="text" value={deviceId} onChange={(e) => setDeviceId(e.target.value)} className="form-input" placeholder="e.g., device-abc123" /></div>
                <button onClick={authenticateDevice} disabled={loading} className="btn btn-primary w-full">{loading ? 'Authenticating...' : 'Authenticate Device'}</button>
              </div>
            )}

            {activeTab === 'email' && (
              <div className="space-y-4">
                <div className="flex justify-between items-center">
                  <h3 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>Email Authentication</h3>
                  <button onClick={() => setIsRegister(!isRegister)} className="text-sm hover:opacity-80" style={{ color: 'var(--color-accent)' }}>{isRegister ? 'Switch to Login' : 'Switch to Register'}</button>
                </div>
                <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>{isRegister ? 'Register a new account with email and password.' : 'Login with existing email and password credentials.'}</p>
                {isRegister && <div><label className="form-label">Username</label><input type="text" value={username} onChange={(e) => setUsername(e.target.value)} className="form-input" placeholder="Choose a username" /></div>}
                <div><label className="form-label">Email</label><input type="email" value={email} onChange={(e) => setEmail(e.target.value)} className="form-input" placeholder="user@example.com" /></div>
                <div><label className="form-label">Password</label><input type="password" value={password} onChange={(e) => setPassword(e.target.value)} className="form-input" placeholder="Enter password" /></div>
                <button onClick={authenticateEmail} disabled={loading} className="btn btn-primary w-full">{loading ? 'Processing...' : isRegister ? 'Register' : 'Login'}</button>
              </div>
            )}

            {activeTab === 'social' && (
              <div className="space-y-4">
                <h3 className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>Social Authentication</h3>
                <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>Authenticate using a social provider. Simulates OAuth flow by providing provider details directly.</p>
                <div><label className="form-label">Provider</label><select value={provider} onChange={(e) => setProvider(e.target.value)} className="form-input"><option value="google">Google</option><option value="facebook">Facebook</option><option value="apple">Apple</option><option value="steam">Steam</option><option value="discord">Discord</option><option value="custom">Custom</option></select></div>
                <div><label className="form-label">Provider User ID</label><input type="text" value={providerId} onChange={(e) => setProviderId(e.target.value)} className="form-input" placeholder="e.g., 123456789" /></div>
                <div><label className="form-label">Access Token</label><input type="text" value={accessToken} onChange={(e) => setAccessToken(e.target.value)} className="form-input" placeholder="OAuth access token" /></div>
                <button onClick={authenticateSocial} disabled={loading} className="btn btn-primary w-full">{loading ? 'Authenticating...' : `Authenticate with ${provider}`}</button>
              </div>
            )}

            {error && <Alert variant="danger" className="mt-4">{error}</Alert>}
          </div>
        </div>

        <div className="card p-0 overflow-hidden">
          <div className="px-4 py-3 font-semibold" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)' }}>Authentication Result</div>
          <div className="p-6">
            {result ? (
              <div className="space-y-4">
                <Alert variant="success">Authentication successful!</Alert>
                <div><label className="text-sm" style={{ color: 'var(--text-secondary)' }}>User ID</label><div className="p-2 rounded font-mono text-sm break-all mt-1" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)' }}>{result.user_id}</div></div>
                <div><label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Username</label><div className="p-2 rounded font-mono text-sm mt-1" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)' }}>{result.username}</div></div>
                <div><label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Expires At</label><div className="p-2 rounded font-mono text-sm mt-1" style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)' }}>{formatTimestamp(result.expires_at * 1000)}</div></div>
                <div><label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Access Token</label><textarea readOnly value={result.access_token} className="form-input w-full h-24 font-mono text-xs mt-1 resize-none" /></div>
                <div><label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Refresh Token</label><textarea readOnly value={result.refresh_token} className="form-input w-full h-24 font-mono text-xs mt-1 resize-none" /></div>
              </div>
            ) : (
              <div className="text-center py-8" style={{ color: 'var(--text-muted)' }}>Authenticate to see the result</div>
            )}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="card"><h4 className="font-semibold mb-2" style={{ color: 'var(--color-accent)' }}>Device Auth</h4><p className="text-sm" style={{ color: 'var(--text-secondary)' }}>Best for mobile games and anonymous play. Device ID is generated once and stored locally.</p></div>
        <div className="card"><h4 className="font-semibold mb-2" style={{ color: 'var(--color-success)' }}>Email Auth</h4><p className="text-sm" style={{ color: 'var(--text-secondary)' }}>Traditional email/password login. Supports registration and password verification.</p></div>
        <div className="card"><h4 className="font-semibold mb-2" style={{ color: 'var(--color-warning)' }}>Social Auth</h4><p className="text-sm" style={{ color: 'var(--text-secondary)' }}>OAuth-based login with third-party providers like Google, Facebook, Apple, Steam, and Discord.</p></div>
      </div>
    </div>
  );
}
