//! Minimal MQTT 3.1.1 client implementation for Guitar Hero III telemetry.

use anyhow::Context;
use std::io::Read;
use std::io::Write;
use std::mem::size_of;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::time::Duration;
use std::time::Instant;
use zerocopy::FromBytes;

/// Errors encountered during MQTT operations.
#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    /// Attempted an operation while not connected.
    #[error("Not connected to MQTT broker")]
    NotConnected,

    /// Failed to resolve the broker hostname or port.
    #[error("Failed to resolve broker address '{host}:{port}': {source}")]
    ResolveAddress {
        /// Configured broker hostname.
        host: String,
        /// Configured broker port.
        port: u16,
        /// Underlying socket resolution error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to establish a TCP connection within timeout.
    #[error("Could not connect to broker at {host}:{port}")]
    ConnectionFailed {
        /// Configured broker hostname.
        host: String,
        /// Configured broker port.
        port: u16,
    },

    /// Received an invalid or malformed CONNACK packet.
    #[error(
        "Invalid CONNACK packet: expected packet_type=0x20 and remaining_length=2, got packet_type={packet_type:#04x}, len={remaining_length}"
    )]
    InvalidConnack {
        /// Received packet type byte.
        packet_type: proto::PacketType,
        /// Received remaining length byte.
        remaining_length: proto::RemainingLength,
    },

    /// Broker refused the connection.
    #[error("Broker rejected connection with code {code} ({reason})")]
    ConnectionRefused {
        /// MQTT return code received from broker.
        code: proto::ConnectReturnCode,
        /// Human-readable explanation of return code.
        reason: &'static str,
    },

    /// An underlying I/O error occurred.
    #[error("MQTT I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl MqttError {
    /// Returns true if this error indicates an unrecoverable rejection by the
    /// broker (such as protocol mismatch, bad client identifier, bad
    /// credentials, or unauthorized).
    pub fn is_fatal(&self) -> bool {
        match self {
            Self::ConnectionRefused { code, .. } => code.is_fatal(),
            _ => false,
        }
    }
}

/// A lightweight, non-blocking MQTT 3.1.1 client.
pub struct MqttClient {
    stream: Option<TcpStream>,
    host: String,
    port: u16,
    client_id: String,
    username: Option<String>,
    password: Option<String>,
    will_topic: String,
    will_payload: String,
    keep_alive_secs: u16,
    last_ping: Instant,
}

impl MqttClient {
    /// Creates a new `MqttClient` instance with connection parameters.
    pub fn new(
        host: String,
        port: u16,
        client_id: String,
        username: Option<String>,
        password: Option<String>,
        will_topic: String,
        will_payload: String,
    ) -> Self {
        Self {
            stream: None,
            host,
            port,
            client_id,
            username,
            password,
            will_topic,
            will_payload,
            keep_alive_secs: 30,
            last_ping: Instant::now(),
        }
    }

