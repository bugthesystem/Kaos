# KaosNet Console - Design Plan

Admin UI for KaosNet game server. Inspired by Nakama Console.

## Executive Summary

Web-based administration console for monitoring and managing KaosNet game servers. Provides real-time visibility into sessions, rooms, server health, Lua runtime state, and full account/RBAC management.

---

## 1. Feature Analysis

### Nakama Console Features vs KaosNet Mapping

| Nakama Feature | KaosNet Equivalent | Priority | Notes |
|----------------|-------------------|----------|-------|
| Dashboard/Status | Server Status | P0 | Node health, metrics |
| User Management | Session Browser | P0 | View/kick sessions |
| Matches | Room Browser | P0 | View active rooms |
| Console Users | Account Management | P0 | Admin users + RBAC |
| API Keys | API Key Management | P0 | Service authentication |
| Storage | - | P2 | Future enhancement |
| Leaderboards | - | P2 | Future enhancement |
| Chat Messages | - | P2 | Future enhancement |
| Runtime Modules | Lua Scripts | P1 | View loaded scripts |
| Configuration | Server Config | P1 | View/edit config |
| API Explorer | RPC Tester | P1 | Test RPC endpoints |

### Core Features (MVP)

```
┌────────────────────────────────────────────────────────────────┐
│                    KaosNet Console MVP                         │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  1. Dashboard                                                  │
│     ├── Server uptime & version                                │
│     ├── Connected sessions count                               │
│     ├── Active rooms count                                     │
│     ├── Messages/sec throughput                                │
│     ├── Memory usage                                           │
│     └── Tick rate / lag metrics                                │
│                                                                │
│  2. Sessions                                                   │
│     ├── List all connected sessions                            │
│     ├── Search by ID / address                                 │
│     ├── View session details (state, metadata, latency)        │
│     ├── Kick session                                           │
│     └── View session's rooms                                   │
│                                                                │
│  3. Rooms                                                      │
│     ├── List all active rooms                                  │
│     ├── Search by ID / label                                   │
│     ├── View room details (state, tick rate, players)          │
│     ├── View room state (JSON)                                 │
│     ├── List players in room                                   │
│     └── Terminate room                                         │
│                                                                │
│  4. Lua Runtime                                                │
│     ├── List loaded scripts                                    │
│     ├── View registered RPCs                                   │
│     ├── View registered hooks (before/after)                   │
│     └── View match handlers                                    │
│                                                                │
│  5. Account Management                                         │
│     ├── List console users                                     │
│     ├── Create / edit / delete users                           │
│     ├── Role assignment (Admin, Developer, Viewer)             │
│     ├── Password management                                    │
│     └── Session management (logout all)                        │
│                                                                │
│  6. API Keys                                                   │
│     ├── List API keys                                          │
│     ├── Create / revoke keys                                   │
│     ├── Key permissions (scopes)                               │
│     ├── Usage statistics                                       │
│     └── Expiration management                                  │
│                                                                │
│  7. Configuration                                              │
│     ├── View server config                                     │
│     └── Export as YAML/JSON                                    │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## 2. Technology Stack

### Backend: kaos-http

New crate for HTTP/2 API layer:

```
kaos-http/
├── src/
│   ├── lib.rs           # Public API
│   ├── server.rs        # HTTP/2 server
│   ├── router.rs        # Request routing
│   ├── middleware.rs    # Auth, CORS, logging
│   └── error.rs         # Error types
├── Cargo.toml
└── README.md
```

**Dependencies:**
- `hyper` v1 - HTTP/1.1 + HTTP/2 support
- `http-body-util` - Body utilities
- `hyper-util` - Server utilities
- `rustls` - TLS (optional)

### Frontend: React + Vite

Modern, fast development with production optimization:

```
console-ui/
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   ├── Header.tsx
│   │   │   └── Layout.tsx
│   │   ├── ui/
│   │   │   ├── Button.tsx
│   │   │   ├── Card.tsx
│   │   │   ├── Table.tsx
│   │   │   ├── Modal.tsx
│   │   │   └── Input.tsx
│   │   └── features/
│   │       ├── Dashboard.tsx
│   │       ├── Sessions.tsx
│   │       ├── Rooms.tsx
│   │       ├── Accounts.tsx
│   │       └── ApiKeys.tsx
│   ├── hooks/
│   │   ├── useApi.ts
│   │   └── useAuth.ts
│   ├── stores/
│   │   └── auth.ts
│   └── styles/
│       └── globals.css
├── index.html
├── vite.config.ts
├── tailwind.config.js
└── package.json
```

**Dependencies:**
- `react` + `react-dom` v18
- `react-router-dom` v6
- `@tanstack/react-query` - Data fetching
- `zustand` - State management (tiny)
- `tailwindcss` - Utility CSS
- `lucide-react` - Icons

### Design System

Minimalist, sleek, professional:

```css
/* Tailwind Config */
colors: {
  bg: {
    primary: '#09090b',      /* zinc-950 */
    secondary: '#18181b',    /* zinc-900 */
    tertiary: '#27272a',     /* zinc-800 */
  },
  border: '#3f3f46',         /* zinc-700 */
  text: {
    primary: '#fafafa',      /* zinc-50 */
    secondary: '#a1a1aa',    /* zinc-400 */
    muted: '#71717a',        /* zinc-500 */
  },
  accent: '#3b82f6',         /* blue-500 */
  success: '#22c55e',        /* green-500 */
  warning: '#f59e0b',        /* amber-500 */
  danger: '#ef4444',         /* red-500 */
}
```

**UI Principles:**
- Dark mode only (gaming aesthetic)
- Generous whitespace
- Subtle borders, no heavy shadows
- Monospace for data/IDs
- Smooth transitions (150ms)
- Minimal color, accent sparingly

---

## 3. Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         KaosNet Server                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐          │
│  │  Game Port   │    │ Console API  │    │  Metrics     │          │
│  │  (UDP 7350)  │    │ (HTTP/2 7351)│    │  (optional)  │          │
│  │  kaos-rudp   │    │  kaos-http   │    │              │          │
│  └──────┬───────┘    └──────┬───────┘    └──────────────┘          │
│         │                   │                                       │
│         │            ┌──────▼───────┐                               │
│         │            │   Router     │                               │
│         │            │  + Middleware│                               │
│         │            │  (Auth/RBAC) │                               │
│         │            └──────┬───────┘                               │
│         │                   │                                       │
│  ┌──────▼───────────────────▼───────────────────────────────┐      │
│  │                    Shared State                           │      │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ │      │
│  │  │ Sessions  │ │  Rooms    │ │ Lua Pool  │ │ Accounts  │ │      │
│  │  │ Registry  │ │ Registry  │ │           │ │ + API Keys│ │      │
│  │  └───────────┘ └───────────┘ └───────────┘ └───────────┘ │      │
│  └──────────────────────────────────────────────────────────┘      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                      Console Frontend                               │
├─────────────────────────────────────────────────────────────────────┤
│  React + Vite (built, served as static files or embedded)          │
│  HTTP/2 → kaos-http API                                             │
└─────────────────────────────────────────────────────────────────────┘
```

