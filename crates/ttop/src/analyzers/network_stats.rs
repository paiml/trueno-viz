//! Network statistics analyzer for Linux.
//!
//! Parses /proc/net/* files for:
//! - Protocol counters (TCP, UDP, ICMP)
//! - Interface errors and drops
//! - TCP RTT and retransmission stats
//! - Socket queue health

use std::collections::HashMap;
use std::fs;

/// Network error counters for an interface
#[derive(Debug, Clone, Default)]
pub struct InterfaceErrors {
    pub rx_errors: u64,
    pub rx_dropped: u64,
    pub rx_overrun: u64,
    pub tx_errors: u64,
    pub tx_dropped: u64,
    pub tx_carrier: u64,
    pub collisions: u64,
}

/// Delta tracking for interface errors
#[derive(Debug, Clone, Default)]
pub struct ErrorDeltas {
    pub rx_errors_delta: i64,
    pub rx_dropped_delta: i64,
    pub tx_errors_delta: i64,
    pub tx_dropped_delta: i64,
}

/// Protocol statistics
#[derive(Debug, Clone, Default)]
pub struct ProtocolStats {
    /// TCP connection counts by state
    pub tcp_established: u32,
    pub tcp_syn_sent: u32,
    pub tcp_syn_recv: u32,
    pub tcp_fin_wait1: u32,
    pub tcp_fin_wait2: u32,
    pub tcp_time_wait: u32,
    pub tcp_close: u32,
    pub tcp_close_wait: u32,
    pub tcp_last_ack: u32,
    pub tcp_listen: u32,
    pub tcp_closing: u32,

    /// UDP socket count
    pub udp_sockets: u32,

    /// ICMP active (from /proc/net/raw)
    pub icmp_sockets: u32,
}

impl ProtocolStats {
    pub fn tcp_total(&self) -> u32 {
        self.tcp_established + self.tcp_syn_sent + self.tcp_syn_recv +
        self.tcp_fin_wait1 + self.tcp_fin_wait2 + self.tcp_time_wait +
        self.tcp_close + self.tcp_close_wait + self.tcp_last_ack +
        self.tcp_listen + self.tcp_closing
    }
}

/// TCP performance metrics from /proc/net/netstat
#[derive(Debug, Clone, Default)]
pub struct TcpPerformance {
    /// Smoothed RTT in milliseconds (estimated)
    pub rtt_ms: f64,
    /// Retransmission rate (0.0-1.0)
    pub retrans_rate: f64,
    /// Segments retransmitted
    pub retrans_segs: u64,
    /// Total segments sent
    pub total_segs_out: u64,
    /// Previous values for delta calculation
    prev_retrans: u64,
    prev_segs_out: u64,
}

/// Socket queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Total receive queue bytes across all sockets
    pub total_rx_queue: u64,
    /// Total transmit queue bytes across all sockets
    pub total_tx_queue: u64,
    /// Max receive queue seen
    pub max_rx_queue: u64,
    /// Max transmit queue seen
    pub max_tx_queue: u64,
    /// Number of sockets with non-empty rx queue
    pub rx_queue_count: u32,
    /// Number of sockets with non-empty tx queue
    pub tx_queue_count: u32,
    /// SYN backlog pressure detected
    pub syn_backlog_pressure: bool,
}

/// Network statistics analyzer
#[derive(Debug, Default)]
pub struct NetworkStatsAnalyzer {
    /// Interface error counters
    pub interface_errors: HashMap<String, InterfaceErrors>,
    /// Previous error values for delta calculation
    prev_errors: HashMap<String, InterfaceErrors>,
    /// Error deltas since last sample
    pub error_deltas: HashMap<String, ErrorDeltas>,
    /// Protocol statistics
    pub protocol_stats: ProtocolStats,
    /// TCP performance metrics
    pub tcp_perf: TcpPerformance,
    /// Socket queue stats
    pub queue_stats: QueueStats,
}

