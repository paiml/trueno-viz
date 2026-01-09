//! Network connection tracking - Little Snitch style.
//!
//! On Linux: Parses /proc/net/tcp and /proc/net/udp for active connections.
//! On macOS: Uses netstat -anp tcp/udp for connections.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use std::fs;

#[cfg(target_os = "macos")]
use std::process::Command;

/// Well-known port to service/protocol mapping
pub fn port_to_service(port: u16) -> Option<&'static str> {
    match port {
        20 => Some("FTP-D"),
        21 => Some("FTP"),
        22 => Some("SSH"),
        23 => Some("Telnet"),
        25 => Some("SMTP"),
        53 => Some("DNS"),
        67 | 68 => Some("DHCP"),
        80 => Some("HTTP"),
        110 => Some("POP3"),
        123 => Some("NTP"),
        143 => Some("IMAP"),
        161 | 162 => Some("SNMP"),
        389 => Some("LDAP"),
        443 => Some("HTTPS"),
        445 => Some("SMB"),
        465 => Some("SMTPS"),
        514 => Some("Syslog"),
        587 => Some("Submit"),
        636 => Some("LDAPS"),
        993 => Some("IMAPS"),
        995 => Some("POP3S"),
        1433 => Some("MSSQL"),
        1521 => Some("Oracle"),
        3306 => Some("MySQL"),
        3389 => Some("RDP"),
        5432 => Some("PgSQL"),
        5672 => Some("AMQP"),
        5900 => Some("VNC"),
        6379 => Some("Redis"),
        6443 => Some("K8s"),
        8080 => Some("HTTP-Alt"),
        8443 => Some("HTTPS-Alt"),
        9000 => Some("PHP-FPM"),
        9090 => Some("Prometheus"),
        9200 => Some("Elastic"),
        11211 => Some("Memcache"),
        27017 => Some("MongoDB"),
        _ => None,
    }
}

/// Get service icon/emoji for a port
pub fn port_to_icon(port: u16) -> &'static str {
    match port {
        22 => "🔐",
        80 | 8080 => "🌐",
        443 | 8443 => "🔒",
        3306 | 5432 | 1433 | 1521 | 27017 => "🗄",
        6379 | 11211 => "⚡",
        25 | 465 | 587 => "📧",
        53 => "📡",
        _ => "",
    }
}

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
    #[cfg(target_os = "linux")]
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

/// Connection key for tracking duration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConnKey {
    local_port: u16,
    remote_ip: Ipv4Addr,
    remote_port: u16,
}

impl From<&Connection> for ConnKey {
    fn from(conn: &Connection) -> Self {
        Self {
            local_port: conn.local_port,
            remote_ip: conn.remote_ip,
            remote_port: conn.remote_port,
        }
    }
}

/// DNS cache entry
struct DnsEntry {
    hostname: String,
    expires: Instant,
}

/// Connection analyzer with duration tracking, DNS cache, and bandwidth monitoring
pub struct ConnectionAnalyzer {
    connections: Vec<Connection>,
    inode_to_pid: HashMap<u64, (u32, String)>, // inode -> (pid, name)
    /// Track when connections were first seen
    first_seen: HashMap<ConnKey, Instant>,
    /// DNS reverse lookup cache
    dns_cache: HashMap<Ipv4Addr, DnsEntry>,
    /// Previous queue values for bandwidth delta
    prev_queues: HashMap<ConnKey, (u64, u64)>, // (tx, rx)
    /// Current bandwidth deltas
    bandwidth_deltas: HashMap<ConnKey, (u64, u64)>, // (tx_delta, rx_delta)
}

