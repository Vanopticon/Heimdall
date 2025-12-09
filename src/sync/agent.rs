use anyhow::{Context, Result};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerName};
use tokio_rustls::TlsConnector;

use crate::sync::auth::OidcProvider;

/// Maximum size for a single change log entry (10MB)
const MAX_ENTRY_SIZE: usize = 10 * 1024 * 1024;

/// Global sync metrics instance
static GLOBAL_SYNC_METRICS: once_cell::sync::Lazy<SyncMetrics> =
	once_cell::sync::Lazy::new(|| SyncMetrics::default());

/// Get a reference to the global sync metrics
pub fn global_sync_metrics() -> &'static SyncMetrics {
	&GLOBAL_SYNC_METRICS
}

/// Change log entry representing a single write operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeLogEntry {
	/// Unique identifier for this change
	pub id: String,
	/// Timestamp when the change occurred (Unix epoch seconds)
	pub timestamp: u64,
	/// Node label (e.g., "FieldValue", "Entity")
	pub label: String,
	/// Entity key for MERGE operations
	pub key: String,
	/// Properties to set on the node
	pub props: serde_json::Value,
	/// Origin node identifier
	pub origin: String,
	/// Version vector for conflict detection (origin -> version)
	pub version_vector: std::collections::HashMap<String, u64>,
	/// Tombstone flag for deletions
	pub tombstone: bool,
}

/// Sync protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncMessage {
	/// Authenticate with an OIDC token
	Auth { token: String },
	/// Authentication successful
	AuthOk,
	/// Authentication failed
	AuthFailed { reason: String },
	/// Push a batch of change log entries
	Push { entries: Vec<ChangeLogEntry> },
	/// Acknowledge receipt of push
	PushAck { count: usize },
	/// Pull change logs since a given timestamp
	Pull { since_timestamp: u64 },
	/// Response to pull with change log entries
	PullResponse { entries: Vec<ChangeLogEntry> },
	/// Heartbeat to keep connection alive
	Ping,
	/// Heartbeat response
	Pong,
	/// Error message
	Error { message: String },
}

/// Metrics for sync operations
pub struct SyncMetrics {
	pub push_attempts: AtomicU64,
	pub push_successes: AtomicU64,
	pub push_failures: AtomicU64,
	pub pull_attempts: AtomicU64,
	pub pull_successes: AtomicU64,
	pub pull_failures: AtomicU64,
	pub entries_sent: AtomicU64,
	pub entries_received: AtomicU64,
	pub reconnections: AtomicU64,
	pub auth_failures: AtomicU64,
}

impl Default for SyncMetrics {
	fn default() -> Self {
		Self {
			push_attempts: AtomicU64::new(0),
			push_successes: AtomicU64::new(0),
			push_failures: AtomicU64::new(0),
			pull_attempts: AtomicU64::new(0),
			pull_successes: AtomicU64::new(0),
			pull_failures: AtomicU64::new(0),
			entries_sent: AtomicU64::new(0),
			entries_received: AtomicU64::new(0),
			reconnections: AtomicU64::new(0),
			auth_failures: AtomicU64::new(0),
		}
	}
}

