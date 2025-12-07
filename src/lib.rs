pub mod age_client;
pub mod config;
pub mod devops;
pub mod ingest;

pub async fn run() {
	// Start a minimal dev HTTP server exposing the ingest endpoints so
	// integration tests and local development can exercise the handlers.
	use axum::{Router, routing::post};
	use rustls::{Certificate, PrivateKey, ServerConfig};
	use rustls_pemfile as pemfile;
	use std::sync::Arc;

	let settings = match crate::config::load() {
		Ok(s) => s,
		Err(e) => {
			eprintln!("failed to load config, using defaults: {}", e);
			crate::config::Settings::default()
		}
	};

	let addr = format!("{}:{}", settings.host, settings.port);
	let _app: axum::Router<()> = Router::new()
		.route(
			"/ingest/ndjson",
			post(crate::ingest::handler::ndjson_upload),
		)
		.route(
			"/ingest/bulk",
			post(crate::ingest::handler::bulk_dump_upload),
		)
		.route("/health", post(|| async { "ok" }));

	// For now we only construct the app and perform a minimal TLS sanity
	// check. Heimdall refuses to run without a valid TLS certificate and
	// key (self-signed certs are rejected in dev). This keeps local runs
	// honest and avoids defaulting to plaintext HTTP.

	// `Settings` contains concrete String paths for cert/key (defaults set in `Default`).
	let cert_path = settings.tls_cert.as_str();
	let key_path = settings.tls_key.as_str();

	if !std::path::Path::new(cert_path).exists() || !std::path::Path::new(key_path).exists() {
		eprintln!(
			"TLS certificate or key not found at '{}' and '{}'. Heimdall requires HTTPS with a valid TLS certificate (TLS 1.3 minimum). Refusing to start.",
			cert_path, key_path
		);
		std::process::exit(1);
	}

	// Strict TLS: parse cert and key using rustls-pemfile and construct a
	// rustls ServerConfig restricted to TLS1.3. This ensures the cert/key are
	// well-formed and that the server can be configured to use modern TLS only.
	let certs = match std::fs::File::open(cert_path) {
		Ok(f) => {
			let mut reader = std::io::BufReader::new(f);
			match pemfile::certs(&mut reader) {
				Ok(c) if !c.is_empty() => c.into_iter().map(Certificate).collect::<Vec<_>>(),
				_ => {
					eprintln!(
						"No certificates found in '{}'. Heimdall requires a PEM certificate chain.",
						cert_path
					);
					std::process::exit(1);
				}
			}
		}
		Err(e) => {
			eprintln!("Failed to open certificate '{}': {}", cert_path, e);
			std::process::exit(1);
		}
	};

	let key = match std::fs::File::open(key_path) {
		Ok(f) => {
			let mut reader = std::io::BufReader::new(f);
			let pkcs8 = pemfile::pkcs8_private_keys(&mut reader).unwrap_or_default();
			if !pkcs8.is_empty() {
				PrivateKey(pkcs8[0].clone())
			} else {
				// retry for RSA keys
				let mut reader = std::io::BufReader::new(std::fs::File::open(key_path).unwrap());
				let rsa = pemfile::rsa_private_keys(&mut reader).unwrap_or_default();
				if !rsa.is_empty() {
					PrivateKey(rsa[0].clone())
				} else {
					eprintln!(
						"No private key found in '{}'. Heimdall requires a PEM private key (PKCS8 or RSA).",
						key_path
					);
					std::process::exit(1);
				}
			}
		}
		Err(e) => {
			eprintln!("Failed to open private key '{}': {}", key_path, e);
			std::process::exit(1);
		}
	};

	// Basic cert checks: ensure the first certificate is not self-signed and
	// can be parsed as X.509. This catches common dev mistakes such as using
	// a generated self-signed cert without a proper CA chain.
	if let Some(first) = certs.get(0) {
		match x509_parser::parse_x509_certificate(&first.0) {
			Ok((_rem, cert)) => {
				if cert.tbs_certificate.subject == cert.tbs_certificate.issuer {
					eprintln!(
						"TLS certificate at '{}' appears to be self-signed; Heimdall will not start with self-signed certificates.",
						cert_path
					);
					std::process::exit(1);
				}
			}
			Err(e) => {
				eprintln!("Failed to parse certificate DER: {}", e);
				std::process::exit(1);
			}
		}
	}

	// Try to build a minimal ServerConfig to ensure the cert/key are usable.
	// Enforce TLS 1.3 only by explicitly selecting protocol versions with the
	// rustls ConfigBuilder API. The builder requires choosing cipher suites,
	// kx groups and protocol versions in that order. We use the safe defaults
	// for cipher suites and kx groups, then restrict protocol versions to
	// TLS1.3 explicitly. If any step fails, refuse to start.
	let server_config = match ServerConfig::builder()
		.with_safe_default_cipher_suites()
		.with_safe_default_kx_groups()
		.with_protocol_versions(&[&rustls::version::TLS13])
	{
		Ok(cb) => match cb.with_no_client_auth().with_single_cert(certs, key) {
			Ok(cfg) => Arc::new(cfg),
			Err(e) => {
				eprintln!("Failed to configure TLS server: {}", e);
				std::process::exit(1);
			}
		},
		Err(e) => {
			eprintln!("Failed to configure TLS protocol versions: {}", e);
			std::process::exit(1);
		}
	};

	println!(
		"Heimdall dev endpoints registered (not started): https://{} (POST /ingest/ndjson, /ingest/bulk)",
		addr
	);
}