impl ConnectionAnalyzer {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            inode_to_pid: HashMap::new(),
            first_seen: HashMap::new(),
            dns_cache: HashMap::new(),
            prev_queues: HashMap::new(),
            bandwidth_deltas: HashMap::new(),
        }
    }

    /// Collect connection data
    pub fn collect(&mut self) {
        self.connections.clear();
        self.bandwidth_deltas.clear();

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

        // Track first_seen and bandwidth for all connections
        let now = Instant::now();
        let mut current_keys: std::collections::HashSet<ConnKey> = std::collections::HashSet::new();

        for conn in &self.connections {
            let key = ConnKey::from(conn);
            current_keys.insert(key.clone());

            // Track first seen
            self.first_seen.entry(key.clone()).or_insert(now);

            // Calculate bandwidth delta
            if let Some(&(prev_tx, prev_rx)) = self.prev_queues.get(&key) {
                let tx_delta = conn.tx_queue.saturating_sub(prev_tx);
                let rx_delta = conn.rx_queue.saturating_sub(prev_rx);
                if tx_delta > 0 || rx_delta > 0 {
                    self.bandwidth_deltas.insert(key.clone(), (tx_delta, rx_delta));
                }
            }

            // Update prev queues
            self.prev_queues.insert(key, (conn.tx_queue, conn.rx_queue));
        }

        // Clean up stale entries (connections that no longer exist)
        self.first_seen.retain(|k, _| current_keys.contains(k));
        self.prev_queues.retain(|k, _| current_keys.contains(k));

        // Clean expired DNS cache entries
        self.dns_cache.retain(|_, entry| entry.expires > now);
    }

    /// Get connection duration
    pub fn connection_duration(&self, conn: &Connection) -> Option<Duration> {
        let key = ConnKey::from(conn);
        self.first_seen.get(&key).map(|t| t.elapsed())
    }

    /// Format duration as human-readable string
    pub fn format_duration(duration: Duration) -> String {
        let secs = duration.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m{}s", secs / 60, secs % 60)
        } else if secs < 86400 {
            format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
        } else {
            format!("{}d{}h", secs / 86400, (secs % 86400) / 3600)
        }
    }

    /// Get bandwidth delta for a connection
    pub fn bandwidth_delta(&self, conn: &Connection) -> Option<(u64, u64)> {
        let key = ConnKey::from(conn);
        self.bandwidth_deltas.get(&key).copied()
    }

    /// Check if connection is "hot" (high bandwidth)
    pub fn is_hot_connection(&self, conn: &Connection) -> bool {
        if let Some((tx, rx)) = self.bandwidth_delta(conn) {
            tx > 1000 || rx > 1000 // More than 1KB queued
        } else {
            false
        }
    }

    /// Get DNS hostname for an IP (returns cached value)
    pub fn get_hostname(&self, ip: Ipv4Addr) -> Option<&str> {
        self.dns_cache.get(&ip).and_then(|e| {
            if e.hostname.is_empty() || e.expires < Instant::now() {
                None
            } else {
                Some(e.hostname.as_str())
            }
        })
    }

    /// Lookup and cache DNS hostname for an IP
    pub fn lookup_hostname(&mut self, ip: Ipv4Addr) {
        // Skip lookups for private/local IPs
        if ip.is_private() || ip.is_loopback() || ip.is_unspecified() {
            return;
        }

        // Skip if already cached
        if self.dns_cache.contains_key(&ip) {
            return;
        }

        // Do synchronous lookup and cache
        let hostname = Self::reverse_dns_lookup(ip);
        let entry = DnsEntry {
            hostname: hostname.unwrap_or_default(),
            expires: Instant::now() + Duration::from_secs(300), // 5 min TTL
        };
        self.dns_cache.insert(ip, entry);
    }

    /// Perform reverse DNS lookup (blocking)
    fn reverse_dns_lookup(ip: Ipv4Addr) -> Option<String> {
        use std::net::ToSocketAddrs;

        // Try to resolve - this is blocking but we cache results
        let addr = format!("{}:0", ip);
        if let Ok(mut addrs) = addr.to_socket_addrs() {
            if let Some(socket_addr) = addrs.next() {
                // Use DNS lookup via system resolver
                #[cfg(target_os = "linux")]
                {
                    // Try reading from /etc/hosts or use getaddrinfo
                    if let Ok(output) = std::process::Command::new("getent")
                        .args(["hosts", &ip.to_string()])
                        .output()
                    {
                        if output.status.success() {
                            let result = String::from_utf8_lossy(&output.stdout);
                            if let Some(hostname) = result.split_whitespace().nth(1) {
                                return Some(hostname.to_string());
                            }
                        }
                    }
                }
                let _ = socket_addr; // Suppress unused warning
            }
        }
        None
    }

    /// Get service name for connection's remote port
    pub fn service_name(&self, conn: &Connection) -> Option<&'static str> {
        // Check remote port first (for outbound connections)
        if let Some(svc) = port_to_service(conn.remote_port) {
            return Some(svc);
        }
        // Check local port (for listening/inbound)
        port_to_service(conn.local_port)
    }

    /// Get service icon for connection
    pub fn service_icon(&self, conn: &Connection) -> &'static str {
        let icon = port_to_icon(conn.remote_port);
        if icon.is_empty() {
            port_to_icon(conn.local_port)
        } else {
            icon
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
    #[cfg(target_os = "linux")]
    fn test_conn_state_from_hex() {
        assert_eq!(ConnState::from_hex("01"), ConnState::Established);
        assert_eq!(ConnState::from_hex("0A"), ConnState::Listen);
        assert_eq!(ConnState::from_hex("FF"), ConnState::Unknown);
    }

    #[test]
    #[cfg(target_os = "linux")]
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

    #[test]
    fn test_port_to_service_common() {
        assert_eq!(port_to_service(22), Some("SSH"));
        assert_eq!(port_to_service(80), Some("HTTP"));
        assert_eq!(port_to_service(443), Some("HTTPS"));
        assert_eq!(port_to_service(53), Some("DNS"));
        assert_eq!(port_to_service(25), Some("SMTP"));
    }

    #[test]
    fn test_port_to_service_databases() {
        assert_eq!(port_to_service(3306), Some("MySQL"));
        assert_eq!(port_to_service(5432), Some("PgSQL"));
        assert_eq!(port_to_service(6379), Some("Redis"));
        assert_eq!(port_to_service(27017), Some("MongoDB"));
        assert_eq!(port_to_service(1433), Some("MSSQL"));
        assert_eq!(port_to_service(1521), Some("Oracle"));
    }

    #[test]
    fn test_port_to_service_all() {
        assert_eq!(port_to_service(20), Some("FTP-D"));
        assert_eq!(port_to_service(21), Some("FTP"));
        assert_eq!(port_to_service(23), Some("Telnet"));
        assert_eq!(port_to_service(67), Some("DHCP"));
        assert_eq!(port_to_service(68), Some("DHCP"));
        assert_eq!(port_to_service(110), Some("POP3"));
        assert_eq!(port_to_service(123), Some("NTP"));
        assert_eq!(port_to_service(143), Some("IMAP"));
        assert_eq!(port_to_service(161), Some("SNMP"));
        assert_eq!(port_to_service(162), Some("SNMP"));
        assert_eq!(port_to_service(389), Some("LDAP"));
        assert_eq!(port_to_service(445), Some("SMB"));
        assert_eq!(port_to_service(465), Some("SMTPS"));
        assert_eq!(port_to_service(514), Some("Syslog"));
        assert_eq!(port_to_service(587), Some("Submit"));
        assert_eq!(port_to_service(636), Some("LDAPS"));
        assert_eq!(port_to_service(993), Some("IMAPS"));
        assert_eq!(port_to_service(995), Some("POP3S"));
        assert_eq!(port_to_service(3389), Some("RDP"));
        assert_eq!(port_to_service(5672), Some("AMQP"));
        assert_eq!(port_to_service(5900), Some("VNC"));
        assert_eq!(port_to_service(6443), Some("K8s"));
        assert_eq!(port_to_service(8080), Some("HTTP-Alt"));
        assert_eq!(port_to_service(8443), Some("HTTPS-Alt"));
        assert_eq!(port_to_service(9000), Some("PHP-FPM"));
        assert_eq!(port_to_service(9090), Some("Prometheus"));
        assert_eq!(port_to_service(9200), Some("Elastic"));
        assert_eq!(port_to_service(11211), Some("Memcache"));
    }

    #[test]
    fn test_port_to_service_unknown() {
        assert_eq!(port_to_service(12345), None);
        assert_eq!(port_to_service(0), None);
        assert_eq!(port_to_service(65535), None);
    }

    #[test]
    fn test_port_to_icon_web() {
        assert_eq!(port_to_icon(80), "🌐");
        assert_eq!(port_to_icon(8080), "🌐");
        assert_eq!(port_to_icon(443), "🔒");
        assert_eq!(port_to_icon(8443), "🔒");
    }

    #[test]
    fn test_port_to_icon_databases() {
        assert_eq!(port_to_icon(3306), "🗄");
        assert_eq!(port_to_icon(5432), "🗄");
        assert_eq!(port_to_icon(1433), "🗄");
        assert_eq!(port_to_icon(1521), "🗄");
        assert_eq!(port_to_icon(27017), "🗄");
    }

    #[test]
    fn test_port_to_icon_cache() {
        assert_eq!(port_to_icon(6379), "⚡");
        assert_eq!(port_to_icon(11211), "⚡");
    }

    #[test]
    fn test_port_to_icon_mail() {
        assert_eq!(port_to_icon(25), "📧");
        assert_eq!(port_to_icon(465), "📧");
        assert_eq!(port_to_icon(587), "📧");
    }

    #[test]
    fn test_port_to_icon_misc() {
        assert_eq!(port_to_icon(22), "🔐");
        assert_eq!(port_to_icon(53), "📡");
        assert_eq!(port_to_icon(12345), "");
    }

    #[test]
    fn test_conn_state_as_char() {
        assert_eq!(ConnState::Established.as_char(), 'E');
        assert_eq!(ConnState::Listen.as_char(), 'L');
        assert_eq!(ConnState::TimeWait.as_char(), 'T');
        assert_eq!(ConnState::CloseWait.as_char(), 'C');
        assert_eq!(ConnState::SynSent.as_char(), 'S');
        assert_eq!(ConnState::SynRecv.as_char(), 'R');
        assert_eq!(ConnState::FinWait1.as_char(), 'F');
        assert_eq!(ConnState::FinWait2.as_char(), 'F');
        assert_eq!(ConnState::Closing.as_char(), 'X');
        assert_eq!(ConnState::LastAck.as_char(), 'X');
        assert_eq!(ConnState::Close.as_char(), 'X');
        assert_eq!(ConnState::Unknown.as_char(), '?');
    }

    #[test]
    fn test_connection_local_addr() {
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
        assert_eq!(conn.local_addr(), "192.168.1.100:54321");
    }

    #[test]
    fn test_connection_remote_addr_unspecified() {
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(0, 0, 0, 0),
            local_port: 80,
            remote_ip: Ipv4Addr::new(0, 0, 0, 0),
            remote_port: 0,
            state: ConnState::Listen,
            inode: 12345,
            uid: 0,
            tx_queue: 0,
            rx_queue: 0,
        };
        assert_eq!(conn.remote_addr(), "*");
        assert!(conn.is_listening());
        assert!(!conn.is_outbound());
    }

    #[test]
    fn test_connection_not_outbound_low_port() {
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(192, 168, 1, 100),
            local_port: 80, // Low port
            remote_ip: Ipv4Addr::new(8, 8, 8, 8),
            remote_port: 54321,
            state: ConnState::Established,
            inode: 12345,
            uid: 1000,
            tx_queue: 0,
            rx_queue: 0,
        };
        assert!(!conn.is_outbound()); // Low local port = not outbound
    }

    #[test]
    fn test_protocol_enum() {
        let tcp = Protocol::Tcp;
        let udp = Protocol::Udp;
        assert_ne!(tcp, udp);
    }

    #[test]
    fn test_conn_key_from() {
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
        let key = ConnKey::from(&conn);
        assert_eq!(key.local_port, 54321);
        assert_eq!(key.remote_ip, Ipv4Addr::new(8, 8, 8, 8));
        assert_eq!(key.remote_port, 443);
    }

    #[test]
    fn test_format_duration_seconds() {
        use std::time::Duration;
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        use std::time::Duration;
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(60)), "1m0s");
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(90)), "1m30s");
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(3599)), "59m59s");
    }

    #[test]
    fn test_format_duration_hours() {
        use std::time::Duration;
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(3600)), "1h0m");
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(7200)), "2h0m");
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(86399)), "23h59m");
    }

    #[test]
    fn test_format_duration_days() {
        use std::time::Duration;
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(86400)), "1d0h");
        assert_eq!(ConnectionAnalyzer::format_duration(Duration::from_secs(172800)), "2d0h");
    }

    #[test]
    fn test_active_connections() {
        let analyzer = ConnectionAnalyzer::new();
        let active = analyzer.active_connections();
        // Should return empty or active connections
        assert!(active.len() <= analyzer.connections().len());
    }

    #[test]
    fn test_listening_connections() {
        let analyzer = ConnectionAnalyzer::new();
        let listening = analyzer.listening();
        // Should return empty or listening connections
        for conn in &listening {
            assert!(conn.is_listening());
        }
    }

    #[test]
    fn test_service_name() {
        let analyzer = ConnectionAnalyzer::new();
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(0, 0, 0, 0),
            local_port: 80,
            remote_ip: Ipv4Addr::new(0, 0, 0, 0),
            remote_port: 0,
            state: ConnState::Listen,
            inode: 0,
            uid: 0,
            tx_queue: 0,
            rx_queue: 0,
        };
        // Listening on port 80 should be HTTP
        assert_eq!(analyzer.service_name(&conn), Some("HTTP"));
    }

    #[test]
    fn test_service_icon() {
        let analyzer = ConnectionAnalyzer::new();
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(192, 168, 1, 1),
            local_port: 54321,
            remote_ip: Ipv4Addr::new(8, 8, 8, 8),
            remote_port: 443,
            state: ConnState::Established,
            inode: 0,
            uid: 0,
            tx_queue: 0,
            rx_queue: 0,
        };
        // Remote port 443 should get lock icon
        assert_eq!(analyzer.service_icon(&conn), "🔒");
    }

    #[test]
    fn test_is_hot_connection() {
        let mut analyzer = ConnectionAnalyzer::new();
        analyzer.collect();
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(192, 168, 1, 1),
            local_port: 54321,
            remote_ip: Ipv4Addr::new(8, 8, 8, 8),
            remote_port: 443,
            state: ConnState::Established,
            inode: 0,
            uid: 0,
            tx_queue: 0,
            rx_queue: 0,
        };
        // A new connection shouldn't be hot (no history)
        let _ = analyzer.is_hot_connection(&conn);
    }

    #[test]
    fn test_connection_duration() {
        let mut analyzer = ConnectionAnalyzer::new();
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(192, 168, 1, 1),
            local_port: 54321,
            remote_ip: Ipv4Addr::new(8, 8, 8, 8),
            remote_port: 443,
            state: ConnState::Established,
            inode: 0,
            uid: 0,
            tx_queue: 0,
            rx_queue: 0,
        };
        // Duration for new connection should be None
        assert!(analyzer.connection_duration(&conn).is_none());
    }

    #[test]
    fn test_bandwidth_delta() {
        let mut analyzer = ConnectionAnalyzer::new();
        let conn = Connection {
            protocol: Protocol::Tcp,
            local_ip: Ipv4Addr::new(192, 168, 1, 1),
            local_port: 54321,
            remote_ip: Ipv4Addr::new(8, 8, 8, 8),
            remote_port: 443,
            state: ConnState::Established,
            inode: 0,
            uid: 0,
            tx_queue: 100,
            rx_queue: 200,
        };
        // No history, should return None
        assert!(analyzer.bandwidth_delta(&conn).is_none());
    }

    #[test]
    fn test_get_hostname_unknown() {
        let analyzer = ConnectionAnalyzer::new();
        let ip = Ipv4Addr::new(1, 2, 3, 4);
        // Unknown IP should return None
        assert!(analyzer.get_hostname(ip).is_none());
    }
}