impl SyncMetrics {
	/// Generate Prometheus-compatible metrics text
	pub fn to_prometheus_text(&self) -> String {
		let mut out = String::new();

		out.push_str("# HELP heimdall_sync_push_attempts_total Total sync push attempts\n");
		out.push_str("# TYPE heimdall_sync_push_attempts_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_push_attempts_total {}\n",
			self.push_attempts.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_push_successes_total Successful sync pushes\n");
		out.push_str("# TYPE heimdall_sync_push_successes_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_push_successes_total {}\n",
			self.push_successes.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_push_failures_total Failed sync pushes\n");
		out.push_str("# TYPE heimdall_sync_push_failures_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_push_failures_total {}\n",
			self.push_failures.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_pull_attempts_total Total sync pull attempts\n");
		out.push_str("# TYPE heimdall_sync_pull_attempts_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_pull_attempts_total {}\n",
			self.pull_attempts.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_pull_successes_total Successful sync pulls\n");
		out.push_str("# TYPE heimdall_sync_pull_successes_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_pull_successes_total {}\n",
			self.pull_successes.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_pull_failures_total Failed sync pulls\n");
		out.push_str("# TYPE heimdall_sync_pull_failures_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_pull_failures_total {}\n",
			self.pull_failures.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_entries_sent_total Total change log entries sent\n");
		out.push_str("# TYPE heimdall_sync_entries_sent_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_entries_sent_total {}\n",
			self.entries_sent.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_entries_received_total Total change log entries received\n");
		out.push_str("# TYPE heimdall_sync_entries_received_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_entries_received_total {}\n",
			self.entries_received.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_reconnections_total Total reconnection attempts\n");
		out.push_str("# TYPE heimdall_sync_reconnections_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_reconnections_total {}\n",
			self.reconnections.load(Ordering::Relaxed)
		));

		out.push_str("# HELP heimdall_sync_auth_failures_total Total authentication failures\n");
		out.push_str("# TYPE heimdall_sync_auth_failures_total counter\n");
		out.push_str(&format!(
			"heimdall_sync_auth_failures_total {}\n",
			self.auth_failures.load(Ordering::Relaxed)
		));

		out
	}
}

/// Configuration for a sync peer
#[derive(Debug, Clone)]
pub struct PeerConfig {
	/// Peer hostname
	pub host: String,
	/// Peer port
	pub port: u16,
	/// SNI hostname for TLS verification
	pub sni_hostname: String,
	/// Sync interval in seconds
	pub sync_interval_secs: u64,
}

/// Sync agent for push/pull replication over TLS
pub struct SyncAgent {
	/// This node's identifier
	node_id: String,
	/// OIDC provider for authentication
	oidc_provider: Arc<OidcProvider>,
	/// Peer configurations
	peers: Vec<PeerConfig>,
	/// Metrics
	metrics: Arc<SyncMetrics>,
	/// TLS connector
	tls_connector: TlsConnector,
	/// Pending change log entries to push
	pending_entries: Arc<RwLock<Vec<ChangeLogEntry>>>,
	/// Last pull timestamp per peer
	last_pull_timestamps: Arc<RwLock<std::collections::HashMap<String, u64>>>,
}

impl SyncAgent {
	/// Create a new sync agent
	pub fn new(
		node_id: String,
		oidc_provider: Arc<OidcProvider>,
		peers: Vec<PeerConfig>,
	) -> Result<Self> {
		// Build TLS client config with system root certs
		let mut root_store = RootCertStore::empty();
		let certs = rustls_native_certs::load_native_certs()
			.context("failed to load native root certificates")?;

		let mut valid_certs = 0;
		for cert in certs {
			match root_store.add(&tokio_rustls::rustls::Certificate(cert.to_vec())) {
				Ok(_) => valid_certs += 1,
				Err(e) => {
					// Log individual certificate errors but continue loading others
					debug!("Skipping invalid certificate from native store: {:?}", e);
				}
			}
		}

		if valid_certs == 0 {
			anyhow::bail!("no valid root certificates found in native store");
		}

		debug!("Loaded {} valid root certificates", valid_certs);

		let client_config = ClientConfig::builder()
			.with_safe_default_cipher_suites()
			.with_safe_default_kx_groups()
			.with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
			.context("failed to configure TLS protocol versions")?
			.with_root_certificates(root_store)
			.with_no_client_auth();

		let tls_connector = TlsConnector::from(Arc::new(client_config));

		Ok(Self {
			node_id,
			oidc_provider,
			peers,
			metrics: Arc::new(SyncMetrics::default()),
			tls_connector,
			pending_entries: Arc::new(RwLock::new(Vec::new())),
			last_pull_timestamps: Arc::new(RwLock::new(std::collections::HashMap::new())),
		})
	}

	/// Add a change log entry to the pending queue
	pub async fn enqueue_change(&self, entry: ChangeLogEntry) {
		let mut entries = self.pending_entries.write().await;
		entries.push(entry);
		debug!("Enqueued change log entry, queue size: {}", entries.len());
	}

	/// Get the metrics for this sync agent
	pub fn metrics(&self) -> Arc<SyncMetrics> {
		Arc::clone(&self.metrics)
	}

	/// Start the sync agent background tasks
	pub async fn start(self: Arc<Self>) {
		info!("Starting sync agent for node: {}", self.node_id);

		for peer in self.peers.clone() {
			let agent = Arc::clone(&self);
			tokio::spawn(async move {
				agent.sync_loop(peer).await;
			});
		}
	}

	/// Main sync loop for a peer
	async fn sync_loop(&self, peer: PeerConfig) {
		let peer_addr = format!("{}:{}", peer.host, peer.port);
		let mut tick = interval(Duration::from_secs(peer.sync_interval_secs));

		loop {
			tick.tick().await;

			debug!("Starting sync cycle with peer: {}", peer_addr);

			match self.sync_with_peer(&peer).await {
				Ok(_) => {
					debug!("Sync cycle completed successfully with peer: {}", peer_addr);
				}
				Err(e) => {
					error!("Sync cycle failed with peer {}: {}", peer_addr, e);
					self.metrics.reconnections.fetch_add(1, Ordering::Relaxed);
					// Exponential backoff with jitter to avoid thundering herd
					let jitter = (std::time::SystemTime::now()
						.duration_since(std::time::UNIX_EPOCH)
						.unwrap()
						.as_millis() % 5000) as u64 / 1000;
					let backoff_secs = 5 + jitter;
					sleep(Duration::from_secs(backoff_secs)).await;
				}
			}
		}
	}

	/// Perform a sync operation with a peer
	async fn sync_with_peer(&self, peer: &PeerConfig) -> Result<()> {
		// Connect to peer over TLS
		let stream = self.connect_tls(peer).await?;
		let (mut reader, mut writer) = tokio::io::split(stream);

		// Authenticate with OIDC token
		self.authenticate(&mut reader, &mut writer).await?;

		// Push pending changes
		self.push_changes(&mut reader, &mut writer).await?;

		// Pull remote changes
		self.pull_changes(&mut reader, &mut writer, peer).await?;

		Ok(())
	}

	/// Establish a TLS connection to a peer
	async fn connect_tls(
		&self,
		peer: &PeerConfig,
	) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
		let addr = format!("{}:{}", peer.host, peer.port);
		debug!("Connecting to peer: {}", addr);

		let tcp_stream = TcpStream::connect(&addr)
			.await
			.context("failed to connect to peer")?;

		let server_name = ServerName::try_from(peer.sni_hostname.as_str())
			.context("invalid SNI hostname")?;

		let tls_stream = self
			.tls_connector
			.connect(server_name, tcp_stream)
			.await
			.context("TLS handshake failed")?;

		info!("TLS connection established with peer: {}", addr);
		Ok(tls_stream)
	}

	/// Authenticate with a peer using OIDC token
	async fn authenticate<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin>(
		&self,
		reader: &mut R,
		writer: &mut W,
	) -> Result<()> {
		debug!("Authenticating with peer using OIDC");

		// Obtain client credentials token
		let token = self
			.oidc_provider
			.get_client_credentials_token(Some("sync"))
			.await
			.context("failed to obtain OIDC token")?;

		// Send auth message
		let auth_msg = SyncMessage::Auth { token };
		self.send_message(writer, &auth_msg).await?;

		// Wait for auth response
		let response = self.receive_message(reader).await?;

		match response {
			SyncMessage::AuthOk => {
				info!("Authentication successful");
				Ok(())
			}
			SyncMessage::AuthFailed { reason } => {
				self.metrics.auth_failures.fetch_add(1, Ordering::Relaxed);
				anyhow::bail!("authentication failed: {}", reason)
			}
			_ => {
				self.metrics.auth_failures.fetch_add(1, Ordering::Relaxed);
				anyhow::bail!("unexpected response to auth: {:?}", response)
			}
		}
	}

	/// Push pending changes to a peer
	async fn push_changes<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin>(
		&self,
		reader: &mut R,
		writer: &mut W,
	) -> Result<()> {
		self.metrics.push_attempts.fetch_add(1, Ordering::Relaxed);

		let mut pending = self.pending_entries.write().await;
		if pending.is_empty() {
			debug!("No pending changes to push");
			return Ok(());
		}

		let entries_to_push = pending.clone();
		let count = entries_to_push.len();

		debug!("Pushing {} change log entries", count);

		let push_msg = SyncMessage::Push {
			entries: entries_to_push,
		};

		self.send_message(writer, &push_msg).await?;

		// Wait for acknowledgment
		let response = self.receive_message(reader).await?;

		match response {
			SyncMessage::PushAck { count: ack_count } => {
				if ack_count == count {
					info!("Push acknowledged: {} entries", ack_count);
					pending.clear();
					self.metrics.push_successes.fetch_add(1, Ordering::Relaxed);
					self.metrics.entries_sent.fetch_add(ack_count as u64, Ordering::Relaxed);
					Ok(())
				} else {
					self.metrics.push_failures.fetch_add(1, Ordering::Relaxed);
					anyhow::bail!("push ack count mismatch: expected {}, got {}", count, ack_count)
				}
			}
			SyncMessage::Error { message } => {
				self.metrics.push_failures.fetch_add(1, Ordering::Relaxed);
				anyhow::bail!("push failed: {}", message)
			}
			_ => {
				self.metrics.push_failures.fetch_add(1, Ordering::Relaxed);
				anyhow::bail!("unexpected response to push: {:?}", response)
			}
		}
	}

	/// Pull changes from a peer
	async fn pull_changes<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin>(
		&self,
		reader: &mut R,
		writer: &mut W,
		peer: &PeerConfig,
	) -> Result<()> {
		self.metrics.pull_attempts.fetch_add(1, Ordering::Relaxed);

		let peer_id = format!("{}:{}", peer.host, peer.port);
		let timestamps = self.last_pull_timestamps.read().await;
		let since_timestamp = timestamps.get(&peer_id).copied().unwrap_or(0);
		drop(timestamps);

		debug!("Pulling changes since timestamp: {}", since_timestamp);

		let pull_msg = SyncMessage::Pull { since_timestamp };
		self.send_message(writer, &pull_msg).await?;

		// Wait for pull response
		let response = self.receive_message(reader).await?;

		match response {
			SyncMessage::PullResponse { entries } => {
				let count = entries.len();
				info!("Received {} change log entries from peer", count);

				// Update last pull timestamp
				if let Some(last_entry) = entries.last() {
					let mut timestamps = self.last_pull_timestamps.write().await;
					timestamps.insert(peer_id, last_entry.timestamp);
				}

				self.metrics.pull_successes.fetch_add(1, Ordering::Relaxed);
				self.metrics.entries_received.fetch_add(count as u64, Ordering::Relaxed);

				// Process received entries (in a real implementation, this would
				// apply merge rules and update the local database)
				debug!("Processing {} received entries", count);

				Ok(())
			}
			SyncMessage::Error { message } => {
				self.metrics.pull_failures.fetch_add(1, Ordering::Relaxed);
				anyhow::bail!("pull failed: {}", message)
			}
			_ => {
				self.metrics.pull_failures.fetch_add(1, Ordering::Relaxed);
				anyhow::bail!("unexpected response to pull: {:?}", response)
			}
		}
	}

	/// Send a sync message over the wire
	async fn send_message<W: AsyncWriteExt + Unpin>(
		&self,
		writer: &mut W,
		msg: &SyncMessage,
	) -> Result<()> {
		let json = serde_json::to_vec(msg).context("failed to serialize message")?;
		let len = json.len();

		if len > MAX_ENTRY_SIZE {
			anyhow::bail!("message size {} exceeds maximum {}", len, MAX_ENTRY_SIZE);
		}

		// Write length prefix (4 bytes, big-endian)
		writer
			.write_all(&(len as u32).to_be_bytes())
			.await
			.context("failed to write message length")?;

		// Write message body
		writer
			.write_all(&json)
			.await
			.context("failed to write message body")?;

		writer.flush().await.context("failed to flush writer")?;

		Ok(())
	}

	/// Receive a sync message from the wire
	async fn receive_message<R: AsyncReadExt + Unpin>(&self, reader: &mut R) -> Result<SyncMessage> {
		// Read length prefix (4 bytes, big-endian)
		let mut len_bytes = [0u8; 4];
		reader
			.read_exact(&mut len_bytes)
			.await
			.context("failed to read message length")?;

		let len = u32::from_be_bytes(len_bytes) as usize;

		if len > MAX_ENTRY_SIZE {
			anyhow::bail!("message size {} exceeds maximum {}", len, MAX_ENTRY_SIZE);
		}

		// Read message body
		let mut buf = vec![0u8; len];
		reader
			.read_exact(&mut buf)
			.await
			.context("failed to read message body")?;

		let msg: SyncMessage =
			serde_json::from_slice(&buf).context("failed to deserialize message")?;

		Ok(msg)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_sync_message_serialization() {
		let msg = SyncMessage::Ping;
		let json = serde_json::to_string(&msg).unwrap();
		assert!(json.contains("Ping"));

		let deserialized: SyncMessage = serde_json::from_str(&json).unwrap();
		assert!(matches!(deserialized, SyncMessage::Ping));
	}

	#[test]
	fn test_change_log_entry_serialization() {
		let mut version_vector = std::collections::HashMap::new();
		version_vector.insert("node1".to_string(), 1);

		let entry = ChangeLogEntry {
			id: "test-id".to_string(),
			timestamp: 1234567890,
			label: "TestLabel".to_string(),
			key: "test-key".to_string(),
			props: serde_json::json!({"field": "value"}),
			origin: "node1".to_string(),
			version_vector,
			tombstone: false,
		};

		let json = serde_json::to_string(&entry).unwrap();
		assert!(json.contains("test-id"));
		assert!(json.contains("TestLabel"));

		let deserialized: ChangeLogEntry = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized.id, "test-id");
		assert_eq!(deserialized.tombstone, false);
	}

	#[test]
	fn test_sync_metrics_default() {
		let metrics = SyncMetrics::default();
		assert_eq!(metrics.push_attempts.load(Ordering::Relaxed), 0);
		assert_eq!(metrics.entries_sent.load(Ordering::Relaxed), 0);
	}

	#[test]
	fn test_sync_metrics_prometheus_text() {
		let metrics = SyncMetrics::default();
		metrics.push_attempts.store(5, Ordering::Relaxed);
		metrics.entries_sent.store(100, Ordering::Relaxed);

		let text = metrics.to_prometheus_text();
		assert!(text.contains("heimdall_sync_push_attempts_total 5"));
		assert!(text.contains("heimdall_sync_entries_sent_total 100"));
	}
}