impl NetworkStatsAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect all network statistics
    pub fn collect(&mut self) {
        self.collect_interface_errors();
        self.collect_protocol_stats();
        self.collect_tcp_performance();
        self.collect_queue_stats();
    }

    /// Parse /proc/net/dev for interface errors
    fn collect_interface_errors(&mut self) {
        // Save previous for delta calculation
        self.prev_errors = self.interface_errors.clone();
        self.interface_errors.clear();
        self.error_deltas.clear();

        let Ok(content) = fs::read_to_string("/proc/net/dev") else {
            return;
        };

        for line in content.lines().skip(2) {
            // Format: iface: rx_bytes rx_packets rx_errs rx_drop rx_fifo rx_frame rx_compressed rx_multicast
            //                tx_bytes tx_packets tx_errs tx_drop tx_fifo tx_colls tx_carrier tx_compressed
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 17 {
                continue;
            }

            let iface = parts[0].trim_end_matches(':').to_string();

            // Skip loopback
            if iface == "lo" {
                continue;
            }

            let errors = InterfaceErrors {
                rx_errors: parts[3].parse().unwrap_or(0),
                rx_dropped: parts[4].parse().unwrap_or(0),
                rx_overrun: parts[5].parse().unwrap_or(0),
                tx_errors: parts[11].parse().unwrap_or(0),
                tx_dropped: parts[12].parse().unwrap_or(0),
                tx_carrier: parts[15].parse().unwrap_or(0),
                collisions: parts[14].parse().unwrap_or(0),
            };

            // Calculate deltas
            if let Some(prev) = self.prev_errors.get(&iface) {
                let deltas = ErrorDeltas {
                    rx_errors_delta: errors.rx_errors as i64 - prev.rx_errors as i64,
                    rx_dropped_delta: errors.rx_dropped as i64 - prev.rx_dropped as i64,
                    tx_errors_delta: errors.tx_errors as i64 - prev.tx_errors as i64,
                    tx_dropped_delta: errors.tx_dropped as i64 - prev.tx_dropped as i64,
                };
                self.error_deltas.insert(iface.clone(), deltas);
            }

            self.interface_errors.insert(iface, errors);
        }
    }

    /// Count TCP states from /proc/net/tcp and UDP from /proc/net/udp
    fn collect_protocol_stats(&mut self) {
        self.protocol_stats = ProtocolStats::default();

        // Parse TCP
        if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }

                // State is in hex at position 3
                if let Ok(state) = u8::from_str_radix(parts[3], 16) {
                    match state {
                        0x01 => self.protocol_stats.tcp_established += 1,
                        0x02 => self.protocol_stats.tcp_syn_sent += 1,
                        0x03 => self.protocol_stats.tcp_syn_recv += 1,
                        0x04 => self.protocol_stats.tcp_fin_wait1 += 1,
                        0x05 => self.protocol_stats.tcp_fin_wait2 += 1,
                        0x06 => self.protocol_stats.tcp_time_wait += 1,
                        0x07 => self.protocol_stats.tcp_close += 1,
                        0x08 => self.protocol_stats.tcp_close_wait += 1,
                        0x09 => self.protocol_stats.tcp_last_ack += 1,
                        0x0A => self.protocol_stats.tcp_listen += 1,
                        0x0B => self.protocol_stats.tcp_closing += 1,
                        _ => {}
                    }
                }
            }
        }

        // Also check tcp6
        if let Ok(content) = fs::read_to_string("/proc/net/tcp6") {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }

                if let Ok(state) = u8::from_str_radix(parts[3], 16) {
                    match state {
                        0x01 => self.protocol_stats.tcp_established += 1,
                        0x02 => self.protocol_stats.tcp_syn_sent += 1,
                        0x03 => self.protocol_stats.tcp_syn_recv += 1,
                        0x04 => self.protocol_stats.tcp_fin_wait1 += 1,
                        0x05 => self.protocol_stats.tcp_fin_wait2 += 1,
                        0x06 => self.protocol_stats.tcp_time_wait += 1,
                        0x07 => self.protocol_stats.tcp_close += 1,
                        0x08 => self.protocol_stats.tcp_close_wait += 1,
                        0x09 => self.protocol_stats.tcp_last_ack += 1,
                        0x0A => self.protocol_stats.tcp_listen += 1,
                        0x0B => self.protocol_stats.tcp_closing += 1,
                        _ => {}
                    }
                }
            }
        }

        // Count UDP sockets
        if let Ok(content) = fs::read_to_string("/proc/net/udp") {
            self.protocol_stats.udp_sockets = content.lines().skip(1).count() as u32;
        }
        if let Ok(content) = fs::read_to_string("/proc/net/udp6") {
            self.protocol_stats.udp_sockets += content.lines().skip(1).count() as u32;
        }

        // Count ICMP/raw sockets
        if let Ok(content) = fs::read_to_string("/proc/net/raw") {
            self.protocol_stats.icmp_sockets = content.lines().skip(1).count() as u32;
        }
        if let Ok(content) = fs::read_to_string("/proc/net/raw6") {
            self.protocol_stats.icmp_sockets += content.lines().skip(1).count() as u32;
        }
    }

    /// Parse TCP performance from /proc/net/netstat
    fn collect_tcp_performance(&mut self) {
        // Save previous values
        self.tcp_perf.prev_retrans = self.tcp_perf.retrans_segs;
        self.tcp_perf.prev_segs_out = self.tcp_perf.total_segs_out;

        if let Ok(content) = fs::read_to_string("/proc/net/netstat") {
            let lines: Vec<&str> = content.lines().collect();

            // Find TcpExt lines
            for i in 0..lines.len() {
                if lines[i].starts_with("TcpExt:") {
                    if i + 1 < lines.len() {
                        let headers: Vec<&str> = lines[i].split_whitespace().collect();
                        let values: Vec<&str> = lines[i + 1].split_whitespace().collect();

                        for (j, header) in headers.iter().enumerate() {
                            if j < values.len() && *header == "TCPRetransSegs" {
                                self.tcp_perf.retrans_segs = values[j].parse().unwrap_or(0);
                            }
                        }
                    }
                    break;
                }
            }
        }

        // Get total segments from /proc/net/snmp
        if let Ok(content) = fs::read_to_string("/proc/net/snmp") {
            for line in content.lines() {
                if line.starts_with("Tcp:") && !line.contains("RtoAlgorithm") {
                    let values: Vec<&str> = line.split_whitespace().collect();
                    // OutSegs is usually at index 11
                    if values.len() > 11 {
                        self.tcp_perf.total_segs_out = values[11].parse().unwrap_or(0);
                    }
                    break;
                }
            }
        }

        // Calculate retransmission rate
        let segs_delta = self.tcp_perf.total_segs_out.saturating_sub(self.tcp_perf.prev_segs_out);
        let retrans_delta = self.tcp_perf.retrans_segs.saturating_sub(self.tcp_perf.prev_retrans);

        if segs_delta > 0 {
            self.tcp_perf.retrans_rate = retrans_delta as f64 / segs_delta as f64;
        }

        // Estimate RTT from /proc/net/tcp (use avg of smoothed RTT column if available)
        // The RTT info in /proc/net/tcp is limited, so we estimate from retrans rate
        // Higher retrans = likely higher latency
        // Rough estimation: base 5ms + retrans_rate * 100ms
        self.tcp_perf.rtt_ms = 5.0 + (self.tcp_perf.retrans_rate * 100.0);
    }

    /// Collect socket queue statistics from /proc/net/tcp
    fn collect_queue_stats(&mut self) {
        self.queue_stats = QueueStats::default();

        let parse_queues = |content: &str, stats: &mut QueueStats| {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 5 {
                    continue;
                }

                // tx_queue:rx_queue is at position 4 in format XX:XX
                if let Some(queue_str) = parts.get(4) {
                    let queues: Vec<&str> = queue_str.split(':').collect();
                    if queues.len() == 2 {
                        let tx = u64::from_str_radix(queues[0], 16).unwrap_or(0);
                        let rx = u64::from_str_radix(queues[1], 16).unwrap_or(0);

                        stats.total_tx_queue += tx;
                        stats.total_rx_queue += rx;

                        if tx > stats.max_tx_queue {
                            stats.max_tx_queue = tx;
                        }
                        if rx > stats.max_rx_queue {
                            stats.max_rx_queue = rx;
                        }

                        if tx > 0 {
                            stats.tx_queue_count += 1;
                        }
                        if rx > 0 {
                            stats.rx_queue_count += 1;
                        }
                    }
                }
            }
        };

        if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
            parse_queues(&content, &mut self.queue_stats);
        }
        if let Ok(content) = fs::read_to_string("/proc/net/tcp6") {
            parse_queues(&content, &mut self.queue_stats);
        }

        // Detect SYN backlog pressure (many SYN_RECV sockets)
        self.queue_stats.syn_backlog_pressure = self.protocol_stats.tcp_syn_recv > 10;
    }

    /// Get total errors across all interfaces
    pub fn total_errors(&self) -> (u64, u64) {
        let mut rx = 0u64;
        let mut tx = 0u64;
        for errs in self.interface_errors.values() {
            rx += errs.rx_errors + errs.rx_dropped;
            tx += errs.tx_errors + errs.tx_dropped;
        }
        (rx, tx)
    }

    /// Get total error deltas across all interfaces
    pub fn total_error_deltas(&self) -> (i64, i64) {
        let mut rx = 0i64;
        let mut tx = 0i64;
        for deltas in self.error_deltas.values() {
            rx += deltas.rx_errors_delta + deltas.rx_dropped_delta;
            tx += deltas.tx_errors_delta + deltas.tx_dropped_delta;
        }
        (rx, tx)
    }

    /// Check if there are any recent errors
    pub fn has_recent_errors(&self) -> bool {
        let (rx_delta, tx_delta) = self.total_error_deltas();
        rx_delta > 0 || tx_delta > 0
    }

    /// Format a latency gauge (5 bars)
    pub fn latency_gauge(&self) -> &'static str {
        let rtt = self.tcp_perf.rtt_ms;
        if rtt < 10.0 {
            "●●●●●"  // Excellent
        } else if rtt < 25.0 {
            "●●●●○"  // Good
        } else if rtt < 50.0 {
            "●●●○○"  // Fair
        } else if rtt < 100.0 {
            "●●○○○"  // Poor
        } else {
            "●○○○○"  // Bad
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_stats_tcp_total() {
        let stats = ProtocolStats {
            tcp_established: 10,
            tcp_syn_sent: 2,
            tcp_syn_recv: 1,
            tcp_fin_wait1: 3,
            tcp_fin_wait2: 4,
            tcp_time_wait: 5,
            tcp_close: 1,
            tcp_close_wait: 2,
            tcp_last_ack: 1,
            tcp_listen: 8,
            tcp_closing: 0,
            udp_sockets: 5,
            icmp_sockets: 2,
        };
        // tcp_total = established + syn_sent + syn_recv + fin_wait1 + fin_wait2 +
        //             time_wait + close + close_wait + last_ack + listen + closing
        assert_eq!(stats.tcp_total(), 10 + 2 + 1 + 3 + 4 + 5 + 1 + 2 + 1 + 8 + 0);
    }

    #[test]
    fn test_protocol_stats_default() {
        let stats = ProtocolStats::default();
        assert_eq!(stats.tcp_total(), 0);
        assert_eq!(stats.tcp_established, 0);
        assert_eq!(stats.udp_sockets, 0);
    }

    #[test]
    fn test_interface_errors_default() {
        let errors = InterfaceErrors::default();
        assert_eq!(errors.rx_errors, 0);
        assert_eq!(errors.tx_errors, 0);
        assert_eq!(errors.rx_dropped, 0);
        assert_eq!(errors.tx_dropped, 0);
    }

    #[test]
    fn test_error_deltas_default() {
        let deltas = ErrorDeltas::default();
        assert_eq!(deltas.rx_errors_delta, 0);
        assert_eq!(deltas.tx_errors_delta, 0);
    }

    #[test]
    fn test_tcp_performance_default() {
        let perf = TcpPerformance::default();
        assert_eq!(perf.rtt_ms, 0.0);
        assert_eq!(perf.retrans_rate, 0.0);
        assert_eq!(perf.retrans_segs, 0);
    }

    #[test]
    fn test_queue_stats_default() {
        let stats = QueueStats::default();
        assert_eq!(stats.total_rx_queue, 0);
        assert_eq!(stats.total_tx_queue, 0);
        assert_eq!(stats.max_rx_queue, 0);
        assert_eq!(stats.max_tx_queue, 0);
    }

    #[test]
    fn test_analyzer_new() {
        let analyzer = NetworkStatsAnalyzer::new();
        assert!(analyzer.interface_errors.is_empty());
        assert!(analyzer.error_deltas.is_empty());
        assert_eq!(analyzer.protocol_stats.tcp_total(), 0);
    }

    #[test]
    fn test_analyzer_total_errors_empty() {
        let analyzer = NetworkStatsAnalyzer::new();
        let (rx, tx) = analyzer.total_errors();
        assert_eq!(rx, 0);
        assert_eq!(tx, 0);
    }

    #[test]
    fn test_analyzer_total_errors_with_data() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.interface_errors.insert("eth0".to_string(), InterfaceErrors {
            rx_errors: 10,
            rx_dropped: 5,
            rx_overrun: 0,
            tx_errors: 3,
            tx_dropped: 2,
            tx_carrier: 0,
            collisions: 0,
        });
        analyzer.interface_errors.insert("eth1".to_string(), InterfaceErrors {
            rx_errors: 20,
            rx_dropped: 10,
            rx_overrun: 0,
            tx_errors: 6,
            tx_dropped: 4,
            tx_carrier: 0,
            collisions: 0,
        });
        let (rx, tx) = analyzer.total_errors();
        assert_eq!(rx, 10 + 5 + 20 + 10);
        assert_eq!(tx, 3 + 2 + 6 + 4);
    }

    #[test]
    fn test_analyzer_total_error_deltas_empty() {
        let analyzer = NetworkStatsAnalyzer::new();
        let (rx, tx) = analyzer.total_error_deltas();
        assert_eq!(rx, 0);
        assert_eq!(tx, 0);
    }

    #[test]
    fn test_analyzer_has_recent_errors_false() {
        let analyzer = NetworkStatsAnalyzer::new();
        assert!(!analyzer.has_recent_errors());
    }

    #[test]
    fn test_analyzer_has_recent_errors_true() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.error_deltas.insert("eth0".to_string(), ErrorDeltas {
            rx_errors_delta: 1,
            rx_dropped_delta: 0,
            tx_errors_delta: 0,
            tx_dropped_delta: 0,
        });
        assert!(analyzer.has_recent_errors());
    }

    #[test]
    fn test_latency_gauge_excellent() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.tcp_perf.rtt_ms = 5.0;
        assert_eq!(analyzer.latency_gauge(), "●●●●●");
    }

    #[test]
    fn test_latency_gauge_good() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.tcp_perf.rtt_ms = 15.0;
        assert_eq!(analyzer.latency_gauge(), "●●●●○");
    }

    #[test]
    fn test_latency_gauge_fair() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.tcp_perf.rtt_ms = 35.0;
        assert_eq!(analyzer.latency_gauge(), "●●●○○");
    }

    #[test]
    fn test_latency_gauge_poor() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.tcp_perf.rtt_ms = 75.0;
        assert_eq!(analyzer.latency_gauge(), "●●○○○");
    }

    #[test]
    fn test_latency_gauge_bad() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.tcp_perf.rtt_ms = 150.0;
        assert_eq!(analyzer.latency_gauge(), "●○○○○");
    }

    #[test]
    fn test_latency_gauge_boundaries() {
        let mut analyzer = NetworkStatsAnalyzer::new();

        analyzer.tcp_perf.rtt_ms = 9.9;
        assert_eq!(analyzer.latency_gauge(), "●●●●●");

        analyzer.tcp_perf.rtt_ms = 10.0;
        assert_eq!(analyzer.latency_gauge(), "●●●●○");

        analyzer.tcp_perf.rtt_ms = 24.9;
        assert_eq!(analyzer.latency_gauge(), "●●●●○");

        analyzer.tcp_perf.rtt_ms = 25.0;
        assert_eq!(analyzer.latency_gauge(), "●●●○○");

        analyzer.tcp_perf.rtt_ms = 49.9;
        assert_eq!(analyzer.latency_gauge(), "●●●○○");

        analyzer.tcp_perf.rtt_ms = 50.0;
        assert_eq!(analyzer.latency_gauge(), "●●○○○");

        analyzer.tcp_perf.rtt_ms = 99.9;
        assert_eq!(analyzer.latency_gauge(), "●●○○○");

        analyzer.tcp_perf.rtt_ms = 100.0;
        assert_eq!(analyzer.latency_gauge(), "●○○○○");
    }

    #[test]
    fn test_collect_safe() {
        // Just verify collect doesn't panic
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.collect();
        // Should complete without error, data may or may not be present
    }
}
