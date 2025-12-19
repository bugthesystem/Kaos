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

    // Event handlers
    on_chat_message: Arc<RwLock<Option<ChatMessageHandler>>>,
    on_match_data: Arc<RwLock<Option<MatchDataHandler>>>,
    on_match_presence: Arc<RwLock<Option<MatchPresenceHandler>>>,
    on_matchmaker_matched: Arc<RwLock<Option<MatchmakerMatchedHandler>>>,
    on_notification: Arc<RwLock<Option<NotificationHandler>>>,
    on_status_presence: Arc<RwLock<Option<StatusPresenceHandler>>>,
    on_error: Arc<RwLock<Option<ErrorHandler>>>,
    on_disconnect: Arc<RwLock<Option<DisconnectHandler>>>,
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
            on_chat_message: Arc::new(RwLock::new(None)),
            on_match_data: Arc::new(RwLock::new(None)),
            on_match_presence: Arc::new(RwLock::new(None)),
            on_matchmaker_matched: Arc::new(RwLock::new(None)),
            on_notification: Arc::new(RwLock::new(None)),
            on_status_presence: Arc::new(RwLock::new(None)),
            on_error: Arc::new(RwLock::new(None)),
            on_disconnect: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to the server.
    pub async fn connect(&self, session: &Session) -> Result<()> {
        let url = format!("{}/ws?token={}", self.base_url, session.token);

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

        // Spawn read task
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

            // Call disconnect handler
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
