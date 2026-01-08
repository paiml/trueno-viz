//! Network connection tracking - Little Snitch style.
//!
//! On Linux: Parses /proc/net/tcp and /proc/net/udp for active connections.
//! On macOS: Uses netstat -anp tcp/udp for connections.

use std::collections::HashMap;
use std::net::Ipv4Addr;

#[cfg(target_os = "linux")]
use std::fs;

#[cfg(target_os = "macos")]
use std::process::Command;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnState {
    Established,
    Listen,
    TimeWait,
    CloseWait,
    SynSent,
    SynRecv,
    FinWait1,
    FinWait2,
    Closing,
    LastAck,
    Close,
    Unknown,
}

impl ConnState {
    fn from_hex(hex: &str) -> Self {
        match hex {
            "01" => Self::Established,
            "02" => Self::SynSent,
            "03" => Self::SynRecv,
            "04" => Self::FinWait1,
            "05" => Self::FinWait2,
            "06" => Self::TimeWait,
            "07" => Self::Close,
            "08" => Self::CloseWait,
            "09" => Self::LastAck,
            "0A" => Self::Listen,
            "0B" => Self::Closing,
            _ => Self::Unknown,
        }
    }

    pub fn as_char(&self) -> char {
        match self {
            Self::Established => 'E',
            Self::Listen => 'L',
            Self::TimeWait => 'T',
            Self::CloseWait => 'C',
            Self::SynSent => 'S',
            Self::SynRecv => 'R',
            Self::FinWait1 | Self::FinWait2 => 'F',
            Self::Closing | Self::LastAck | Self::Close => 'X',
            Self::Unknown => '?',
        }
    }
}

/// Network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

/// A single network connection
#[derive(Debug, Clone)]
pub struct Connection {
    pub protocol: Protocol,
    pub local_ip: Ipv4Addr,
    pub local_port: u16,
    pub remote_ip: Ipv4Addr,
    pub remote_port: u16,
    pub state: ConnState,
    pub inode: u64,
    pub uid: u32,
    pub tx_queue: u64,
    pub rx_queue: u64,
}

impl Connection {
    /// Check if this is an outbound connection
    pub fn is_outbound(&self) -> bool {
        self.state == ConnState::Established && self.local_port > 1024
    }

    /// Check if this is listening
    pub fn is_listening(&self) -> bool {
        self.state == ConnState::Listen
    }

    /// Get remote address as string
    pub fn remote_addr(&self) -> String {
        if self.remote_ip.is_unspecified() {
            "*".to_string()
        } else {
            format!("{}:{}", self.remote_ip, self.remote_port)
        }
    }

    /// Get local address as string
    pub fn local_addr(&self) -> String {
        format!("{}:{}", self.local_ip, self.local_port)
    }
}

/// Connection analyzer
pub struct ConnectionAnalyzer {
    connections: Vec<Connection>,
    inode_to_pid: HashMap<u64, (u32, String)>, // inode -> (pid, name)
}

