//! KaosNet HTTP client for REST API calls.

use crate::error::{Error, Result};
use crate::session::Session;
use crate::socket::KaosSocket;
use crate::types::*;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// KaosNet client for connecting to a game server.
///
/// # Example
///
/// ```rust,no_run
/// use kaosnet_rs::KaosClient;
///
/// # async fn example() -> kaosnet_rs::Result<()> {
/// let client = KaosClient::new("localhost", 7350);
/// let session = client.authenticate_device("my-device-id").await?;
/// println!("Logged in as: {}", session.user_id);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct KaosClient {
    http: Client,
    base_url: String,
    ws_url: String,
    timeout: Duration,
}

impl KaosClient {
    /// Create a new client with default options.
    pub fn new(host: &str, port: u16) -> Self {
        Self::builder()
            .host(host)
            .port(port)
            .build()
    }

    /// Create a new client with SSL enabled.
    pub fn new_ssl(host: &str, port: u16) -> Self {
        Self::builder()
            .host(host)
            .port(port)
            .use_ssl(true)
            .build()
    }

    /// Create a client builder for advanced configuration.
    pub fn builder() -> KaosClientBuilder {
        KaosClientBuilder::default()
    }

    // ========================================================================
    // Internal HTTP helpers
    // ========================================================================

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<impl Serialize>,
        session: Option<&Session>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        let mut req = self.http.request(method, &url);

        if let Some(s) = session {
            req = req.header("Authorization", s.auth_header());
        }

        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();

