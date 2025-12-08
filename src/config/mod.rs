use hostname;
use log::Level;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

/// Runtime configuration for Heimdall.
///
/// Values are loaded from (in order): a config file - in the `/etc/vanopticon/heimdall.json` file,
/// and in the user config folder (optional), and environment variables
/// prefixed with `HMD_` (e.g. `HMD_PORT`). This is a small, intentionally conservative
/// bootstrap for the project's configuration system.
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct Settings {
	pub host: String,
	pub port: u16,
	pub database_url: Url,
	pub tls_cert: String,
	pub tls_key: String,
	pub log_level: Level,
	// Rate limiting: requests-per-second and burst size (tokens)
	pub rate_limit_rps: u32,
	pub rate_limit_burst: u32,
	// AGE graph name to use when persisting
	pub age_graph: String,
}

impl Default for Settings {
	fn default() -> Self {
		let host = hostname::get()
			.ok()
			.and_then(|s| s.into_string().ok())
			.unwrap_or_else(|| "127.0.0.1".to_string());

		Self {
			host,
			port: 443,
			database_url: Url::parse("postgresql://heimdall:heimdall@rainbowbridge/heimdall1")
				.unwrap(),
			tls_cert: "/etc/tls/tls.crt".to_string(),
			tls_key: "/etc/tls/tls.key".to_string(),
			log_level: Level::Info,
			// sensible defaults for dev: 10 RPS refill, burst up to 100
			rate_limit_rps: 10,
			rate_limit_burst: 100,
			age_graph: "heimdall_graph".to_string(),
		}
	}
}

#[derive(Debug, Error)]
pub enum SettingsError {
	#[error("configuration error: {0}")]
	Config(#[from] config::ConfigError),
}

pub fn load() -> Result<Settings, SettingsError> {
	let mut builder = config::Config::builder()
		.add_source(config::File::with_name("/etc/vanopticon/heimdall.json").required(false));

	if let Some(folder) = dirs::config_dir() {
		let user_config_path = folder.join("vanopticon").join("heimdall.json");
		builder = builder.add_source(config::File::from(user_config_path).required(false));
	}
	if let Some(folder) = dirs::config_local_dir() {
		let local_config_path = folder.join("vanopticon").join("heimdall.json");
		builder = builder.add_source(config::File::from(local_config_path).required(false));
	}

	builder = builder.add_source(config::Environment::with_prefix("HMD").separator("__"));

	let cfg = builder.build()?;

	let mut s: Settings = cfg.try_deserialize()?;

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
			if let Ok(parsed) = Url::parse(&db) {
				s.database_url = parsed;
			}
		}
	}
	if let Ok(c) = std::env::var("HMD_TLS_CERT") {
		if !c.is_empty() {
			s.tls_cert = c;
		}
	}
	if let Ok(k) = std::env::var("HMD_TLS_KEY") {
		if !k.is_empty() {
			s.tls_key = k;
		}
	}
	if let Ok(r) = std::env::var("HMD_RATE_LIMIT_RPS") {
		if !r.is_empty() {
			if let Ok(parsed) = r.parse::<u32>() {
				s.rate_limit_rps = parsed;
			}
		}
	}
	if let Ok(b) = std::env::var("HMD_RATE_LIMIT_BURST") {
		if !b.is_empty() {
			if let Ok(parsed) = b.parse::<u32>() {
				s.rate_limit_burst = parsed;
			}
		}
	}
	if let Ok(g) = std::env::var("HMD_AGE_GRAPH") {
		if !g.is_empty() {
			s.age_graph = g;
		}
	}
	if let Ok(l) = std::env::var("HMD_LOG_LEVEL") {
		if !l.is_empty() {
			if let Ok(parsed) = l.parse::<Level>() {
				s.log_level = parsed;
			}
		}
	}

	Ok(s)
}

#[cfg(test)]
#[cfg(feature = "unit-tests")]
mod tests {
	use std::env;

	use log::Level;

	use crate::config::{Settings, load};

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
			s2.database_url.as_str(),
			"postgres://user:pass@localhost/db"
		);
		assert_eq!(s2.tls_cert, "/tmp/cert.pem");
		assert_eq!(s2.tls_key, "/tmp/key.pem");
		assert_eq!(s2.log_level, Level::Debug);

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
