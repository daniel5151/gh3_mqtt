//! Minimal MQTT 3.1.1 client implementation for Guitar Hero III telemetry.

use anyhow::Context;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::time::Duration;
use std::time::Instant;
use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

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
        packet_type: u8,
        /// Received remaining length byte.
        remaining_length: u8,
    },

    /// Broker refused the connection.
    #[error("Broker rejected connection with code {code} ({reason})")]
    ConnectionRefused {
        /// MQTT return code received from broker.
        code: u8,
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
            Self::ConnectionRefused { code, .. } => matches!(*code, 1 | 2 | 4 | 5),
            _ => false,
        }
    }
}

/// MQTT 3.1.1 CONNACK packet fixed and variable header.
#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
struct ConnackPacket {
    packet_type: u8,
    remaining_length: u8,
    session_present: u8,
    return_code: u8,
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
        // Protocol Name: "MQTT"
        encode_str("MQTT", &mut variable_header);
        // Protocol Level: 4 (MQTT 3.1.1)
        variable_header.push(4);

        // Connect Flags
        let mut flags: u8 = 0x02; // Clean Session
        if !self.will_topic.is_empty() {
            flags |= 0x04; // Will Flag
            flags |= 0x20; // Will Retain
        }
        if self.username.is_some() {
            flags |= 0x80;
        }
        if self.password.is_some() {
            flags |= 0x40;
        }
        variable_header.push(flags);

        // Keep Alive
        variable_header.extend_from_slice(&self.keep_alive_secs.to_be_bytes());

        // Payload
        let mut payload = Vec::new();
        encode_str(&self.client_id, &mut payload);

        if !self.will_topic.is_empty() {
            encode_str(&self.will_topic, &mut payload);
            encode_str(&self.will_payload, &mut payload);
        }

        if let Some(ref u) = self.username {
            encode_str(u, &mut payload);
        }
        if let Some(ref p) = self.password {
            encode_str(p, &mut payload);
        }

        let remaining_length = variable_header.len() + payload.len();
        let mut packet = Vec::with_capacity(remaining_length + 5);
        packet.push(0x10); // CONNECT
        encode_varlen(remaining_length, &mut packet);
        packet.extend_from_slice(&variable_header);
        packet.extend_from_slice(&payload);

        stream.write_all(&packet)?;

        // Read CONNACK response
        let mut connack_buf = [0u8; 4];
        stream.read_exact(&mut connack_buf)?;

        let connack = ConnackPacket::read_from_bytes(&connack_buf).map_err(|_| {
            MqttError::InvalidConnack {
                packet_type: connack_buf[0],
                remaining_length: connack_buf[1],
            }
        })?;

        if connack.packet_type != 0x20 || connack.remaining_length != 0x02 {
            return Err(MqttError::InvalidConnack {
                packet_type: connack.packet_type,
                remaining_length: connack.remaining_length,
            });
        }
        if connack.return_code != 0x00 {
            return Err(MqttError::ConnectionRefused {
                code: connack.return_code,
                reason: {
                    let code = connack.return_code;
                    match code {
                        1 => "unacceptable protocol version",
                        2 => "identifier rejected",
                        3 => "server unavailable",
                        4 => "bad user name or password",
                        5 => "not authorized",
                        _ => "unknown return code",
                    }
                },
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
        encode_str(topic, &mut variable_header);

        let remaining_length = variable_header.len() + payload.len();
        let mut packet = Vec::with_capacity(remaining_length + 5);

        // Fixed header: 0x30 = PUBLISH, QoS 0, retain flag
        let mut header: u8 = 0x30;
        if retain {
            header |= 0x01;
        }
        packet.push(header);
        encode_varlen(remaining_length, &mut packet);
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
            let pingreq = [0xC0, 0x00];
            if let Err(e) = stream.write_all(&pingreq) {
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
            let disconnect_packet = [0xE0, 0x00];
            let _ = stream.write_all(&disconnect_packet);
        }
    }
}

fn encode_varlen(mut len: usize, out: &mut Vec<u8>) {
    loop {
        let mut byte = (len & 0x7F) as u8;
        len >>= 7;
        if len > 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if len == 0 {
            break;
        }
    }
}

fn encode_str(s: &str, out: &mut Vec<u8>) {
    let len = s.len() as u16;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}