            // Try to parse error message from JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
                    return Err(Error::server(error));
                }
            }

            return Err(Error::server(format!("HTTP {}: {}", status, text)));
        }

        let result = resp.json().await?;
        Ok(result)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str, session: Option<&Session>) -> Result<T> {
        self.request(reqwest::Method::GET, path, None::<()>, session).await
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: impl Serialize,
        session: Option<&Session>,
    ) -> Result<T> {
        self.request(reqwest::Method::POST, path, Some(body), session).await
    }

    async fn delete<T: DeserializeOwned>(
        &self,
        path: &str,
        session: Option<&Session>,
    ) -> Result<T> {
        self.request(reqwest::Method::DELETE, path, None::<()>, session).await
    }

    // ========================================================================
    // Authentication
    // ========================================================================

    /// Authenticate with a device ID (anonymous auth).
    ///
    /// Creates a new account if one doesn't exist for this device.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kaosnet_rs::KaosClient;
    /// # async fn example() -> kaosnet_rs::Result<()> {
    /// let client = KaosClient::new("localhost", 7350);
    /// let session = client.authenticate_device("unique-device-id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authenticate_device(&self, device_id: &str) -> Result<Session> {
        self.authenticate_device_with_options(device_id, true, None).await
    }

    /// Authenticate with a device ID with options.
    pub async fn authenticate_device_with_options(
        &self,
        device_id: &str,
        create: bool,
        username: Option<&str>,
    ) -> Result<Session> {
        #[derive(Serialize)]
        struct Request<'a> {
            device_id: &'a str,
            create: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            username: Option<&'a str>,
        }

        let resp: AuthResponse = self.post(
            "/api/auth/device",
            Request { device_id, create, username },
            None,
        ).await?;

        Ok(Session::from_data(resp.session))
    }

    /// Authenticate with email and password.
    ///
    /// Creates a new account if `create` is true and one doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kaosnet_rs::KaosClient;
    /// # async fn example() -> kaosnet_rs::Result<()> {
    /// let client = KaosClient::new("localhost", 7350);
    /// let session = client.authenticate_email("user@example.com", "password123").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authenticate_email(&self, email: &str, password: &str) -> Result<Session> {
        self.authenticate_email_with_options(email, password, true, None).await
    }

    /// Authenticate with email and password with options.
    pub async fn authenticate_email_with_options(
        &self,
        email: &str,
        password: &str,
        create: bool,
        username: Option<&str>,
    ) -> Result<Session> {
        #[derive(Serialize)]
        struct Request<'a> {
            email: &'a str,
            password: &'a str,
            create: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            username: Option<&'a str>,
        }

        let resp: AuthResponse = self.post(
            "/api/auth/email",
            Request { email, password, create, username },
            None,
        ).await?;

        Ok(Session::from_data(resp.session))
    }

    /// Authenticate with a custom method.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kaosnet_rs::KaosClient;
    /// # async fn example() -> kaosnet_rs::Result<()> {
    /// let client = KaosClient::new("localhost", 7350);
    /// let session = client.authenticate_custom("steam-user-id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authenticate_custom(&self, id: &str) -> Result<Session> {
        self.authenticate_custom_with_options(id, true, None, None).await
    }

    /// Authenticate with a custom method with options.
    pub async fn authenticate_custom_with_options(
        &self,
        id: &str,
        create: bool,
        username: Option<&str>,
        vars: Option<HashMap<String, String>>,
    ) -> Result<Session> {
        #[derive(Serialize)]
        struct Request<'a> {
            id: &'a str,
            create: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            username: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            vars: Option<HashMap<String, String>>,
        }

        let resp: AuthResponse = self.post(
            "/api/auth/custom",
            Request { id, create, username, vars },
            None,
        ).await?;

        Ok(Session::from_data(resp.session))
    }

    /// Refresh an expired session token.
    pub async fn refresh_session(&self, session: &Session) -> Result<Session> {
        let refresh_token = session.refresh_token.as_ref()
            .ok_or_else(|| Error::Auth("Session does not have a refresh token".into()))?;

        #[derive(Serialize)]
        struct Request<'a> {
            refresh_token: &'a str,
        }

        let resp: AuthResponse = self.post(
            "/api/auth/refresh",
            Request { refresh_token },
            None,
        ).await?;

        Ok(Session::from_data(resp.session))
    }

    // ========================================================================
    // Socket
    // ========================================================================

    /// Create a new WebSocket for real-time communication.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kaosnet_rs::KaosClient;
    /// # async fn example() -> kaosnet_rs::Result<()> {
    /// let client = KaosClient::new("localhost", 7350);
    /// let session = client.authenticate_device("device-id").await?;
    /// let socket = client.create_socket();
    /// socket.connect(&session).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_socket(&self) -> KaosSocket {
        KaosSocket::new(&self.ws_url)
    }

    // ========================================================================
    // Matchmaker
    // ========================================================================

    /// Add to matchmaker queue.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kaosnet_rs::KaosClient;
    /// # async fn example() -> kaosnet_rs::Result<()> {
    /// let client = KaosClient::new("localhost", 7350);
    /// let session = client.authenticate_device("device-id").await?;
    ///
    /// let ticket = client.add_matchmaker(&session, "ranked")
    ///     .string_property("region", "us")
    ///     .numeric_property("skill", 1500.0)
    ///     .min_count(2)
    ///     .max_count(4)
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_matchmaker<'a>(&'a self, session: &'a Session, queue: &'a str) -> MatchmakerAddBuilder<'a> {
        MatchmakerAddBuilder::new(self, session, queue)
    }

    /// Remove from matchmaker queue.
    pub async fn remove_matchmaker(&self, session: &Session) -> Result<()> {
        #[derive(Deserialize)]
        struct Response {
            #[allow(dead_code)]
            message: String,
        }

        let _: Response = self.delete("/api/matchmaker/remove", Some(session)).await?;
        Ok(())
    }

    /// Get current matchmaker ticket.
    pub async fn get_matchmaker_ticket(&self, session: &Session) -> Result<Option<MatchmakerTicket>> {
        let path = format!("/api/matchmaker/tickets/{}", session.user_id);
        match self.get::<MatchmakerTicket>(&path, Some(session)).await {
            Ok(ticket) => Ok(Some(ticket)),
            Err(Error::Server { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// List matchmaker queues with stats.
    pub async fn list_matchmaker_queues(&self, session: &Session) -> Result<Vec<QueueStats>> {
        #[derive(Deserialize)]
        struct Response {
            queues: Vec<QueueStats>,
        }

        let resp: Response = self.get("/api/matchmaker/queues", Some(session)).await?;
        Ok(resp.queues)
    }

    // ========================================================================
    // Storage
    // ========================================================================

    /// Read storage objects.
    pub async fn read_storage_objects<T: DeserializeOwned>(
        &self,
        session: &Session,
        requests: &[StorageReadRequest],
    ) -> Result<Vec<StorageObject<T>>> {
        #[derive(Serialize)]
        struct Request<'a> {
            object_ids: &'a [StorageReadRequest],
        }

        #[derive(Deserialize)]
        struct Response<T> {
            objects: Vec<StorageObject<T>>,
        }

        let resp: Response<T> = self.post(
            "/api/storage/read",
            Request { object_ids: requests },
            Some(session),
        ).await?;

        Ok(resp.objects)
    }

    /// Write storage objects.
    pub async fn write_storage_objects<T: Serialize + DeserializeOwned>(
        &self,
        session: &Session,
        objects: &[StorageWriteRequest<T>],
    ) -> Result<Vec<StorageObject<T>>> {
        #[derive(Serialize)]
        struct Request<'a, T> {
            objects: &'a [StorageWriteRequest<T>],
        }

        #[derive(Deserialize)]
        struct Response<T> {
            objects: Vec<StorageObject<T>>,
        }

        let resp: Response<T> = self.post(
            "/api/storage/write",
            Request { objects },
            Some(session),
        ).await?;

        Ok(resp.objects)
    }

    /// Delete storage objects.
    pub async fn delete_storage_objects(
        &self,
        session: &Session,
        requests: &[StorageDeleteRequest],
    ) -> Result<()> {
        #[derive(Serialize)]
        struct Request<'a> {
            object_ids: &'a [StorageDeleteRequest],
        }

        let _: serde_json::Value = self.post(
            "/api/storage/delete",
            Request { object_ids: requests },
            Some(session),
        ).await?;

        Ok(())
    }

    // ========================================================================
    // Leaderboards
    // ========================================================================

    /// List leaderboard records.
    pub async fn list_leaderboard_records(
        &self,
        session: &Session,
        leaderboard_id: &str,
        limit: Option<u32>,
    ) -> Result<LeaderboardRecordList> {
        let mut path = format!("/api/leaderboards/{}/records", leaderboard_id);
        if let Some(l) = limit {
            path.push_str(&format!("?limit={}", l));
        }

        self.get(&path, Some(session)).await
    }

    /// Write a leaderboard record.
    pub async fn write_leaderboard_record(
        &self,
        session: &Session,
        leaderboard_id: &str,
        score: i64,
        subscore: Option<i64>,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct Request {
            score: i64,
            #[serde(skip_serializing_if = "Option::is_none")]
            subscore: Option<i64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            metadata: Option<serde_json::Value>,
        }

        let path = format!("/api/leaderboards/{}/records", leaderboard_id);
        let _: serde_json::Value = self.post(
            &path,
            Request { score, subscore, metadata },
            Some(session),
        ).await?;

        Ok(())
    }

    // ========================================================================
    // RPC
    // ========================================================================

    /// Call a server RPC function.
    pub async fn rpc<T: DeserializeOwned>(
        &self,
        session: &Session,
        id: &str,
        payload: Option<serde_json::Value>,
    ) -> Result<T> {
        #[derive(Serialize)]
        struct Request {
            #[serde(skip_serializing_if = "Option::is_none")]
            payload: Option<serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct Response<T> {
            payload: T,
        }

        let path = format!("/api/rpc/{}", id);
        let resp: Response<T> = self.post(
            &path,
            Request { payload },
            Some(session),
        ).await?;

        Ok(resp.payload)
    }
}

// ============================================================================
// Builder
// ============================================================================

/// Builder for KaosClient configuration.
#[derive(Debug, Clone)]
pub struct KaosClientBuilder {
    host: String,
    port: u16,
    use_ssl: bool,
    timeout: Duration,
}

impl Default for KaosClientBuilder {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 7350,
            use_ssl: false,
            timeout: Duration::from_secs(10),
        }
    }
}