impl ConnectionAnalyzer {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            inode_to_pid: HashMap::new(),
        }
    }

    /// Collect connection data
    pub fn collect(&mut self) {
        self.connections.clear();

        #[cfg(target_os = "linux")]
        {
            // Parse TCP connections
            if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
                self.parse_proc_net(&content, Protocol::Tcp);
            }

            // Parse UDP connections
            if let Ok(content) = fs::read_to_string("/proc/net/udp") {
                self.parse_proc_net(&content, Protocol::Udp);
            }

            // Build inode -> pid mapping (expensive, do periodically)
            self.build_inode_map();
        }

        #[cfg(target_os = "macos")]
        {
            self.collect_macos();
        }
    }

    /// Collect connections on macOS using netstat
    #[cfg(target_os = "macos")]
    fn collect_macos(&mut self) {
        // netstat -anp tcp gives us TCP connections
        if let Ok(output) = Command::new("netstat")
            .args(["-anp", "tcp"])
            .output()
        {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout);
                self.parse_netstat_macos(&content, Protocol::Tcp);
            }
        }

        // netstat -anp udp gives us UDP connections
        if let Ok(output) = Command::new("netstat")
            .args(["-anp", "udp"])
            .output()
        {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout);
                self.parse_netstat_macos(&content, Protocol::Udp);
            }
        }
    }

    /// Parse macOS netstat output
    #[cfg(target_os = "macos")]
    fn parse_netstat_macos(&mut self, content: &str, protocol: Protocol) {
        // macOS netstat format:
        // Proto Recv-Q Send-Q  Local Address          Foreign Address        (state)
        // tcp4       0      0  192.168.1.100.443      10.0.0.1.52134         ESTABLISHED
        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }

            // Parse local address (ip.port format)
            let (local_ip, local_port) = match Self::parse_macos_addr(parts[3]) {
                Some(a) => a,
                None => continue,
            };

            // Parse remote address
            let (remote_ip, remote_port) = match Self::parse_macos_addr(parts[4]) {
                Some(a) => a,
                None => continue,
            };

            // Parse state (TCP only)
            let state = if parts.len() > 5 {
                Self::parse_macos_state(parts[5])
            } else {
                ConnState::Unknown
            };

            self.connections.push(Connection {
                protocol,
                local_ip,
                local_port,
                remote_ip,
                remote_port,
                state,
                inode: 0,
                uid: 0,
                tx_queue: 0,
                rx_queue: 0,
            });
        }
    }

    /// Parse macOS address format: ip.port or *.port
    #[cfg(target_os = "macos")]
    fn parse_macos_addr(addr: &str) -> Option<(Ipv4Addr, u16)> {
        // Find the last dot (port separator)
        let last_dot = addr.rfind('.')?;
        let ip_str = &addr[..last_dot];
        let port_str = &addr[last_dot + 1..];

        let port = port_str.parse().ok()?;

        let ip = if ip_str == "*" {
            Ipv4Addr::new(0, 0, 0, 0)
        } else {
            ip_str.parse().unwrap_or(Ipv4Addr::new(0, 0, 0, 0))
        };

        Some((ip, port))
    }

    /// Parse macOS connection state
    #[cfg(target_os = "macos")]
    fn parse_macos_state(state: &str) -> ConnState {
        match state {
            "ESTABLISHED" => ConnState::Established,
            "LISTEN" => ConnState::Listen,
            "TIME_WAIT" => ConnState::TimeWait,
            "CLOSE_WAIT" => ConnState::CloseWait,
            "SYN_SENT" => ConnState::SynSent,
            "SYN_RECEIVED" => ConnState::SynRecv,
            "FIN_WAIT_1" => ConnState::FinWait1,
            "FIN_WAIT_2" => ConnState::FinWait2,
            "CLOSING" => ConnState::Closing,
            "LAST_ACK" => ConnState::LastAck,
            "CLOSED" => ConnState::Close,
            _ => ConnState::Unknown,
        }
    }

    #[cfg(target_os = "linux")]
    fn parse_proc_net(&mut self, content: &str, protocol: Protocol) {
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }

            // Parse local address
            let (local_ip, local_port) = match Self::parse_addr(parts[1]) {
                Some(a) => a,
                None => continue,
            };

            // Parse remote address
            let (remote_ip, remote_port) = match Self::parse_addr(parts[2]) {
                Some(a) => a,
                None => continue,
            };

            let state = ConnState::from_hex(parts[3]);

            // Parse queues (tx:rx)
            let queues: Vec<&str> = parts[4].split(':').collect();
            let tx_queue = u64::from_str_radix(queues.first().unwrap_or(&"0"), 16).unwrap_or(0);
            let rx_queue = u64::from_str_radix(queues.get(1).unwrap_or(&"0"), 16).unwrap_or(0);

            let uid = parts[7].parse().unwrap_or(0);
            let inode = parts[9].parse().unwrap_or(0);

            self.connections.push(Connection {
                protocol,
                local_ip,
                local_port,
                remote_ip,
                remote_port,
                state,
                inode,
                uid,
                tx_queue,
                rx_queue,
            });
        }
    }

    #[cfg(target_os = "linux")]
    fn parse_addr(hex_addr: &str) -> Option<(Ipv4Addr, u16)> {
        let parts: Vec<&str> = hex_addr.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        // IP is in little-endian hex
        let ip_hex = parts[0];
        if ip_hex.len() != 8 {
            return None;
        }

        let ip_num = u32::from_str_radix(ip_hex, 16).ok()?;
        let ip = Ipv4Addr::from(ip_num.swap_bytes());

        let port = u16::from_str_radix(parts[1], 16).ok()?;

        Some((ip, port))
    }

    #[cfg(target_os = "linux")]
    fn build_inode_map(&mut self) {
        self.inode_to_pid.clear();

        // Read /proc/*/fd/* to map inodes to PIDs
        let proc = match fs::read_dir("/proc") {
            Ok(p) => p,
            Err(_) => return,
        };

        for entry in proc.flatten() {
            let pid: u32 = match entry.file_name().to_str().and_then(|s| s.parse().ok()) {
                Some(p) => p,
                None => continue,
            };

            // Get process name
            let comm_path = format!("/proc/{}/comm", pid);
            let name = fs::read_to_string(&comm_path)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            // Read fd directory
            let fd_path = format!("/proc/{}/fd", pid);
            let fds = match fs::read_dir(&fd_path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            for fd in fds.flatten() {
                if let Ok(link) = fs::read_link(fd.path()) {
                    let link_str = link.to_string_lossy();
                    if link_str.starts_with("socket:[") {
                        if let Some(inode_str) = link_str.strip_prefix("socket:[").and_then(|s| s.strip_suffix(']')) {
                            if let Ok(inode) = inode_str.parse::<u64>() {
                                self.inode_to_pid.insert(inode, (pid, name.clone()));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get all connections
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    /// Get active (established) connections
    pub fn active_connections(&self) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.state == ConnState::Established)
            .collect()
    }

    /// Get listening sockets
    pub fn listening(&self) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.state == ConnState::Listen)
            .collect()
    }

    /// Get process info for a connection
    pub fn process_for_connection(&self, conn: &Connection) -> Option<(u32, &str)> {
        self.inode_to_pid
            .get(&conn.inode)
            .map(|(pid, name)| (*pid, name.as_str()))
    }

    /// Count by state
    pub fn count_by_state(&self) -> HashMap<ConnState, usize> {
        let mut counts = HashMap::new();
        for conn in &self.connections {
            *counts.entry(conn.state).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for ConnectionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_analyzer_creation() {
        let analyzer = ConnectionAnalyzer::new();
        assert!(analyzer.connections().is_empty());
    }

    #[test]
    fn test_conn_state_from_hex() {
        assert_eq!(ConnState::from_hex("01"), ConnState::Established);
        assert_eq!(ConnState::from_hex("0A"), ConnState::Listen);
        assert_eq!(ConnState::from_hex("FF"), ConnState::Unknown);
    }

    #[test]
    fn test_parse_addr() {
        // 127.0.0.1:631 in hex (little endian)
        let result = ConnectionAnalyzer::parse_addr("0100007F:0277");
        assert!(result.is_some());
        let (ip, port) = result.unwrap();
        assert_eq!(ip, Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(port, 631);
    }

    #[test]
    fn test_connection_methods() {
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(192, 168, 1, 100),
            local_port: 54321,
            remote_ip: Ipv4Addr::new(8, 8, 8, 8),
            remote_port: 443,
            state: ConnState::Established,
            inode: 12345,
            uid: 1000,
            tx_queue: 0,
            rx_queue: 0,
        };

        assert!(conn.is_outbound());
        assert!(!conn.is_listening());
        assert_eq!(conn.remote_addr(), "8.8.8.8:443");
    }

    #[test]
    fn test_analyzer_collect_safe() {
        let mut analyzer = ConnectionAnalyzer::new();
        analyzer.collect();
        // Should not panic, may or may not have connections
    }
}
