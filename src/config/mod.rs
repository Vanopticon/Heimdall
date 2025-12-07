use anyhow::Result;
use serde::Deserialize;

/// Runtime configuration for Heimdall.
///
/// Values are loaded from (in order): `config` file (optional) and environment variables
/// prefixed with `HMD_` (e.g. `HMD_PORT`). This is a small, intentionally conservative
/// bootstrap for the project's configuration system.
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct Settings {
	pub host: String,
	pub port: u16,
	pub database_url: Option<String>,
	pub tls_cert: Option<String>,
	pub tls_key: Option<String>,
	pub log_level: Option<String>,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			host: "127.0.0.1".to_string(),
			port: 443,
			database_url: None,
			tls_cert: None,
			tls_key: None,
			log_level: Some("info".to_string()),
		}
	}
}

/// Partial settings used to overlay environment/file values on top of defaults.
#[derive(Debug, Deserialize)]
struct PartialSettings {
	host: Option<String>,
	port: Option<u16>,
	database_url: Option<String>,
	tls_cert: Option<String>,
	tls_key: Option<String>,
	log_level: Option<String>,
}

/// Load settings from config file (optional) and environment variables.
pub fn load() -> Result<Settings> {
	let builder = config::Config::builder()
		.add_source(config::File::with_name("config").required(false))
		// Use a double-underscore separator so single-underscore env names like
		// `HMD_DATABASE_URL` map to `database_url` instead of nested `database.url`.
		.add_source(config::Environment::with_prefix("HMD").separator("__"));

	let cfg = builder.build()?;

	let partial: PartialSettings = cfg.try_deserialize()?;

	let mut s = Settings::default();
	if let Some(host) = partial.host {
		s.host = host;
	}
	if let Some(port) = partial.port {
		s.port = port;
	}
	if let Some(db) = partial.database_url {
		s.database_url = Some(db);
	}
	if let Some(cert) = partial.tls_cert {
		s.tls_cert = Some(cert);
	}
	if let Some(key) = partial.tls_key {
		s.tls_key = Some(key);
	}
	if let Some(level) = partial.log_level {
		s.log_level = Some(level);
	}
	// Explicitly prefer direct environment variables when present. Some
	// environments (CI, test harnesses) may set env vars in ways that the
	// `config` crate doesn't map as expected; read them directly to ensure
	// explicit overrides take effect.
	if let Ok(h) = std::env::var("HMD_HOST") {
		if !h.is_empty() {
			s.host = h;
		}
	}
	if let Ok(p) = std::env::var("HMD_PORT") {
		if let Ok(pn) = p.parse::<u16>() {
			s.port = pn;
		}
	}
	if let Ok(db) = std::env::var("HMD_DATABASE_URL") {
		if !db.is_empty() {
			s.database_url = Some(db);
		}
	}
	if let Ok(c) = std::env::var("HMD_TLS_CERT") {
		if !c.is_empty() {
			s.tls_cert = Some(c);
		}
	}
	if let Ok(k) = std::env::var("HMD_TLS_KEY") {
		if !k.is_empty() {
			s.tls_key = Some(k);
		}
	}
	if let Ok(l) = std::env::var("HMD_LOG_LEVEL") {
		if !l.is_empty() {
			s.log_level = Some(l);
		}
	}

	Ok(s)
}

#[cfg(feature = "unit-tests")]
mod tests {
	use super::*;
	use std::env;

	#[test]
	fn test_load_defaults_and_env_overlay() {
		// Save original values so we can restore them
		let orig_host = env::var_os("HMD_HOST");
		let orig_port = env::var_os("HMD_PORT");
		let orig_db = env::var_os("HMD_DATABASE_URL");
		let orig_cert = env::var_os("HMD_TLS_CERT");
		let orig_key = env::var_os("HMD_TLS_KEY");
		let orig_level = env::var_os("HMD_LOG_LEVEL");

		// Ensure environment is clean for the defaults check
		unsafe { env::remove_var("HMD_HOST") };
		unsafe { env::remove_var("HMD_PORT") };
		unsafe { env::remove_var("HMD_DATABASE_URL") };
		unsafe { env::remove_var("HMD_TLS_CERT") };
		unsafe { env::remove_var("HMD_TLS_KEY") };
		unsafe { env::remove_var("HMD_LOG_LEVEL") };

		let s = load().expect("load should succeed with defaults");
		let d = Settings::default();
		assert_eq!(s.host, d.host);
		assert_eq!(s.port, d.port);
		assert_eq!(s.log_level, d.log_level);

		// Overlay environment values and verify they take effect
		unsafe { env::set_var("HMD_HOST", "0.0.0.0") };
		unsafe { env::set_var("HMD_PORT", "8080") };
		unsafe { env::set_var("HMD_DATABASE_URL", "postgres://user:pass@localhost/db") };
		unsafe { env::set_var("HMD_TLS_CERT", "/tmp/cert.pem") };
		unsafe { env::set_var("HMD_TLS_KEY", "/tmp/key.pem") };
		unsafe { env::set_var("HMD_LOG_LEVEL", "debug") };

		let s2 = load().expect("load should succeed with env");
		assert_eq!(s2.host, "0.0.0.0");
		assert_eq!(s2.port, 8080u16);
		assert_eq!(
			s2.database_url.as_deref(),
			Some("postgres://user:pass@localhost/db")
		);
		assert_eq!(s2.tls_cert.as_deref(), Some("/tmp/cert.pem"));
		assert_eq!(s2.tls_key.as_deref(), Some("/tmp/key.pem"));
		assert_eq!(s2.log_level.as_deref(), Some("debug"));

		// restore originals
		match orig_host {
			Some(v) => unsafe { env::set_var("HMD_HOST", v) },
			None => unsafe { env::remove_var("HMD_HOST") },
		}
		match orig_port {
			Some(v) => unsafe { env::set_var("HMD_PORT", v) },
			None => unsafe { env::remove_var("HMD_PORT") },
		}
		match orig_db {
			Some(v) => unsafe { env::set_var("HMD_DATABASE_URL", v) },
			None => unsafe { env::remove_var("HMD_DATABASE_URL") },
		}
		match orig_cert {
			Some(v) => unsafe { env::set_var("HMD_TLS_CERT", v) },
			None => unsafe { env::remove_var("HMD_TLS_CERT") },
		}
		match orig_key {
			Some(v) => unsafe { env::set_var("HMD_TLS_KEY", v) },
			None => unsafe { env::remove_var("HMD_TLS_KEY") },
		}
		match orig_level {
			Some(v) => unsafe { env::set_var("HMD_LOG_LEVEL", v) },
			None => unsafe { env::remove_var("HMD_LOG_LEVEL") },
		}
	}
}