### Console API Endpoints

```
# Static / UI
GET  /                          # Serve React app (index.html)
GET  /assets/*                  # Static assets

# Authentication
POST /api/auth/login            # Login → JWT
POST /api/auth/logout           # Invalidate session
POST /api/auth/refresh          # Refresh token
GET  /api/auth/me               # Current user info

# Status
GET  /api/status                # Server status/metrics
GET  /api/config                # Server configuration

# Sessions
GET  /api/sessions              # List sessions (paginated)
GET  /api/sessions/:id          # Session details
POST /api/sessions/:id/kick     # Kick session

# Rooms
GET  /api/rooms                 # List rooms (paginated)
GET  /api/rooms/:id             # Room details
GET  /api/rooms/:id/state       # Room state (JSON)
GET  /api/rooms/:id/players     # Players in room
POST /api/rooms/:id/terminate   # Terminate room

# Lua Runtime
GET  /api/lua/scripts           # Loaded scripts
GET  /api/lua/rpcs              # Registered RPCs
GET  /api/lua/hooks             # Registered hooks
POST /api/lua/rpc/:name         # Execute RPC (testing)

# Account Management (Admin only)
GET  /api/accounts              # List console users
POST /api/accounts              # Create user
GET  /api/accounts/:id          # Get user
PUT  /api/accounts/:id          # Update user
DELETE /api/accounts/:id        # Delete user
POST /api/accounts/:id/password # Change password

# API Keys
GET  /api/keys                  # List API keys
POST /api/keys                  # Create API key
GET  /api/keys/:id              # Get key details
DELETE /api/keys/:id            # Revoke key
GET  /api/keys/:id/usage        # Key usage stats
```

