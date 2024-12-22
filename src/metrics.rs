use prometheus_client::metrics::{counter::Counter, gauge::Gauge, histogram::Histogram};
use prometheus_client::registry::Registry;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing;

/// NetworkMetrics tracks the health and performance of the network.
/// This includes peer connections, message statistics, and regional distribution.
pub struct NetworkMetrics {
    // Basic peer metrics
    active_peers: Gauge,
    total_connections: Counter,
    disconnections: Counter,
    
    // Message metrics
    messages_sent: Counter,
    messages_received: Counter,
    message_sizes: Histogram,
    
    // Regional tracking
    peers_by_region: std::collections::HashMap<String, Gauge>,
    
    // Health tracking
    last_update: Arc<Mutex<Instant>>,
}

impl NetworkMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        // Initialize basic peer metrics
        let active_peers = Gauge::default();
        registry.register(
            "romer_active_peers",
            "Number of currently connected peers",
            active_peers.clone(),
        );

        let total_connections = Counter::default();
        registry.register(
            "romer_total_connections",
            "Total peer connections since startup",
            total_connections.clone(),
        );

        let disconnections = Counter::default();
        registry.register(
            "romer_disconnections",
            "Total peer disconnections since startup",
            disconnections.clone(),
        );

        // Initialize message metrics
        let messages_sent = Counter::default();
        registry.register(
            "romer_messages_sent",
            "Total messages sent",
            messages_sent.clone(),
        );

        let messages_received = Counter::default();
        registry.register(
            "romer_messages_received",
            "Total messages received",
            messages_received.clone(),
        );

        // Create histogram buckets as a Vec and convert to iterator
        let buckets = vec![64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0];
        let message_sizes = Histogram::new(buckets.into_iter());
        registry.register(
            "romer_message_sizes_bytes",
            "Distribution of message sizes in bytes",
            message_sizes.clone(),
        );

        NetworkMetrics {
            active_peers,
            total_connections,
            disconnections,
            messages_sent,
            messages_received,
            message_sizes,
            peers_by_region: std::collections::HashMap::new(),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn record_connection(&self, peer_id: &[u8], region: &str) {
        self.active_peers.inc();
        self.total_connections.inc();
        
        if let Some(region_gauge) = self.peers_by_region.get(region) {
            region_gauge.inc();
        }
        
        tracing::info!(
            peer = hex::encode(peer_id),
            region = region,
            active_peers = self.active_peers.get(),
            "Peer connected"
        );
        
        *self.last_update.lock().unwrap() = Instant::now();
    }

    pub fn record_disconnection(&self, peer_id: &[u8], region: &str) {
        self.active_peers.dec();
        self.disconnections.inc();
        
        if let Some(region_gauge) = self.peers_by_region.get(region) {
            region_gauge.dec();
        }
        
        tracing::info!(
            peer = hex::encode(peer_id),
            region = region,
            active_peers = self.active_peers.get(),
            "Peer disconnected"
        );
        
        *self.last_update.lock().unwrap() = Instant::now();
    }

    pub fn record_message(&self, size: usize, is_outbound: bool) {
        if is_outbound {
            self.messages_sent.inc();
        } else {
            self.messages_received.inc();
        }
        self.message_sizes.observe(size as f64);
        *self.last_update.lock().unwrap() = Instant::now();
    }

    pub async fn run_health_check(&self) {
        let check_interval = Duration::from_secs(60);
        let mut interval = tokio::time::interval(check_interval);

        loop {
            interval.tick().await;
            
            let now = Instant::now();
            let last_update = *self.last_update.lock().unwrap();
            
            // Alert if no updates in 5 minutes
            if now.duration_since(last_update) > Duration::from_secs(300) {
                tracing::warn!(
                    seconds_since_update = now.duration_since(last_update).as_secs(),
                    "Network monitor hasn't received updates recently"
                );
            }

            // Log current state
            tracing::info!(
                active_peers = self.active_peers.get(),
                total_connections = self.total_connections.get(),
                messages_sent = self.messages_sent.get(),
                messages_received = self.messages_received.get(),
                "Network health check"
            );
        }
    }
}