impl KaosClientBuilder {
    /// Set the server host.
    pub fn host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    /// Set the server port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Enable or disable SSL/TLS.
    pub fn use_ssl(mut self, use_ssl: bool) -> Self {
        self.use_ssl = use_ssl;
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Build the client.
    pub fn build(self) -> KaosClient {
        let protocol = if self.use_ssl { "https" } else { "http" };
        let ws_protocol = if self.use_ssl { "wss" } else { "ws" };

        let base_url = format!("{}://{}:{}", protocol, self.host, self.port);
        // WebSocket is typically on port + 1
        let ws_url = format!("{}://{}:{}", ws_protocol, self.host, self.port + 1);

        let http = Client::builder()
            .timeout(self.timeout)
            .build()
            .expect("Failed to create HTTP client");

        KaosClient {
            http,
            base_url,
            ws_url,
            timeout: self.timeout,
        }
    }
}

// ============================================================================
// Matchmaker Builder
// ============================================================================

/// Builder for matchmaker add requests.
pub struct MatchmakerAddBuilder<'a> {
    client: &'a KaosClient,
    session: &'a Session,
    queue: &'a str,
    query: Option<String>,
    min_count: usize,
    max_count: usize,
    string_properties: HashMap<String, String>,
    numeric_properties: HashMap<String, f64>,
}

impl<'a> MatchmakerAddBuilder<'a> {
    fn new(client: &'a KaosClient, session: &'a Session, queue: &'a str) -> Self {
        Self {
            client,
            session,
            queue,
            query: None,
            min_count: 2,
            max_count: 8,
            string_properties: HashMap::new(),
            numeric_properties: HashMap::new(),
        }
    }

    /// Set the matchmaker query string.
    pub fn query(mut self, query: &str) -> Self {
        self.query = Some(query.to_string());
        self
    }

    /// Set minimum player count.
    pub fn min_count(mut self, count: usize) -> Self {
        self.min_count = count;
        self
    }

    /// Set maximum player count.
    pub fn max_count(mut self, count: usize) -> Self {
        self.max_count = count;
        self
    }

    /// Add a string property for matching.
    pub fn string_property(mut self, key: &str, value: &str) -> Self {
        self.string_properties.insert(key.to_string(), value.to_string());
        self
    }

    /// Add a numeric property for matching.
    pub fn numeric_property(mut self, key: &str, value: f64) -> Self {
        self.numeric_properties.insert(key.to_string(), value);
        self
    }

    /// Send the matchmaker request.
    pub async fn send(self) -> Result<MatchmakerTicket> {
        #[derive(Serialize)]
        struct Request {
            queue: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            query: Option<String>,
            min_count: usize,
            max_count: usize,
            string_properties: HashMap<String, String>,
            numeric_properties: HashMap<String, f64>,
        }

        let resp: MatchmakerAddResponse = self.client.post(
            "/api/matchmaker/add",
            Request {
                queue: self.queue.to_string(),
                query: self.query,
                min_count: self.min_count,
                max_count: self.max_count,
                string_properties: self.string_properties,
                numeric_properties: self.numeric_properties,
            },
            Some(self.session),
        ).await?;

        Ok(resp.ticket)
    }
}