---

## 4. Authentication & Authorization

### Authentication Methods

```
1. Console Users (JWT)
   - Username/password login
   - JWT with 1h expiry, refresh token
   - Stored in httpOnly cookie or Authorization header

2. API Keys (Bearer)
   - For service-to-service calls
   - Authorization: Bearer <api_key>
   - Scoped permissions
```

### RBAC Roles

```rust
#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Admin,      // Full access
    Developer,  // Read + RPC testing + room management
    Viewer,     // Read-only access
}

impl Role {
    pub fn can_manage_accounts(&self) -> bool {
        matches!(self, Role::Admin)
    }

    pub fn can_manage_api_keys(&self) -> bool {
        matches!(self, Role::Admin)
    }

    pub fn can_kick_sessions(&self) -> bool {
        matches!(self, Role::Admin | Role::Developer)
    }

    pub fn can_terminate_rooms(&self) -> bool {
        matches!(self, Role::Admin | Role::Developer)
    }

    pub fn can_execute_rpc(&self) -> bool {
        matches!(self, Role::Admin | Role::Developer)
    }

    pub fn can_view(&self) -> bool {
        true // All roles can view
    }
}
```

### Permission Matrix

| Action | Admin | Developer | Viewer |
|--------|-------|-----------|--------|
| View dashboard | ✓ | ✓ | ✓ |
| View sessions | ✓ | ✓ | ✓ |
| Kick sessions | ✓ | ✓ | ✗ |
| View rooms | ✓ | ✓ | ✓ |
| Terminate rooms | ✓ | ✓ | ✗ |
| View Lua scripts | ✓ | ✓ | ✓ |
| Execute RPC | ✓ | ✓ | ✗ |
| Manage accounts | ✓ | ✗ | ✗ |
| Manage API keys | ✓ | ✗ | ✗ |
| View config | ✓ | ✓ | ✓ |

### API Key Scopes

```rust
bitflags! {
    pub struct ApiKeyScope: u32 {
        const READ_STATUS   = 0b0000_0001;
        const READ_SESSIONS = 0b0000_0010;
        const READ_ROOMS    = 0b0000_0100;
        const KICK_SESSIONS = 0b0000_1000;
        const TERMINATE_ROOMS = 0b0001_0000;
        const EXECUTE_RPC   = 0b0010_0000;
        const READ_ALL = Self::READ_STATUS.bits()
                       | Self::READ_SESSIONS.bits()
                       | Self::READ_ROOMS.bits();
        const FULL = 0xFFFF_FFFF;
    }
}
```

---

## 5. Data Models

### Account

```rust
#[derive(Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,    // argon2
    pub role: Role,
    pub created_at: i64,
    pub last_login: Option<i64>,
    pub disabled: bool,
}
```

### API Key

```rust
#[derive(Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub key_hash: String,         // sha256
    pub key_prefix: String,       // First 8 chars for identification
    pub scopes: ApiKeyScope,
    pub created_by: Uuid,         // Account ID
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used: Option<i64>,
    pub request_count: u64,
    pub disabled: bool,
}
```

### API Responses

```rust
// POST /api/auth/login
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: i64,
    pub user: AccountInfo,
}

#[derive(Serialize)]
pub struct AccountInfo {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

// POST /api/keys (create key)
#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id: Uuid,
    pub key: String,              // Only shown once!
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<i64>,
}

// GET /api/keys
#[derive(Serialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,       // "kn_abc123..."
    pub scopes: Vec<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub last_used: Option<i64>,
    pub request_count: u64,
}
```

---

