# Multi-Tenant Architecture Plan

This document outlines the plan for adding multi-tenant support to KaosNet.

## Overview

Multi-tenancy allows multiple organizations/projects to share the same KaosNet infrastructure while keeping their data isolated. Each tenant gets their own logical namespace for players, leaderboards, storage, etc.

## Data Model

### New Tables

```sql
-- Tenants (organizations/projects)
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(63) NOT NULL UNIQUE,  -- URL-friendly identifier
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Tenant membership
CREATE TABLE tenant_members (
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID REFERENCES accounts(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member',  -- owner, admin, member
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (tenant_id, user_id)
);

-- API keys scoped to tenant
CREATE TABLE tenant_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    key_hash VARCHAR(255) NOT NULL,
    permissions JSONB DEFAULT '[]',
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP
);
```

### Schema Changes

Add `tenant_id` to all existing tables:

```sql
ALTER TABLE players ADD COLUMN tenant_id UUID REFERENCES tenants(id);
ALTER TABLE leaderboards ADD COLUMN tenant_id UUID REFERENCES tenants(id);
ALTER TABLE leaderboard_records ADD COLUMN tenant_id UUID REFERENCES tenants(id);
ALTER TABLE storage_objects ADD COLUMN tenant_id UUID REFERENCES tenants(id);
ALTER TABLE chat_channels ADD COLUMN tenant_id UUID REFERENCES tenants(id);
ALTER TABLE notifications ADD COLUMN tenant_id UUID REFERENCES tenants(id);
ALTER TABLE tournaments ADD COLUMN tenant_id UUID REFERENCES tenants(id);
ALTER TABLE groups ADD COLUMN tenant_id UUID REFERENCES tenants(id);
-- etc.

-- Add indexes for tenant queries
CREATE INDEX idx_players_tenant ON players(tenant_id);
CREATE INDEX idx_leaderboards_tenant ON leaderboards(tenant_id);
-- etc.
```

## Backend Changes

### 1. Tenant Context

```rust
// src/tenant.rs
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub tenant_slug: String,
    pub settings: TenantSettings,
}

impl TenantContext {
    pub fn from_request(req: &Request) -> Result<Self, Error> {
        // Extract from X-Tenant-Id header or JWT claim
    }
}
```

### 2. Middleware

```rust
// Tenant extraction middleware
pub async fn tenant_middleware(req: Request, next: Next) -> Response {
    let tenant = match extract_tenant(&req) {
        Ok(t) => t,
        Err(e) => return Response::unauthorized(e),
    };

    req.extensions_mut().insert(tenant);
    next.run(req).await
}

fn extract_tenant(req: &Request) -> Result<TenantContext, Error> {
    // 1. Check X-Tenant-Id header
    // 2. Check JWT tenant claim
    // 3. Check API key's tenant
    // 4. Return error if none found
}
```

### 3. Service Changes

All services need tenant scoping:

```rust
// Before
impl Leaderboards {
    pub fn get_top(&self, leaderboard_id: &str, limit: usize) -> Vec<Record> {
        // Query all records
    }
}

// After
impl Leaderboards {
    pub fn get_top(&self, tenant_id: Uuid, leaderboard_id: &str, limit: usize) -> Vec<Record> {
        // Query records WHERE tenant_id = $1
    }
}
```

### 4. Console API Endpoints

```
POST   /api/tenants              - Create tenant
GET    /api/tenants              - List user's tenants
GET    /api/tenants/:id          - Get tenant details
PUT    /api/tenants/:id          - Update tenant
DELETE /api/tenants/:id          - Delete tenant

POST   /api/tenants/:id/members  - Invite member
GET    /api/tenants/:id/members  - List members
PUT    /api/tenants/:id/members/:user_id - Update member role
DELETE /api/tenants/:id/members/:user_id - Remove member

POST   /api/tenants/:id/api-keys - Create API key
GET    /api/tenants/:id/api-keys - List API keys
DELETE /api/tenants/:id/api-keys/:key_id - Revoke API key
```

## Console UI Changes

### 1. Tenant Context Provider

```tsx
// src/contexts/TenantContext.tsx
interface TenantContextValue {
  currentTenant: Tenant | null;
  tenants: Tenant[];
  switchTenant: (tenantId: string) => void;
  isLoading: boolean;
}

export function TenantProvider({ children }: { children: ReactNode }) {
  const [currentTenant, setCurrentTenant] = useState<Tenant | null>(null);
  const [tenants, setTenants] = useState<Tenant[]>([]);

  // Load tenants on mount
  // Store selected tenant in localStorage
  // Include tenant in all API calls
}
```

### 2. Tenant Selector Component

```tsx
// In Layout top bar
<TenantSelector
  tenants={tenants}
  current={currentTenant}
  onSelect={switchTenant}
/>
```

### 3. API Client Updates

```tsx
// src/api/client.ts
class ApiClient {
  private tenantId: string | null = null;

  setTenant(tenantId: string) {
    this.tenantId = tenantId;
  }

  async request(path: string, options: RequestInit) {
    const headers = {
      ...options.headers,
      'X-Tenant-Id': this.tenantId,
    };
    // ...
  }
}
```

### 4. New Pages

- `src/pages/Tenants.tsx` - Tenant management (create, settings)
- `src/pages/TenantMembers.tsx` - Member management
- `src/pages/TenantApiKeys.tsx` - API key management

## Auth Flow

### Login Response

```json
{
  "token": "jwt...",
  "user": { "id": "...", "username": "..." },
  "tenants": [
    { "id": "...", "name": "My Project", "slug": "my-project", "role": "owner" },
    { "id": "...", "name": "Team Project", "slug": "team", "role": "member" }
  ]
}
```

### Tenant Selection

1. After login, UI shows tenant selector if user has multiple tenants
2. Selected tenant stored in localStorage
3. All subsequent API calls include `X-Tenant-Id` header
4. User can switch tenants from top bar dropdown

### JWT Claims (Optional)

```json
{
  "sub": "user-id",
  "tenant_id": "tenant-id",
  "tenant_role": "admin",
  "exp": 1234567890
}
```

## Migration Strategy

### Phase 1: Schema Changes
1. Add `tenant_id` columns (nullable initially)
2. Create default tenant for existing data
3. Update existing records with default tenant_id
4. Make `tenant_id` NOT NULL

### Phase 2: Backend Updates
1. Add tenant middleware
2. Update all services to accept tenant_id
3. Update all queries to filter by tenant
4. Add tenant management endpoints

### Phase 3: UI Updates
1. Add TenantContext provider
2. Add tenant selector to layout
3. Update API client
4. Add tenant management pages

### Phase 4: Testing & Rollout
1. Test tenant isolation thoroughly
2. Test cross-tenant access prevention
3. Performance test with multiple tenants
4. Gradual rollout with feature flag

## Security Considerations

1. **Tenant Isolation**: Every query MUST include tenant_id filter
2. **Cross-Tenant Access**: Validate tenant access on every request
3. **API Keys**: Scoped to single tenant, cannot access other tenants
4. **Audit Logging**: Log tenant context with all actions
5. **Rate Limiting**: Per-tenant rate limits to prevent abuse

## Future Enhancements

- [ ] Custom domains per tenant
- [ ] Tenant-level billing/quotas
- [ ] Tenant data export
- [ ] Tenant deletion with data cleanup
- [ ] Cross-tenant analytics (for platform admins)
- [ ] Tenant templates/presets
