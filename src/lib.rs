pub mod age_client;
pub mod config;
pub mod devops;
pub mod enrich;
pub mod health;
pub mod ingest;
pub mod observability;
pub mod persist;
pub mod pii;
pub mod state;
pub mod sync;
pub mod tls_utils;

// Library modules
pub mod lib {
	pub mod normalizers;
}

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::http::header::{HeaderName, HeaderValue};
use axum::{
	Router,
	routing::{get, post},
};
use hyper_util::rt::TokioExecutor;
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::sensitive_headers::{
	SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::RequestBodyTimeoutLayer;
use tower_http::trace::TraceLayer;

/// Start a hardened dev HTTP server exposing the ingest endpoints.
///
/// This function intentionally logs errors rather than returning them so
/// the simple `main` runner can call it without changing its signature.
pub async fn run() {
	// Initialize observability: structured logging, metrics, and tracing
	let obs_state = match crate::observability::init_observability().await {
		Ok(s) => s,
		Err(e) => {
			eprintln!("warning: failed to initialize observability: {}", e);
			crate::observability::ObservabilityState::default()
		}
	};

	// Load settings (fall back to defaults on error)
	let settings = match crate::config::load() {
		Ok(s) => s,
		Err(e) => {
			eprintln!("warning: failed to load config: {}", e);
			crate::config::Settings::default()
		}
	};

	// Build the router with ingest endpoints
	let app = Router::new()
		.route("/ingest/ndjson", post(crate::ingest::ndjson_upload))
		.route("/ingest/bulk", post(crate::ingest::bulk_dump_upload))
		.route("/ingest/multipart", post(crate::ingest::multipart_upload))
		.route("/health", get(|| async { "OK" }))
		.route("/health/db", get(crate::health::db_health))
		.route("/metrics", get(|| async {
			let mut metrics = crate::persist::metrics_text();
			metrics.push_str(&crate::sync::global_sync_metrics().to_prometheus_text());
			metrics
		}))
		// Defense-in-depth: normalize paths and add conservative security headers
		.layer(TraceLayer::new_for_http())
		.layer(NormalizePathLayer::trim_trailing_slash())
		.layer(SetResponseHeaderLayer::if_not_present(
			HeaderName::from_static("strict-transport-security"),
			HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
		))
		.layer(SetResponseHeaderLayer::if_not_present(
			HeaderName::from_static("x-frame-options"),
			HeaderValue::from_static("DENY"),
		))
		.layer(SetResponseHeaderLayer::if_not_present(
			HeaderName::from_static("x-content-type-options"),
			HeaderValue::from_static("nosniff"),
		))
		.layer(SetResponseHeaderLayer::if_not_present(
			HeaderName::from_static("referrer-policy"),
			HeaderValue::from_static("strict-origin-when-cross-origin"),
		))
		.layer(SetResponseHeaderLayer::if_not_present(
			HeaderName::from_static("permissions-policy"),
			HeaderValue::from_static("geolocation=(), microphone=()"),
		));

	// Initialize mandatory persistence: connect to the configured database
	// and set the global AGE repository used by handlers. When running
	// inside a Docker-based dev environment the DB container may be slow to
	// become ready; poll and retry for a configurable number of attempts
	// (HMD_DB_CONNECT_RETRIES) with a backoff (HMD_DB_CONNECT_BACKOFF_MS).
	let max_retries: u32 = std::env::var("HMD_DB_CONNECT_RETRIES")
		.ok()
		.and_then(|s| s.parse::<u32>().ok())
		.unwrap_or(60);
	let backoff_ms: u64 = std::env::var("HMD_DB_CONNECT_BACKOFF_MS")
		.ok()
		.and_then(|s| s.parse::<u64>().ok())
		.unwrap_or(1000);

	let mut last_err: Option<anyhow::Error> = None;
	let mut client_opt: Option<crate::age_client::AgeClient> = None;
	for attempt in 1..=max_retries {
		match crate::age_client::AgeClient::connect(
			settings.database_url.as_str(),
			&settings.age_graph,
		)
		.await
		{
			Ok(c) => {
				client_opt = Some(c);
				break;
			}
			Err(e) => {
				eprintln!(
					"DB connect attempt {}/{} failed: {}",
					attempt, max_retries, e
				);
				last_err = Some(e);
				if attempt < max_retries {
					tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
				}
			}
		}
	}

	let client = match client_opt {
		Some(c) => c,
		None => {
			eprintln!(
				"failed to connect to DB for persistence after {} attempts: {}",
				max_retries,
				last_err
					.as_ref()
					.map(|e| e.to_string())
					.unwrap_or_else(|| "unknown error".to_string())
			);
			return;
		}
	};

	let repo: std::sync::Arc<dyn crate::age_client::AgeRepo> = std::sync::Arc::new(client);

	// Inject the shared repo into application state and attach it to the
	// router so handlers can access it via Axum's `State` extractor.
	// Start the background persistence batcher and attach the sender to the
	// application state so handlers can enqueue jobs without blocking.
	let persist_capacity: usize = std::env::var("HMD_PERSIST_CHANNEL_CAPACITY")
		.ok()
		.and_then(|s| s.parse::<usize>().ok())
		.unwrap_or(10_000);
	let persist_batch_size: usize = std::env::var("HMD_PERSIST_BATCH_SIZE")
		.ok()
		.and_then(|s| s.parse::<usize>().ok())
		.unwrap_or(100);
	let persist_flush_ms: u64 = std::env::var("HMD_PERSIST_FLUSH_MS")
		.ok()
		.and_then(|s| s.parse::<u64>().ok())
		.unwrap_or(1000);

	let sender = crate::persist::start_batcher(
		repo.clone(),
		obs_state.metrics.clone(),
		persist_capacity,
		persist_batch_size,
		persist_flush_ms,
	);

	// Initialize PII policy engine if master key is configured
	let pii_engine = if let Some(key_hex) = &settings.pii_master_key {
		match crate::pii::pii_policy::PiiPolicyEngine::parse_master_key_hex(key_hex) {
			Ok(key) => {
				// Create a default policy config (can be extended to load from file)
				let config = crate::pii::pii_policy::PiiPolicyConfig::default();
				match crate::pii::pii_policy::PiiPolicyEngine::new(
					config,
					key,
					"default-key-v1".to_string(),
				) {
					Ok(engine) => {
						eprintln!("PII policy engine initialized");
						Some(Arc::new(engine))
					}
					Err(e) => {
						eprintln!("warning: failed to create PII engine: {}", e);
						None
					}
				}
			}
			Err(e) => {
				eprintln!("warning: failed to parse PII master key: {}", e);
				None
			}
		}
	} else {
		None
	};

	let app_state = crate::state::AppState {
		repo: repo.clone(),
		persist_sender: sender,
		metrics: obs_state.metrics.clone(),
	};
	let app = app.with_state(app_state);

	// Load TLS material
	let certs = match tls_utils::load_certs(Path::new(&settings.tls_cert)) {
		Ok(c) => c,
		Err(e) => {
			eprintln!("failed to load TLS certs ({}). Serving disabled.", e);
			return;
		}
	};
	let key = match tls_utils::load_private_key(Path::new(&settings.tls_key)) {
		Ok(k) => k,
		Err(e) => {
			eprintln!("failed to load TLS private key ({}). Serving disabled.", e);
			return;
		}
	};

	// Basic X.509 policy checks on the leaf certificate.
	if certs.is_empty() {
		eprintln!("no TLS certificates loaded; serving disabled");
		return;
	}

	let leaf = &certs[0];
	// Expiry check
	match tls_utils::is_cert_expired(leaf) {
		Ok(true) => {
			eprintln!("TLS certificate appears to be expired; serving disabled");
			return;
		}
		Err(e) => {
			eprintln!(
				"failed to evaluate TLS certificate expiry ({}); serving disabled",
				e
			);
			return;
		}
		_ => {}
	}

	// Hostname (CN / SAN) check against configured host
	let host_to_check = settings.host.clone();
	if !host_to_check.is_empty() {
		let mut matched = false;
		if let Ok(sans) = tls_utils::dns_names_from_cert(leaf) {
			if sans.iter().any(|s| s == &host_to_check) {
				matched = true;
			}
		}
		if !matched {
			if let Ok(Some(cn)) = tls_utils::first_common_name(leaf) {
				if cn == host_to_check {
					matched = true;
				}
			}
		}
		if !matched {
			eprintln!(
				"TLS certificate does not contain configured host '{}' in CN or SAN; serving disabled",
				host_to_check
			);
			return;
		}
	}

	let server_cfg = match tls_utils::build_server_config_tls13(certs, key) {
		Ok(cfg) => cfg,
		Err(e) => {
			eprintln!("failed to build TLS server config: {}", e);
			return;
		}
	};

	let acceptor = TlsAcceptor::from(server_cfg);

	let bind_addr: SocketAddr = match format!("{}:{}", settings.host, settings.port).parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("invalid listen address: {}", e);
			return;
		}
	};

	// Bind TCP listener
	let listener = match TcpListener::bind(bind_addr).await {
		Ok(l) => l,
		Err(e) => {
			eprintln!("failed to bind {}: {}", bind_addr, e);
			return;
		}
	};

	println!(
		"Heimdall dev endpoints registered and hardened: https://{} (POST /ingest/ndjson, /ingest/bulk)",
		bind_addr
	);

	// Accept loop: perform TLS handshake and spawn a per-connection task that
	// serves requests using hyper's connection serving utilities.
	loop {
		let (tcp_stream, peer_addr) = match listener.accept().await {
			Ok(t) => t,
			Err(e) => {
				eprintln!("accept error: {}", e);
				tokio::time::sleep(Duration::from_millis(100)).await;
				continue;
			}
		};

		let acceptor = acceptor.clone();
		let app = app.clone();
		let settings = settings.clone();

		tokio::spawn(async move {
			let _ = tcp_stream.set_nodelay(true);

			let tls_stream = match acceptor.accept(tcp_stream).await {
				Ok(s) => s,
				Err(e) => {
					eprintln!("TLS handshake failed ({}): {}", peer_addr, e);
					return;
				}
			};

			// Mark commonly-sensitive headers so downstream logging and
			// middleware don't accidentally expose secrets. tower-http's
			// sensitive header helpers expect an Arc<[HeaderName]>.
			let req_headers: Arc<[HeaderName]> = Arc::from(
				vec![
					HeaderName::from_static("authorization"),
					HeaderName::from_static("cookie"),
				]
				.into_boxed_slice(),
			);

			let res_headers: Arc<[HeaderName]> =
				Arc::from(vec![HeaderName::from_static("set-cookie")].into_boxed_slice());

			// Build a defensive service stack for transport-level protections.
			let svc = ServiceBuilder::new()
				// Catch panics in handlers and return a safe response instead of
				// unwinding the thread/task.
				.layer(CatchPanicLayer::new())
				// Per-service concurrency limit to protect downstream resources
				// (small by default for dev; tune for production).
				.concurrency_limit(100)
				// Load-shed when the service is saturated instead of queueing.
				.load_shed()
				// Overall request timeout to avoid slowloris-like resource use.
				.timeout(Duration::from_secs(30))
				// (Optional) a simple rate limit was intentionally omitted
				// here because some in-process rate limiters carry internal
				// non-Clone state which interferes with the per-connection
				// serving adapter used below. Prefer a global rate limiter
				// upstream (proxy/load-balancer) or a shared rate limiter
				// implementation if you need in-process rate limits.
				// Limit request body sizes to avoid memory/CPU exhaustion.
				.layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MiB
				// Also enforce timeouts while reading the request body.
				.layer(RequestBodyTimeoutLayer::new(Duration::from_secs(30)))
				// Shared in-process rate limiter (Clone-friendly layer)
				.layer(crate::devops::SharedRateLimitLayer::new(
					settings.rate_limit_burst as usize,
					settings.rate_limit_rps,
				))
				// Mark sensitive headers on both requests and responses so
				// logging and tracing will avoid printing them.
				.layer(SetSensitiveRequestHeadersLayer::from_shared(req_headers.clone()))
				.layer(SetSensitiveResponseHeadersLayer::from_shared(res_headers.clone()))
				.service(app.into_service());

			// Convert tower/axum service into a hyper-compatible service
			let hyper_svc = TowerToHyperService::new(svc);

			// Adapt the tokio-based TLS stream into a sync IO object that
			// hyper_util's auto builder understands, then serve the connection.
			let io = TokioIo::new(tls_stream);
			let builder = AutoBuilder::new(TokioExecutor::new());
			let conn = builder.serve_connection(io, hyper_svc);

			if let Err(err) = conn.await {
				eprintln!("connection error ({}): {}", peer_addr, err);
			}
		});
	}
}