## 6. UI Components

### Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  ┌─────┐  KaosNet                              user@admin  [Logout] │
│  │ ◆◆◆ │                                                            │
├──┴─────┴────────────────────────────────────────────────────────────┤
│         │                                                           │
│    ◆    │   Dashboard                                               │
│  Status │   ─────────────────────────────────────────────────────   │
│         │                                                           │
│    ◆    │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│ Sessions│   │ Sessions │  │  Rooms   │  │ Msg/sec  │  │  Memory  │ │
│         │   │  1,234   │  │    89    │  │  12.3K   │  │  234 MB  │ │
│    ◆    │   └──────────┘  └──────────┘  └──────────┘  └──────────┘ │
│  Rooms  │                                                           │
│         │   Server Uptime: 2d 14h 32m                               │
│    ◆    │   Version: 0.1.0                                          │
│   Lua   │                                                           │
│         │                                                           │
│  ────── │                                                           │
│         │                                                           │
│    ◆    │                                                           │
│ Accounts│                                                           │
│         │                                                           │
│    ◆    │                                                           │
│ API Keys│                                                           │
│         │                                                           │
│    ◆    │                                                           │
│ Settings│                                                           │
│         │                                                           │
└─────────┴───────────────────────────────────────────────────────────┘
```

### Component Library

```tsx
// Button variants
<Button variant="primary">Create</Button>
<Button variant="secondary">Cancel</Button>
<Button variant="danger">Delete</Button>
<Button variant="ghost">View</Button>

// Card
<Card>
  <Card.Header>Sessions</Card.Header>
  <Card.Value>1,234</Card.Value>
  <Card.Trend direction="up">+12%</Card.Trend>
</Card>

// Table
<Table>
  <Table.Header>
    <Table.Column>ID</Table.Column>
    <Table.Column>Address</Table.Column>
    <Table.Column>State</Table.Column>
    <Table.Column align="right">Actions</Table.Column>
  </Table.Header>
  <Table.Body>
    {sessions.map(s => (
      <Table.Row key={s.id}>
        <Table.Cell mono>{s.id}</Table.Cell>
        <Table.Cell>{s.address}</Table.Cell>
        <Table.Cell><Badge variant="success">Connected</Badge></Table.Cell>
        <Table.Cell align="right">
          <Button variant="danger" size="sm">Kick</Button>
        </Table.Cell>
      </Table.Row>
    ))}
  </Table.Body>
</Table>

// Modal
<Modal open={open} onClose={onClose}>
  <Modal.Header>Confirm Action</Modal.Header>
  <Modal.Body>Are you sure you want to kick this session?</Modal.Body>
  <Modal.Footer>
    <Button variant="secondary" onClick={onClose}>Cancel</Button>
    <Button variant="danger" onClick={onConfirm}>Kick</Button>
  </Modal.Footer>
</Modal>
```

### Pages

```
/                    → Dashboard
/sessions            → Sessions list
/sessions/:id        → Session detail
/rooms               → Rooms list
/rooms/:id           → Room detail
/lua                 → Lua scripts/RPCs
/lua/rpc-tester      → RPC testing tool
/accounts            → Account management (admin)
/api-keys            → API key management (admin)
/settings            → Server config view
/login               → Login page
```

---

## 7. kaos-http Crate Design

### Core Types

```rust
// lib.rs
pub mod server;
pub mod router;
pub mod middleware;
pub mod request;
pub mod response;
pub mod error;

pub use server::HttpServer;
pub use router::Router;
pub use request::Request;
pub use response::Response;
pub use error::HttpError;

// Simple handler type
pub type Handler = Box<dyn Fn(Request) -> Response + Send + Sync>;
```

### Router

```rust
// router.rs
pub struct Router {
    routes: Vec<Route>,
    middleware: Vec<Box<dyn Middleware>>,
}

impl Router {
    pub fn new() -> Self { ... }

    pub fn get(&mut self, path: &str, handler: impl Handler) -> &mut Self { ... }
    pub fn post(&mut self, path: &str, handler: impl Handler) -> &mut Self { ... }
    pub fn put(&mut self, path: &str, handler: impl Handler) -> &mut Self { ... }
    pub fn delete(&mut self, path: &str, handler: impl Handler) -> &mut Self { ... }

