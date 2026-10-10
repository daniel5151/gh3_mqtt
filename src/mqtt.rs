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

#[repr(C)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, Debug, PartialEq)]
pub struct ConnackPacket {
    pub packet_type: u8,
    pub remaining_length: u8,
    pub session_present: u8,
    pub return_code: u8,
}

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

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn connect(&mut self) -> Result<(), String> {
        let addr = format!("{}:{}", self.host, self.port);
        let socket_addrs = addr
            .to_socket_addrs()
            .map_err(|e| format!("Failed to resolve {}: {}", addr, e))?;

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

        let mut stream = connected_stream
            .ok_or_else(|| format!("Could not connect to {}:{}", self.host, self.port))?;

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

        stream
            .write_all(&packet)
            .map_err(|e| format!("Failed to send CONNECT: {}", e))?;

        // Read CONNACK response
        let mut connack_buf = [0u8; 4];
        stream
            .read_exact(&mut connack_buf)
            .map_err(|e| format!("Failed to read CONNACK: {}", e))?;

        let connack = ConnackPacket::read_from_bytes(&connack_buf)
            .map_err(|e| format!("Failed to parse CONNACK: {}", e))?;

        if connack.packet_type != 0x20 || connack.remaining_length != 0x02 {
            return Err(format!("Invalid CONNACK response: {:?}", connack));
        }
        if connack.return_code != 0x00 {
            return Err(format!(
                "Broker rejected connection with return code: {}",
                connack.return_code
            ));
        }

        self.stream = Some(stream);
        self.last_ping = Instant::now();
        Ok(())
    }

    pub fn publish(&mut self, topic: &str, payload: &[u8], retain: bool) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

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
            return Err(format!("Publish failed: {}", e));
        }

        Ok(())
    }

    pub fn ping_if_needed(&mut self) -> Result<(), String> {
        if self.stream.is_none() {
            return Ok(());
        }

        if self.last_ping.elapsed() >= Duration::from_secs((self.keep_alive_secs / 2).max(5) as u64)
        {
            let stream = self.stream.as_mut().unwrap();
            let pingreq = [0xC0, 0x00];
            if let Err(e) = stream.write_all(&pingreq) {
                self.stream = None;
                return Err(format!("Ping failed: {}", e));
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
                Err(_) => {
                    self.stream = None;
                }
            }
        }

        Ok(())
    }

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