    /// Checks whether an active TCP stream is currently open.
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Establishes a TCP connection to the broker and performs the MQTT 3.1.1
    /// CONNECT handshake.
    pub fn connect(&mut self) -> Result<(), MqttError> {
        let addr = format!("{}:{}", self.host, self.port);
        let socket_addrs = addr
            .to_socket_addrs()
            .map_err(|e| MqttError::ResolveAddress {
                host: self.host.clone(),
                port: self.port,
                source: e,
            })?;

        let mut connected_stream = None;
        for saddr in socket_addrs {
            if let Ok(s) = TcpStream::connect_timeout(&saddr, Duration::from_secs(3)) {
                let _ = s.set_nodelay(true);
                let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
                let _ = s.set_write_timeout(Some(Duration::from_secs(3)));
                connected_stream = Some(s);
                break;
            }
        }

        let mut stream = connected_stream.ok_or_else(|| MqttError::ConnectionFailed {
            host: self.host.clone(),
            port: self.port,
        })?;

        // Build MQTT 3.1.1 CONNECT packet
        let mut variable_header = Vec::new();
        proto::encode_str(proto::PROTOCOL_NAME, &mut variable_header);
        variable_header.push(proto::ProtocolLevel::MQTT_3_1_1.0);

        // Connect Flags
        let mut flags = proto::ConnectFlags::CLEAN_SESSION;
        if !self.will_topic.is_empty() {
            flags |= proto::ConnectFlags::WILL | proto::ConnectFlags::WILL_RETAIN;
        }
        if self.username.is_some() {
            flags |= proto::ConnectFlags::USERNAME;
        }
        if self.password.is_some() {
            flags |= proto::ConnectFlags::PASSWORD;
        }
        variable_header.push(flags.bits());

        // Keep Alive
        variable_header.extend_from_slice(&self.keep_alive_secs.to_be_bytes());

        // Payload
        let mut payload = Vec::new();
        proto::encode_str(&self.client_id, &mut payload);

        if !self.will_topic.is_empty() {
            proto::encode_str(&self.will_topic, &mut payload);
            proto::encode_str(&self.will_payload, &mut payload);
        }

        if let Some(ref u) = self.username {
            proto::encode_str(u, &mut payload);
        }
        if let Some(ref p) = self.password {
            proto::encode_str(p, &mut payload);
        }

        let remaining_length = variable_header.len() + payload.len();
        let mut packet = Vec::with_capacity(remaining_length + 5);
        packet.push(proto::PacketType::CONNECT.0);
        proto::encode_varlen(remaining_length, &mut packet);
        packet.extend_from_slice(&variable_header);
        packet.extend_from_slice(&payload);

        stream.write_all(&packet)?;

        // Read CONNACK response
        let mut connack_buf = [0u8; size_of::<proto::ConnackPacket>()];
        stream.read_exact(&mut connack_buf)?;

        let connack = proto::ConnackPacket::read_from_bytes(&connack_buf).map_err(|_| {
            MqttError::InvalidConnack {
                packet_type: proto::PacketType(connack_buf[0]),
                remaining_length: proto::RemainingLength(connack_buf[1]),
            }
        })?;

        if connack.packet_type != proto::PacketType::CONNACK
            || connack.remaining_length != proto::RemainingLength::CONNACK
        {
            return Err(MqttError::InvalidConnack {
                packet_type: connack.packet_type,
                remaining_length: connack.remaining_length,
            });
        }
        if connack.return_code != proto::ConnectReturnCode::ACCEPTED {
            return Err(MqttError::ConnectionRefused {
                code: connack.return_code,
                reason: proto::connack_reason(connack.return_code),
            });
        }

        self.stream = Some(stream);
        self.last_ping = Instant::now();
        Ok(())
    }

    /// Publishes a QoS 0 payload to `topic`.
    pub fn publish(&mut self, topic: &str, payload: &[u8], retain: bool) -> anyhow::Result<()> {
        let stream = self.stream.as_mut().ok_or(MqttError::NotConnected)?;

        let mut variable_header = Vec::new();
        proto::encode_str(topic, &mut variable_header);

        let remaining_length = variable_header.len() + payload.len();
        let mut packet = Vec::with_capacity(remaining_length + 5);

        // Fixed header: PUBLISH, QoS 0, retain flag
        let mut flags = proto::PublishFlags::empty();
        if retain {
            flags |= proto::PublishFlags::RETAIN;
        }
        packet.push(proto::PacketType::PUBLISH.0 | flags.bits());
        proto::encode_varlen(remaining_length, &mut packet);
        packet.extend_from_slice(&variable_header);
        packet.extend_from_slice(payload);

        if let Err(e) = stream.write_all(&packet) {
            self.stream = None;
            return Err(e).with_context(|| format!("Publish to topic '{}' failed", topic));
        }

        Ok(())
    }

    /// Sends a PINGREQ packet if half the keep-alive interval has elapsed, and
    /// drains responses.
    pub fn ping_if_needed(&mut self) -> anyhow::Result<()> {
        if self.stream.is_none() {
            return Ok(());
        }

        if self.last_ping.elapsed() >= Duration::from_secs((self.keep_alive_secs / 2).max(5) as u64)
        {
            let stream = self.stream.as_mut().unwrap();
            if let Err(e) = stream.write_all(&proto::PINGREQ) {
                self.stream = None;
                return Err(e).context("Failed to write PINGREQ packet to MQTT broker");
            }
            self.last_ping = Instant::now();
        }

        // Drain any incoming pingresp or messages non-blockingly
        if let Some(stream) = self.stream.as_mut() {
            let mut buf = [0u8; 128];
            match stream.read(&mut buf) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    self.stream = None;
                    return Err(e)
                        .context("Failed to read from MQTT broker stream during keepalive");
                }
            }
        }

        Ok(())
    }

    /// Sends a clean DISCONNECT packet and closes the underlying socket.
    pub fn disconnect(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.write_all(&proto::DISCONNECT);
        }
    }
}