    pub fn group(&mut self, prefix: &str) -> RouterGroup { ... }
    pub fn middleware(&mut self, mw: impl Middleware) -> &mut Self { ... }
}

// Usage
let mut router = Router::new();

router
    .middleware(CorsMiddleware::new())
    .middleware(AuthMiddleware::new(auth_service));

router.group("/api")
    .get("/status", handlers::get_status)
    .get("/sessions", handlers::list_sessions)
    .post("/sessions/:id/kick", handlers::kick_session);
```

### Server

```rust
// server.rs
pub struct HttpServer {
    router: Router,
    addr: SocketAddr,
}

impl HttpServer {
    pub fn bind(addr: impl ToSocketAddrs) -> io::Result<Self> { ... }

    pub fn router(&mut self, router: Router) -> &mut Self { ... }

    pub fn serve(self) -> io::Result<()> { ... }

    // HTTP/2 with TLS
    pub fn serve_tls(self, cert: &Path, key: &Path) -> io::Result<()> { ... }
}
```

### Middleware

```rust
// middleware.rs
pub trait Middleware: Send + Sync {
    fn handle(&self, req: Request, next: Next) -> Response;
}

// Auth middleware
pub struct AuthMiddleware {
    auth: Arc<AuthService>,
}

impl Middleware for AuthMiddleware {
    fn handle(&self, mut req: Request, next: Next) -> Response {
        // Check JWT or API key
        match self.auth.authenticate(&req) {
            Ok(identity) => {
                req.extensions_mut().insert(identity);
                next.run(req)
            }
            Err(_) => Response::unauthorized(),
        }
    }
}

// CORS middleware
pub struct CorsMiddleware { ... }

