//! WebSocket client for real-time communication.

use crate::error::{Error, Result};
use crate::session::Session;
use crate::types::*;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

/// Callback types for real-time events.
pub type ChatMessageHandler = Box<dyn Fn(ChatMessage) + Send + Sync>;
pub type MatchDataHandler = Box<dyn Fn(MatchData) + Send + Sync>;
pub type MatchPresenceHandler = Box<dyn Fn(MatchPresenceEvent) + Send + Sync>;
pub type MatchmakerMatchedHandler = Box<dyn Fn(MatchmakerMatched) + Send + Sync>;
pub type NotificationHandler = Box<dyn Fn(Notification) + Send + Sync>;
pub type StatusPresenceHandler = Box<dyn Fn(StatusPresenceEvent) + Send + Sync>;
pub type ErrorHandler = Box<dyn Fn(Error) + Send + Sync>;
pub type DisconnectHandler = Box<dyn Fn() + Send + Sync>;
pub type ReconnectHandler = Box<dyn Fn(u32) + Send + Sync>;

/// Socket reconnection configuration.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts (0 = disabled, -1 = infinite)
    pub max_attempts: i32,
    /// Initial delay between reconnection attempts in milliseconds.
    pub initial_delay_ms: u64,
    /// Maximum delay between reconnection attempts in milliseconds.
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff (e.g., 2.0 doubles delay each attempt).
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Match presence event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPresenceEvent {
    pub match_id: String,
    pub joins: Vec<UserPresence>,
    pub leaves: Vec<UserPresence>,
}

/// Matchmaker matched event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerMatched {
    pub ticket: String,
    pub match_id: String,
    pub token: String,
    pub users: Vec<MatchmakerUser>,
}

/// User in matchmaker match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerUser {
    pub presence: UserPresence,
    pub string_properties: HashMap<String, String>,
    pub numeric_properties: HashMap<String, f64>,
}

/// Status presence event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPresenceEvent {
    pub joins: Vec<UserPresence>,
    pub leaves: Vec<UserPresence>,
}

/// Channel info returned when joining.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub presences: Vec<UserPresence>,
    #[serde(rename = "self")]
    pub self_presence: UserPresence,
}