/// MQTT 3.1.1 wire protocol types, packet encoding helpers, and constants.
mod proto {
    use bitflags::bitflags;
    use zerocopy::FromBytes;
    use zerocopy::Immutable;
    use zerocopy::IntoBytes;
    use zerocopy::KnownLayout;

    /// MQTT protocol name string.
    pub const PROTOCOL_NAME: &str = "MQTT";

    /// MQTT 3.1.1 protocol level / version.
    #[repr(transparent)]
    #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ProtocolLevel(pub u8);

    impl ProtocolLevel {
        /// MQTT 3.1.1 protocol version level 4.
        pub const MQTT_3_1_1: Self = Self(4);
    }

    /// MQTT 3.1.1 fixed header packet type byte.
    #[repr(transparent)]
    #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq, Eq)]
    pub struct PacketType(pub u8);

    impl PacketType {
        /// CONNECT packet type (0x10).
        pub const CONNECT: Self = Self(0x10);
        /// CONNACK packet type (0x20).
        pub const CONNACK: Self = Self(0x20);
        /// PUBLISH packet type with QoS 0 and no flags (0x30).
        pub const PUBLISH: Self = Self(0x30);
        /// PINGREQ packet type (0xC0).
        pub const PINGREQ: Self = Self(0xC0);
        /// PINGRESP packet type (0xD0).
        #[expect(dead_code)]
        pub const PINGRESP: Self = Self(0xD0);
        /// DISCONNECT packet type (0xE0).
        pub const DISCONNECT: Self = Self(0xE0);
    }