// Logging middleware
pub struct LoggingMiddleware { ... }
```

---

## 8. Implementation Phases

### Phase 1: kaos-http Crate (Week 1-2)

```
Tasks:
- [ ] Create kaos-http crate structure
- [ ] HTTP/2 server with hyper
- [ ] Router with path params
- [ ] Middleware chain
- [ ] Request/Response types
- [ ] JSON body parsing
- [ ] Static file serving
- [ ] Error handling
- [ ] Basic tests
```

### Phase 2: Console Backend (Week 2-3)

```
Tasks:
- [ ] Account storage (in-memory + file persistence)
- [ ] Password hashing (argon2)
- [ ] JWT generation/validation
- [ ] API key generation
- [ ] Auth middleware
- [ ] RBAC middleware
- [ ] /api/auth/* endpoints
- [ ] /api/accounts/* endpoints
- [ ] /api/keys/* endpoints
- [ ] /api/status endpoint
- [ ] /api/sessions/* endpoints
- [ ] /api/rooms/* endpoints
- [ ] /api/lua/* endpoints
```

### Phase 3: React Frontend Setup (Week 3)

```
Tasks:
- [ ] Vite + React + TypeScript setup
- [ ] Tailwind CSS configuration
- [ ] Base component library
- [ ] Layout components
- [ ] Auth context + login page
- [ ] React Query setup
- [ ] API client with auth
```

### Phase 4: Frontend Features (Week 4-5)

```
Tasks:
- [ ] Dashboard page
- [ ] Sessions list + detail
- [ ] Rooms list + detail
- [ ] Lua scripts viewer
- [ ] RPC tester
- [ ] Accounts management (admin)
- [ ] API keys management (admin)
- [ ] Settings page
```

### Phase 5: Integration & Polish (Week 5-6)

```
Tasks:
- [ ] Build React → embed in Rust binary
- [ ] Real-time updates (SSE or polling)
- [ ] Error handling + toasts
- [ ] Loading states
- [ ] Responsive design
- [ ] Production build optimization
- [ ] Documentation
```

---

## 9. Module Structure

```
Kaos/
├── kaos-http/                     # HTTP/2 transport crate
│   ├── src/
│   │   ├── lib.rs
│   │   ├── server.rs
│   │   ├── router.rs
│   │   ├── middleware.rs
│   │   ├── request.rs
│   │   ├── response.rs
│   │   └── error.rs
│   ├── examples/
│   │   └── hello_http2.rs
│   └── Cargo.toml
│
├── prototype/kaosnet/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── console/               # Console backend
│   │   │   ├── mod.rs
│   │   │   ├── server.rs          # Console HTTP server
│   │   │   ├── auth/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── jwt.rs
│   │   │   │   ├── api_key.rs
│   │   │   │   └── rbac.rs
│   │   │   ├── handlers/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── auth.rs
│   │   │   │   ├── status.rs
│   │   │   │   ├── sessions.rs
│   │   │   │   ├── rooms.rs
│   │   │   │   ├── lua.rs
│   │   │   │   ├── accounts.rs
│   │   │   │   └── api_keys.rs
│   │   │   ├── storage/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── accounts.rs
│   │   │   │   └── api_keys.rs
│   │   │   └── types.rs
│   │   └── ...
│   │
│   └── console-ui/                # React frontend
│       ├── src/
│       │   ├── main.tsx
│       │   ├── App.tsx
│       │   ├── api/
│       │   │   └── client.ts
│       │   ├── components/
│       │   │   ├── ui/
│       │   │   └── features/
│       │   ├── hooks/
│       │   ├── pages/
│       │   └── stores/
│       ├── index.html
│       ├── vite.config.ts
│       ├── tailwind.config.js
│       └── package.json
```

---

## 10. Dependencies

### kaos-http

```toml
[package]
name = "kaos-http"
version = "0.1.0-preview"

[dependencies]
hyper = { version = "1", features = ["http1", "http2", "server"] }
hyper-util = { version = "0.1", features = ["server", "tokio"] }
http-body-util = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "net", "macros"] }
bytes = "1"
thiserror = "2"
parking_lot = "0.12"

[dependencies.rustls]
version = "0.23"
optional = true

[features]
default = []
tls = ["rustls", "hyper-rustls"]
```

### kaosnet console feature

```toml
[features]
console = ["kaos-http", "jsonwebtoken", "argon2"]

[dependencies]
kaos-http = { path = "../kaos-http", optional = true }
jsonwebtoken = { version = "9", optional = true }
argon2 = { version = "0.5", optional = true }
```

### console-ui (package.json)

```json
{
  "name": "kaosnet-console",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "react-router-dom": "^6.26.0",
    "@tanstack/react-query": "^5.50.0",
    "zustand": "^4.5.0",
    "lucide-react": "^0.400.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0"
  }
}
```

---

## 11. Security Considerations

### Authentication Security
- Passwords hashed with Argon2id
- JWT signed with HS256/RS256
- Tokens expire in 1 hour
- Refresh tokens in httpOnly cookies
- API keys are 32 random bytes, base64url encoded

### API Key Security
- Keys shown only once on creation
- Stored as SHA-256 hash
- Prefix stored for identification ("kn_abc12...")
- Automatic expiration support
- Usage tracking for auditing

### Network Security
- Console binds to `127.0.0.1:7351` by default
- HTTP/2 with TLS for production
- CORS restricted to console origin
- Rate limiting on auth endpoints

### Input Validation
- All inputs sanitized
- UUID/ID format validation
- JSON schema validation
- No SQL (in-memory storage)

---

## 12. Success Criteria

### MVP Complete When:
1. kaos-http serves HTTP/2 with routing
2. Console backend has full auth flow
3. RBAC enforced on all endpoints
4. API keys working for service auth
5. React UI has all core pages
6. Frontend builds and embeds in Rust
7. Single binary deployment works

### Performance Targets:
- API response < 50ms (p99)
- UI initial load < 1s
- Memory overhead < 20MB
- No blocking on game server thread

---

## Summary

KaosNet Console uses a modern stack:
- **kaos-http**: New HTTP/2 crate for the API layer
- **React + Vite**: Fast, modern frontend with Tailwind
- **Full auth**: JWT for users, API keys for services
- **RBAC**: Admin/Developer/Viewer roles

The architecture separates concerns cleanly while maintaining the Kaos philosophy of performance and minimal bloat. The frontend is built separately but can be embedded in the final binary for single-file deployment.