/// WebSocket envelope for messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Envelope {
    cid: Option<String>,
    #[serde(flatten)]
    message: EnvelopeMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EnvelopeMessage {
    // Outgoing
    ChannelJoin(ChannelJoinMessage),
    ChannelLeave(ChannelLeaveMessage),
    ChannelMessageSend(ChannelMessageSendMessage),
    MatchJoin(MatchJoinMessage),
    MatchLeave(MatchLeaveMessage),
    MatchDataSend(MatchDataSendMessage),
    MatchmakerAdd(MatchmakerAddMessage),
    MatchmakerRemove(MatchmakerRemoveMessage),
    StatusUpdate(StatusUpdateMessage),
    StatusFollow(StatusFollowMessage),
    StatusUnfollow(StatusUnfollowMessage),
    Rpc(RpcMessage),

    // Incoming
    Channel(Channel),
    ChannelMessage(ChatMessage),
    Match(Match),
    MatchData(MatchData),
    MatchPresenceEvent(MatchPresenceEvent),
    MatchmakerTicket(MatchmakerTicket),
    MatchmakerMatched(MatchmakerMatched),
    StatusPresenceEvent(StatusPresenceEvent),
    Notifications(NotificationList),
    Error(ServerError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelJoinMessage {
    target: String,
    #[serde(rename = "type")]
    channel_type: i32,
    persistence: bool,
    hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelLeaveMessage {
    channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelMessageSendMessage {
    channel_id: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchJoinMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    match_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchLeaveMessage {
    match_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchDataSendMessage {
    match_id: String,
    op_code: i64,
    data: String, // Base64 encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    presences: Option<Vec<UserPresence>>,
    reliable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchmakerAddMessage {
    min_count: i32,
    max_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    string_properties: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    numeric_properties: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchmakerRemoveMessage {
    ticket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusUpdateMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusFollowMessage {
    user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusUnfollowMessage {
    user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcMessage {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotificationList {
    notifications: Vec<Notification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerError {
    code: i32,
    message: String,
}

type ResponseSender = oneshot::Sender<std::result::Result<EnvelopeMessage, Error>>;
type PendingRequests = Arc<Mutex<HashMap<String, ResponseSender>>>;

// Type alias for WebSocket read stream
type WsReadStream = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
    >
>;

/// Spawns a read loop task that processes incoming WebSocket messages.
/// This is extracted as a separate function to allow reuse after reconnection.
#[allow(clippy::too_many_arguments)]
fn spawn_read_loop(
    mut read: WsReadStream,
    pending: PendingRequests,
    connected: Arc<RwLock<bool>>,
    on_chat_message: Arc<RwLock<Option<ChatMessageHandler>>>,
    on_match_data: Arc<RwLock<Option<MatchDataHandler>>>,
    on_match_presence: Arc<RwLock<Option<MatchPresenceHandler>>>,
    on_matchmaker_matched: Arc<RwLock<Option<MatchmakerMatchedHandler>>>,
    on_notification: Arc<RwLock<Option<NotificationHandler>>>,
    on_status_presence: Arc<RwLock<Option<StatusPresenceHandler>>>,
    on_error: Arc<RwLock<Option<ErrorHandler>>>,
    on_disconnect: Arc<RwLock<Option<DisconnectHandler>>>,
    on_reconnect: Arc<RwLock<Option<ReconnectHandler>>>,
    reconnect_config: Arc<RwLock<ReconnectConfig>>,
    reconnect_attempts: Arc<RwLock<u32>>,
    stored_token: Arc<RwLock<Option<String>>>,
    reconnecting: Arc<RwLock<bool>>,
    sender_ref: Arc<Mutex<Option<mpsc::Sender<WsMessage>>>>,
    base_url: String,
) {
    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    if let Ok(envelope) = serde_json::from_str::<Envelope>(&text) {
                        // Check if this is a response to a pending request
                        if let Some(cid) = &envelope.cid {
                            let mut pending = pending.lock().await;
                            if let Some(sender) = pending.remove(cid) {
                                let _ = sender.send(Ok(envelope.message));
                                continue;
                            }
                        }

                        // Handle event
                        match envelope.message {
                            EnvelopeMessage::ChannelMessage(msg) => {
                                if let Some(handler) = on_chat_message.read().await.as_ref() {
                                    handler(msg);
                                }
                            }
                            EnvelopeMessage::MatchData(data) => {
                                if let Some(handler) = on_match_data.read().await.as_ref() {
                                    handler(data);
                                }
                            }
                            EnvelopeMessage::MatchPresenceEvent(event) => {
                                if let Some(handler) = on_match_presence.read().await.as_ref() {
                                    handler(event);
                                }
                            }
                            EnvelopeMessage::MatchmakerMatched(matched) => {
                                if let Some(handler) = on_matchmaker_matched.read().await.as_ref() {
                                    handler(matched);
                                }
                            }
                            EnvelopeMessage::StatusPresenceEvent(event) => {
                                if let Some(handler) = on_status_presence.read().await.as_ref() {
                                    handler(event);
                                }
                            }
                            EnvelopeMessage::Notifications(list) => {
                                if let Some(handler) = on_notification.read().await.as_ref() {
                                    for n in list.notifications {
                                        handler(n);
                                    }
                                }
                            }
                            EnvelopeMessage::Error(err) => {
                                if let Some(handler) = on_error.read().await.as_ref() {
                                    handler(Error::server_code(&err.message, err.code));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Mark as disconnected
        {
            let mut c = connected.write().await;
            *c = false;
        }

        // Attempt reconnection
        let config = reconnect_config.read().await.clone();
        let token_opt = stored_token.read().await.clone();

        if config.max_attempts != 0 {
            if let Some(token) = token_opt {
                // Check if already reconnecting
                {
                    let is_reconnecting = *reconnecting.read().await;
                    if is_reconnecting {
                        return;
                    }
                    let mut r = reconnecting.write().await;
                    *r = true;
                }

                let mut delay_ms = config.initial_delay_ms;

                loop {
                    let current_attempt = {
                        let mut attempts = reconnect_attempts.write().await;
                        *attempts += 1;
                        *attempts
                    };

                    if config.max_attempts > 0 && current_attempt as i32 > config.max_attempts {
                        break;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

                    let url = format!("{}/ws?token={}", base_url, token);
                    match connect_async(&url).await {
                        Ok((ws_stream, _)) => {
                            let (write, new_read) = ws_stream.split();

                            let (tx, mut new_rx) = mpsc::channel::<WsMessage>(100);

                            {
                                let mut sender = sender_ref.lock().await;
                                *sender = Some(tx);
                            }

                            {
                                let mut c = connected.write().await;
                                *c = true;
                            }

                            {
                                let mut attempts = reconnect_attempts.write().await;
                                *attempts = 0;
                            }

                            {
                                let mut r = reconnecting.write().await;
                                *r = false;
                            }

                            if let Some(handler) = on_reconnect.read().await.as_ref() {
                                handler(current_attempt);
                            }

                            let write = Arc::new(Mutex::new(write));
                            let write_clone = write.clone();
                            tokio::spawn(async move {
                                while let Some(msg) = new_rx.recv().await {
                                    let mut w = write_clone.lock().await;
                                    if w.send(msg).await.is_err() {
                                        break;
                                    }
                                }
                            });

                            // Recursive call to spawn new read loop
                            spawn_read_loop(
                                new_read,
                                pending,
                                connected,
                                on_chat_message,
                                on_match_data,
                                on_match_presence,
                                on_matchmaker_matched,
                                on_notification,
                                on_status_presence,
                                on_error,
                                on_disconnect,
                                on_reconnect,
                                reconnect_config,
                                reconnect_attempts,
                                stored_token,
                                reconnecting,
                                sender_ref,
                                base_url,
                            );

                            return;
                        }
                        Err(_) => {
                            delay_ms = ((delay_ms as f64) * config.backoff_multiplier) as u64;
                            delay_ms = delay_ms.min(config.max_delay_ms);
                        }
                    }
                }

                {
                    let mut r = reconnecting.write().await;
                    *r = false;
                }
            }
        }

        // Call disconnect handler
        if let Some(handler) = on_disconnect.read().await.as_ref() {
            handler();
        }
    });
}

/// WebSocket client for real-time communication.
///
/// # Example
///
/// ```rust,no_run
/// # use kaosnet_rs::{KaosClient, KaosSocket};
/// # async fn example() -> kaosnet_rs::Result<()> {
/// let client = KaosClient::new("localhost", 7350);
/// let session = client.authenticate_device("device-id").await?;
///
/// let socket = client.create_socket();
/// socket.connect(&session).await?;
///
/// // Join a chat room
/// let channel = socket.join_chat("general", 1, true, false).await?;
/// println!("Joined channel: {}", channel.id);
///
/// // Send a message
/// socket.send_chat_message(&channel.id, "Hello, world!").await?;
///
/// // Disconnect
/// socket.disconnect().await;
/// # Ok(())
/// # }
/// ```
pub struct KaosSocket {
    base_url: String,
    sender: Arc<Mutex<Option<mpsc::Sender<WsMessage>>>>,
    pending: PendingRequests,
    cid_counter: AtomicU64,
    connected: Arc<RwLock<bool>>,

    // Reconnection state
    reconnect_config: Arc<RwLock<ReconnectConfig>>,
    reconnect_attempts: Arc<RwLock<u32>>,
    stored_token: Arc<RwLock<Option<String>>>,
    reconnecting: Arc<RwLock<bool>>,

    // Event handlers
    on_chat_message: Arc<RwLock<Option<ChatMessageHandler>>>,
    on_match_data: Arc<RwLock<Option<MatchDataHandler>>>,
    on_match_presence: Arc<RwLock<Option<MatchPresenceHandler>>>,
    on_matchmaker_matched: Arc<RwLock<Option<MatchmakerMatchedHandler>>>,
    on_notification: Arc<RwLock<Option<NotificationHandler>>>,
    on_status_presence: Arc<RwLock<Option<StatusPresenceHandler>>>,
    on_error: Arc<RwLock<Option<ErrorHandler>>>,
    on_disconnect: Arc<RwLock<Option<DisconnectHandler>>>,
    on_reconnect: Arc<RwLock<Option<ReconnectHandler>>>,
}

impl KaosSocket {
    /// Create a new socket (internal, use KaosClient::create_socket).
    pub(crate) fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            sender: Arc::new(Mutex::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            cid_counter: AtomicU64::new(1),
            connected: Arc::new(RwLock::new(false)),
            reconnect_config: Arc::new(RwLock::new(ReconnectConfig::default())),
            reconnect_attempts: Arc::new(RwLock::new(0)),
            stored_token: Arc::new(RwLock::new(None)),
            reconnecting: Arc::new(RwLock::new(false)),
            on_chat_message: Arc::new(RwLock::new(None)),
            on_match_data: Arc::new(RwLock::new(None)),
            on_match_presence: Arc::new(RwLock::new(None)),
            on_matchmaker_matched: Arc::new(RwLock::new(None)),
            on_notification: Arc::new(RwLock::new(None)),
            on_status_presence: Arc::new(RwLock::new(None)),
            on_error: Arc::new(RwLock::new(None)),
            on_disconnect: Arc::new(RwLock::new(None)),
            on_reconnect: Arc::new(RwLock::new(None)),
        }
    }

    /// Configure reconnection behavior.
    pub async fn set_reconnect_config(&self, config: ReconnectConfig) {
        let mut cfg = self.reconnect_config.write().await;
        *cfg = config;
    }

    /// Disable automatic reconnection.
    pub async fn disable_reconnect(&self) {
        let mut cfg = self.reconnect_config.write().await;
        cfg.max_attempts = 0;
    }

    /// Connect to the server.
    pub async fn connect(&self, session: &Session) -> Result<()> {
        // Store token for potential reconnection
        {
            let mut token = self.stored_token.write().await;
            *token = Some(session.token.clone());
        }

        // Reset reconnect attempts on fresh connect
        {
            let mut attempts = self.reconnect_attempts.write().await;
            *attempts = 0;
        }

        self.connect_internal(&session.token).await
    }

    /// Internal connection logic (used for both initial connect and reconnect).
    async fn connect_internal(&self, token: &str) -> Result<()> {
        let url = format!("{}/ws?token={}", self.base_url, token);

        let (ws_stream, _) = connect_async(&url).await?;
        let (write, mut read) = ws_stream.split();

        // Create channel for sending messages
        let (tx, mut rx) = mpsc::channel::<WsMessage>(100);

        // Store sender
        {
            let mut sender = self.sender.lock().await;
            *sender = Some(tx);
        }

        // Mark as connected
        {
            let mut connected = self.connected.write().await;
            *connected = true;
        }

        // Spawn write task
        let write = Arc::new(Mutex::new(write));
        let write_clone = write.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let mut w = write_clone.lock().await;
                if w.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Spawn read task with reconnection logic
        let pending = self.pending.clone();
        let connected = self.connected.clone();
        let on_chat_message = self.on_chat_message.clone();
        let on_match_data = self.on_match_data.clone();
        let on_match_presence = self.on_match_presence.clone();
        let on_matchmaker_matched = self.on_matchmaker_matched.clone();
        let on_notification = self.on_notification.clone();
        let on_status_presence = self.on_status_presence.clone();
        let on_error = self.on_error.clone();
        let on_disconnect = self.on_disconnect.clone();
        let on_reconnect = self.on_reconnect.clone();

        // Reconnection state
        let reconnect_config = self.reconnect_config.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();
        let stored_token = self.stored_token.clone();
        let reconnecting = self.reconnecting.clone();
        let sender_ref = self.sender.clone();
        let base_url = self.base_url.clone();

        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Ok(envelope) = serde_json::from_str::<Envelope>(&text) {
                            // Check if this is a response to a pending request
                            if let Some(cid) = &envelope.cid {
                                let mut pending = pending.lock().await;
                                if let Some(sender) = pending.remove(cid) {
                                    let _ = sender.send(Ok(envelope.message));
                                    continue;
                                }
                            }

                            // Handle event
                            match envelope.message {
                                EnvelopeMessage::ChannelMessage(msg) => {
                                    if let Some(handler) = on_chat_message.read().await.as_ref() {
                                        handler(msg);
                                    }
                                }
                                EnvelopeMessage::MatchData(data) => {
                                    if let Some(handler) = on_match_data.read().await.as_ref() {
                                        handler(data);
                                    }
                                }
                                EnvelopeMessage::MatchPresenceEvent(event) => {
                                    if let Some(handler) = on_match_presence.read().await.as_ref() {
                                        handler(event);
                                    }
                                }
                                EnvelopeMessage::MatchmakerMatched(matched) => {
                                    if let Some(handler) = on_matchmaker_matched.read().await.as_ref() {
                                        handler(matched);
                                    }
                                }
                                EnvelopeMessage::StatusPresenceEvent(event) => {
                                    if let Some(handler) = on_status_presence.read().await.as_ref() {
                                        handler(event);
                                    }
                                }
                                EnvelopeMessage::Notifications(list) => {
                                    if let Some(handler) = on_notification.read().await.as_ref() {
                                        for n in list.notifications {
                                            handler(n);
                                        }
                                    }
                                }
                                EnvelopeMessage::Error(err) => {
                                    if let Some(handler) = on_error.read().await.as_ref() {
                                        handler(Error::server_code(&err.message, err.code));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }

            // Mark as disconnected
            {
                let mut c = connected.write().await;
                *c = false;
            }

            // Attempt reconnection
            let config = reconnect_config.read().await.clone();
            let token_opt = stored_token.read().await.clone();

            if config.max_attempts != 0 {
                if let Some(token) = token_opt {
                    // Check if already reconnecting (prevent multiple reconnection loops)
                    {
                        let is_reconnecting = *reconnecting.read().await;
                        if is_reconnecting {
                            return;
                        }
                        let mut r = reconnecting.write().await;
                        *r = true;
                    }

                    let mut delay_ms = config.initial_delay_ms;

                    loop {
                        let current_attempt = {
                            let mut attempts = reconnect_attempts.write().await;
                            *attempts += 1;
                            *attempts
                        };

                        // Check max attempts (-1 = infinite)
                        if config.max_attempts > 0 && current_attempt as i32 > config.max_attempts {
                            break;
                        }

                        // Wait before attempting reconnection
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

                        // Attempt to reconnect
                        let url = format!("{}/ws?token={}", base_url, token);
                        match connect_async(&url).await {
                            Ok((ws_stream, _)) => {
                                let (write, new_read) = ws_stream.split();

                                // Create new channel for sending
                                let (tx, mut new_rx) = mpsc::channel::<WsMessage>(100);

                                // Update sender
                                {
                                    let mut sender = sender_ref.lock().await;
                                    *sender = Some(tx);
                                }

                                // Mark as connected
                                {
                                    let mut c = connected.write().await;
                                    *c = true;
                                }

                                // Reset reconnect attempts
                                {
                                    let mut attempts = reconnect_attempts.write().await;
                                    *attempts = 0;
                                }

                                // Mark as no longer reconnecting
                                {
                                    let mut r = reconnecting.write().await;
                                    *r = false;
                                }

                                // Call reconnect handler
                                if let Some(handler) = on_reconnect.read().await.as_ref() {
                                    handler(current_attempt);
                                }

                                // Spawn new write task
                                let write = Arc::new(Mutex::new(write));
                                let write_clone = write.clone();
                                tokio::spawn(async move {
                                    while let Some(msg) = new_rx.recv().await {
                                        let mut w = write_clone.lock().await;
                                        if w.send(msg).await.is_err() {
                                            break;
                                        }
                                    }
                                });

                                // Continue reading on new stream (recursive-like behavior)
                                // We need to spawn a new read loop here
                                let pending = pending.clone();
                                let connected = connected.clone();
                                let on_chat_message = on_chat_message.clone();
                                let on_match_data = on_match_data.clone();
                                let on_match_presence = on_match_presence.clone();
                                let on_matchmaker_matched = on_matchmaker_matched.clone();
                                let on_notification = on_notification.clone();
                                let on_status_presence = on_status_presence.clone();
                                let on_error = on_error.clone();
                                let on_disconnect = on_disconnect.clone();
                                let on_reconnect = on_reconnect.clone();
                                let reconnect_config = reconnect_config.clone();
                                let reconnect_attempts = reconnect_attempts.clone();
                                let stored_token = stored_token.clone();
                                let reconnecting = reconnecting.clone();
                                let sender_ref = sender_ref.clone();
                                let base_url = base_url.clone();

                                // Spawn the new read loop
                                spawn_read_loop(
                                    new_read,
                                    pending,
                                    connected,
                                    on_chat_message,
                                    on_match_data,
                                    on_match_presence,
                                    on_matchmaker_matched,
                                    on_notification,
                                    on_status_presence,
                                    on_error,
                                    on_disconnect,
                                    on_reconnect,
                                    reconnect_config,
                                    reconnect_attempts,
                                    stored_token,
                                    reconnecting,
                                    sender_ref,
                                    base_url,
                                );

                                return;
                            }
                            Err(_) => {
                                // Increase delay with exponential backoff
                                delay_ms = ((delay_ms as f64) * config.backoff_multiplier) as u64;
                                delay_ms = delay_ms.min(config.max_delay_ms);
                            }
                        }
                    }

                    // Mark as no longer reconnecting
                    {
                        let mut r = reconnecting.write().await;
                        *r = false;
                    }
                }
            }

            // Call disconnect handler (only if we couldn't reconnect)
            if let Some(handler) = on_disconnect.read().await.as_ref() {
                handler();
            }
        });

        Ok(())
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Disconnect from the server.
    pub async fn disconnect(&self) {
        let mut sender = self.sender.lock().await;
        if let Some(tx) = sender.take() {
            let _ = tx.send(WsMessage::Close(None)).await;
        }

        let mut connected = self.connected.write().await;
        *connected = false;
    }

    // ========================================================================
    // Event Handlers
    // ========================================================================

    /// Set handler for chat messages.
    pub async fn on_chat_message<F>(&self, handler: F)
    where
        F: Fn(ChatMessage) + Send + Sync + 'static,
    {
        let mut h = self.on_chat_message.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for match data.
    pub async fn on_match_data<F>(&self, handler: F)
    where
        F: Fn(MatchData) + Send + Sync + 'static,
    {
        let mut h = self.on_match_data.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for match presence events.
    pub async fn on_match_presence<F>(&self, handler: F)
    where
        F: Fn(MatchPresenceEvent) + Send + Sync + 'static,
    {
        let mut h = self.on_match_presence.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for matchmaker matched events.
    pub async fn on_matchmaker_matched<F>(&self, handler: F)
    where
        F: Fn(MatchmakerMatched) + Send + Sync + 'static,
    {
        let mut h = self.on_matchmaker_matched.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for notifications.
    pub async fn on_notification<F>(&self, handler: F)
    where
        F: Fn(Notification) + Send + Sync + 'static,
    {
        let mut h = self.on_notification.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for status presence events.
    pub async fn on_status_presence<F>(&self, handler: F)
    where
        F: Fn(StatusPresenceEvent) + Send + Sync + 'static,
    {
        let mut h = self.on_status_presence.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for errors.
    pub async fn on_error<F>(&self, handler: F)
    where
        F: Fn(Error) + Send + Sync + 'static,
    {
        let mut h = self.on_error.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for disconnect.
    pub async fn on_disconnect<F>(&self, handler: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut h = self.on_disconnect.write().await;
        *h = Some(Box::new(handler));
    }

    /// Set handler for reconnection.
    /// The handler receives the number of reconnection attempts that were made.
    pub async fn on_reconnect<F>(&self, handler: F)
    where
        F: Fn(u32) + Send + Sync + 'static,
    {
        let mut h = self.on_reconnect.write().await;
        *h = Some(Box::new(handler));
    }

    /// Check if currently attempting to reconnect.
    pub async fn is_reconnecting(&self) -> bool {
        *self.reconnecting.read().await
    }

    /// Get the current reconnection attempt count.
    pub async fn reconnect_attempt_count(&self) -> u32 {
        *self.reconnect_attempts.read().await
    }

    // ========================================================================
    // Internal
    // ========================================================================

    fn next_cid(&self) -> String {
        self.cid_counter.fetch_add(1, Ordering::Relaxed).to_string()
    }

    async fn send(&self, envelope: Envelope) -> Result<()> {
        let sender = self.sender.lock().await;
        let tx = sender.as_ref().ok_or(Error::NotConnected)?;

        let json = serde_json::to_string(&envelope)?;
        tx.send(WsMessage::Text(json.into())).await
            .map_err(|_| Error::ConnectionClosed)?;

        Ok(())
    }

    async fn send_with_response(&self, message: EnvelopeMessage) -> Result<EnvelopeMessage> {
        let cid = self.next_cid();
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending.lock().await;
            pending.insert(cid.clone(), tx);
        }

        // Send message
        let envelope = Envelope {
            cid: Some(cid.clone()),
            message,
        };
        self.send(envelope).await?;

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::ConnectionClosed),
            Err(_) => {
                // Remove pending request on timeout
                let mut pending = self.pending.lock().await;
                pending.remove(&cid);
                Err(Error::Timeout)
            }
        }
    }

    // ========================================================================
    // Chat
    // ========================================================================

    /// Join a chat channel.
    ///
    /// # Arguments
    /// * `target` - Room name, group ID, or user ID depending on type
    /// * `channel_type` - 1 = Room, 2 = Direct Message, 3 = Group
    /// * `persistence` - Whether messages should be stored
    /// * `hidden` - Whether presence should be hidden
    pub async fn join_chat(
        &self,
        target: &str,
        channel_type: i32,
        persistence: bool,
        hidden: bool,
    ) -> Result<Channel> {
        let response = self.send_with_response(EnvelopeMessage::ChannelJoin(ChannelJoinMessage {
            target: target.to_string(),
            channel_type,
            persistence,
            hidden,
        })).await?;

        match response {
            EnvelopeMessage::Channel(channel) => Ok(channel),
            EnvelopeMessage::Error(err) => Err(Error::server_code(&err.message, err.code)),
            _ => Err(Error::server("Unexpected response")),
        }
    }

    /// Leave a chat channel.
    pub async fn leave_chat(&self, channel_id: &str) -> Result<()> {
        self.send(Envelope {
            cid: None,
            message: EnvelopeMessage::ChannelLeave(ChannelLeaveMessage {
                channel_id: channel_id.to_string(),
            }),
        }).await
    }

    /// Send a chat message.
    pub async fn send_chat_message(&self, channel_id: &str, content: &str) -> Result<ChatMessage> {
        let response = self.send_with_response(EnvelopeMessage::ChannelMessageSend(
            ChannelMessageSendMessage {
                channel_id: channel_id.to_string(),
                content: content.to_string(),
            }
        )).await?;

        match response {
            EnvelopeMessage::ChannelMessage(msg) => Ok(msg),
            EnvelopeMessage::Error(err) => Err(Error::server_code(&err.message, err.code)),
            _ => Err(Error::server("Unexpected response")),
        }
    }

    // ========================================================================
    // Matches
    // ========================================================================

    /// Join a match by ID.
    pub async fn join_match(&self, match_id: &str) -> Result<Match> {
        self.join_match_with_metadata(match_id, None).await
    }

    /// Join a match by ID with metadata.
    pub async fn join_match_with_metadata(
        &self,
        match_id: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<Match> {
        let response = self.send_with_response(EnvelopeMessage::MatchJoin(MatchJoinMessage {
            match_id: Some(match_id.to_string()),
            token: None,
            metadata,
        })).await?;

        match response {
            EnvelopeMessage::Match(m) => Ok(m),
            EnvelopeMessage::Error(err) => Err(Error::server_code(&err.message, err.code)),
            _ => Err(Error::server("Unexpected response")),
        }
    }

    /// Join a match using a matchmaker token.
    pub async fn join_match_token(&self, token: &str) -> Result<Match> {
        let response = self.send_with_response(EnvelopeMessage::MatchJoin(MatchJoinMessage {
            match_id: None,
            token: Some(token.to_string()),
            metadata: None,
        })).await?;

        match response {
            EnvelopeMessage::Match(m) => Ok(m),
            EnvelopeMessage::Error(err) => Err(Error::server_code(&err.message, err.code)),
            _ => Err(Error::server("Unexpected response")),
        }
    }

    /// Leave a match.
    pub async fn leave_match(&self, match_id: &str) -> Result<()> {
        self.send(Envelope {
            cid: None,
            message: EnvelopeMessage::MatchLeave(MatchLeaveMessage {
                match_id: match_id.to_string(),
            }),
        }).await
    }

    /// Send data to match participants.
    pub async fn send_match_data(
        &self,
        match_id: &str,
        op_code: i64,
        data: &[u8],
        presences: Option<Vec<UserPresence>>,
        reliable: bool,
    ) -> Result<()> {
        use base64::Engine;
        let data_b64 = base64::engine::general_purpose::STANDARD.encode(data);

        self.send(Envelope {
            cid: None,
            message: EnvelopeMessage::MatchDataSend(MatchDataSendMessage {
                match_id: match_id.to_string(),
                op_code,
                data: data_b64,
                presences,
                reliable,
            }),
        }).await
    }

    // ========================================================================
    // Matchmaker
    // ========================================================================

    /// Add to matchmaker.
    pub async fn add_matchmaker(
        &self,
        min_count: i32,
        max_count: i32,
        query: Option<&str>,
        string_properties: Option<HashMap<String, String>>,
        numeric_properties: Option<HashMap<String, f64>>,
    ) -> Result<MatchmakerTicket> {
        let response = self.send_with_response(EnvelopeMessage::MatchmakerAdd(MatchmakerAddMessage {
            min_count,
            max_count,
            query: query.map(String::from),
            string_properties,
            numeric_properties,
        })).await?;

        match response {
            EnvelopeMessage::MatchmakerTicket(ticket) => Ok(ticket),
            EnvelopeMessage::Error(err) => Err(Error::server_code(&err.message, err.code)),
            _ => Err(Error::server("Unexpected response")),
        }
    }

    /// Remove from matchmaker.
    pub async fn remove_matchmaker(&self, ticket: &str) -> Result<()> {
        self.send(Envelope {
            cid: None,
            message: EnvelopeMessage::MatchmakerRemove(MatchmakerRemoveMessage {
                ticket: ticket.to_string(),
            }),
        }).await
    }

    // ========================================================================
    // Status
    // ========================================================================

    /// Update status (online presence).
    pub async fn update_status(&self, status: Option<&str>) -> Result<()> {
        self.send(Envelope {
            cid: None,
            message: EnvelopeMessage::StatusUpdate(StatusUpdateMessage {
                status: status.map(String::from),
            }),
        }).await
    }

    /// Follow users to receive status updates.
    pub async fn follow_users(&self, user_ids: Vec<String>) -> Result<()> {
        self.send(Envelope {
            cid: None,
            message: EnvelopeMessage::StatusFollow(StatusFollowMessage { user_ids }),
        }).await
    }

    /// Unfollow users.
    pub async fn unfollow_users(&self, user_ids: Vec<String>) -> Result<()> {
        self.send(Envelope {
            cid: None,
            message: EnvelopeMessage::StatusUnfollow(StatusUnfollowMessage { user_ids }),
        }).await
    }

    // ========================================================================
    // RPC
    // ========================================================================

    /// Call an RPC function over WebSocket.
    pub async fn rpc(&self, id: &str, payload: Option<&str>) -> Result<String> {
        let response = self.send_with_response(EnvelopeMessage::Rpc(RpcMessage {
            id: id.to_string(),
            payload: payload.map(String::from),
        })).await?;

        match response {
            EnvelopeMessage::Rpc(msg) => Ok(msg.payload.unwrap_or_default()),
            EnvelopeMessage::Error(err) => Err(Error::server_code(&err.message, err.code)),
            _ => Err(Error::server("Unexpected response")),
        }
    }
}