    impl std::fmt::LowerHex for PacketType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::LowerHex::fmt(&self.0, f)
        }
    }

    /// Remaining length byte for fixed-size packets.
    #[repr(transparent)]
    #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq, Eq)]
    pub struct RemainingLength(pub u8);

    impl RemainingLength {
        /// Remaining length of zero (PINGREQ, DISCONNECT).
        pub const ZERO: Self = Self(0);
        /// Expected remaining length for CONNACK (2).
        pub const CONNACK: Self = Self(2);
    }

    impl std::fmt::Display for RemainingLength {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    /// MQTT 3.1.1 CONNACK return code.
    #[repr(transparent)]
    #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ConnectReturnCode(pub u8);

    impl ConnectReturnCode {
        /// Connection Accepted (0x00).
        pub const ACCEPTED: Self = Self(0x00);
        /// Connection Refused: unacceptable protocol version (1).
        pub const UNACCEPTABLE_PROTOCOL_VERSION: Self = Self(1);
        /// Connection Refused: identifier rejected (2).
        pub const IDENTIFIER_REJECTED: Self = Self(2);
        /// Connection Refused: server unavailable (3).
        pub const SERVER_UNAVAILABLE: Self = Self(3);
        /// Connection Refused: bad user name or password (4).
        pub const BAD_USERNAME_OR_PASSWORD: Self = Self(4);
        /// Connection Refused: not authorized (5).
        pub const NOT_AUTHORIZED: Self = Self(5);

        /// Returns true if this return code indicates a permanent/unrecoverable
        /// error.
        pub fn is_fatal(self) -> bool {
            matches!(
                self,
                Self::UNACCEPTABLE_PROTOCOL_VERSION
                    | Self::IDENTIFIER_REJECTED
                    | Self::BAD_USERNAME_OR_PASSWORD
                    | Self::NOT_AUTHORIZED
            )
        }

        /// Returns a human-readable explanation of this return code.
        pub fn reason(self) -> &'static str {
            match self {
                Self::ACCEPTED => "connection accepted",
                Self::UNACCEPTABLE_PROTOCOL_VERSION => "unacceptable protocol version",
                Self::IDENTIFIER_REJECTED => "identifier rejected",
                Self::SERVER_UNAVAILABLE => "server unavailable",
                Self::BAD_USERNAME_OR_PASSWORD => "bad user name or password",
                Self::NOT_AUTHORIZED => "not authorized",
                _ => "unknown return code",
            }
        }
    }

    impl std::fmt::Display for ConnectReturnCode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    bitflags! {
        /// MQTT 3.1.1 CONNECT packet flags.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ConnectFlags: u8 {
            /// Clean Session flag (bit 1).
            const CLEAN_SESSION = 0x02;
            /// Will Flag (bit 2).
            const WILL = 0x04;
            /// Will QoS 1 (bit 3).
            const WILL_QOS_1 = 0x08;
            /// Will QoS 2 (bit 4).
            const WILL_QOS_2 = 0x10;
            /// Will Retain flag (bit 5).
            const WILL_RETAIN = 0x20;
            /// Password flag (bit 6).
            const PASSWORD = 0x40;
            /// User Name flag (bit 7).
            const USERNAME = 0x80;
        }
    }

    bitflags! {
        /// MQTT 3.1.1 CONNACK packet flag bits.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ConnackFlagsBits: u8 {
            /// Session Present flag (bit 0).
            const SESSION_PRESENT = 0x01;
        }
    }

    /// MQTT 3.1.1 CONNACK packet flags byte.
    #[repr(transparent)]
    #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ConnackFlags(pub u8);

    #[expect(dead_code)]
    impl ConnackFlags {
        /// Session Present flag (bit 0).
        pub const SESSION_PRESENT: Self = Self(0x01);

        /// Returns the decoded flag bits.
        pub fn flags(self) -> ConnackFlagsBits {
            ConnackFlagsBits::from_bits_truncate(self.0)
        }

        /// Returns true if the session present flag is set.
        pub fn session_present(self) -> bool {
            (self.0 & Self::SESSION_PRESENT.0) != 0
        }
    }

    bitflags! {
        /// MQTT 3.1.1 PUBLISH fixed header flags (bits 0-3).
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct PublishFlags: u8 {
            /// Retain flag (bit 0).
            const RETAIN = 0x01;
            /// QoS 1 flag (bit 1).
            const QOS_1 = 0x02;
            /// QoS 2 flag (bit 2).
            const QOS_2 = 0x04;
            /// Duplicate delivery flag (bit 3).
            const DUP = 0x08;
        }
    }

    /// Fixed 2-byte PINGREQ packet (`[0xC0, 0x00]`).
    pub const PINGREQ: [u8; 2] = [PacketType::PINGREQ.0, RemainingLength::ZERO.0];
    /// Fixed 2-byte DISCONNECT packet (`[0xE0, 0x00]`).
    pub const DISCONNECT: [u8; 2] = [PacketType::DISCONNECT.0, RemainingLength::ZERO.0];

    /// MQTT 3.1.1 CONNACK packet fixed and variable header.
    #[repr(C)]
    #[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
    pub struct ConnackPacket {
        /// Fixed header packet type (0x20).
        pub packet_type: PacketType,
        /// Fixed header remaining length (2).
        pub remaining_length: RemainingLength,
        /// Connect acknowledge flags (session present bit 0).
        pub session_present: ConnackFlags,
        /// Connect return code (0x00 for connection accepted).
        pub return_code: ConnectReturnCode,
    }

    /// Maps an MQTT 3.1.1 CONNACK return code to a human-readable explanation.
    pub fn connack_reason(code: ConnectReturnCode) -> &'static str {
        code.reason()
    }

    const VARLEN_VALUE_MASK: usize = 0x7F;
    const VARLEN_CONTINUATION_BIT: u8 = 0x80;
    const VARLEN_SHIFT: usize = 7;

    /// Encodes a variable-length integer into MQTT byte format.
    pub fn encode_varlen(mut len: usize, out: &mut Vec<u8>) {
        loop {
            let mut byte = (len & VARLEN_VALUE_MASK) as u8;
            len >>= VARLEN_SHIFT;
            if len > 0 {
                byte |= VARLEN_CONTINUATION_BIT;
            }
            out.push(byte);
            if len == 0 {
                break;
            }
        }
    }

    /// Encodes a 2-byte big-endian length-prefixed UTF-8 string into MQTT byte
    /// format.
    pub fn encode_str(s: &str, out: &mut Vec<u8>) {
        let len = s.len() as u16;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(s.as_bytes());
    }
}
