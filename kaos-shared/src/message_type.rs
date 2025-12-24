//! RUDP message type discriminator.

/// RUDP message types for the transport layer.
///
/// These identify the purpose of each RUDP packet:
/// - `Data`: Application data payload
/// - `Ack`: Acknowledgement of received data
/// - `Nak`: Negative acknowledgement (request retransmit)
/// - `Ping`/`Pong`: Keep-alive heartbeat
/// - `Handshake`: Connection establishment
/// - `Disconnect`: Graceful connection close
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// Application data payload
    Data = 0,
    /// Acknowledgement of received sequence
    Ack = 1,
    /// Negative acknowledgement (request retransmit)
    Nak = 2,
    /// Keep-alive ping request
    Ping = 3,
    /// Keep-alive pong response
    Pong = 4,
    /// Connection handshake
    Handshake = 5,
    /// Graceful disconnect
    Disconnect = 6,
}

impl MessageType {
    /// Convert from raw byte value.
    ///
    /// Returns `None` for invalid values.
    #[inline]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Data),
            1 => Some(Self::Ack),
            2 => Some(Self::Nak),
            3 => Some(Self::Ping),
            4 => Some(Self::Pong),
            5 => Some(Self::Handshake),
            6 => Some(Self::Disconnect),
            _ => None,
        }
    }

    /// Convert from raw byte value, defaulting to `Data` for invalid values.
    ///
    /// Use this for lossy conversion when you want to handle unknown types gracefully.
    #[inline]
    pub fn from_u8_lossy(value: u8) -> Self {
        Self::from_u8(value).unwrap_or(Self::Data)
    }
}

// Note: We only implement TryFrom (strict conversion).
// Use `MessageType::from_u8_lossy()` for lossy conversion that defaults to Data.
impl TryFrom<u8> for MessageType {
    type Error = ();

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_values() {
        assert_eq!(MessageType::Data as u8, 0);
        assert_eq!(MessageType::Ack as u8, 1);
        assert_eq!(MessageType::Nak as u8, 2);
        assert_eq!(MessageType::Ping as u8, 3);
        assert_eq!(MessageType::Pong as u8, 4);
        assert_eq!(MessageType::Handshake as u8, 5);
        assert_eq!(MessageType::Disconnect as u8, 6);
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(MessageType::from_u8(0), Some(MessageType::Data));
        assert_eq!(MessageType::from_u8(5), Some(MessageType::Handshake));
        assert_eq!(MessageType::from_u8(7), None);
        assert_eq!(MessageType::from_u8(255), None);
    }

    #[test]
    fn test_from_u8_lossy() {
        assert_eq!(MessageType::from_u8_lossy(0), MessageType::Data);
        assert_eq!(MessageType::from_u8_lossy(5), MessageType::Handshake);
        assert_eq!(MessageType::from_u8_lossy(7), MessageType::Data); // Invalid defaults to Data
        assert_eq!(MessageType::from_u8_lossy(255), MessageType::Data);
    }

    #[test]
    fn test_try_from() {
        assert_eq!(MessageType::try_from(1u8), Ok(MessageType::Ack));
        assert_eq!(MessageType::try_from(100u8), Err(()));
    }
}
