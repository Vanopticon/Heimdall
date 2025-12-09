use anyhow::{Context, Result};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use log::{debug, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// OIDC discovery document structure as defined by OpenID Connect Discovery 1.0
#[derive(Debug, Deserialize, Clone)]
pub struct OidcDiscoveryDocument {
	pub issuer: String,
	pub authorization_endpoint: String,
	pub token_endpoint: String,
	pub jwks_uri: String,
	pub userinfo_endpoint: Option<String>,
	pub end_session_endpoint: Option<String>,
}

/// JSON Web Key Set (JWKS) structure
#[derive(Debug, Deserialize, Clone)]
pub struct Jwks {
	pub keys: Vec<Jwk>,
}

/// Individual JSON Web Key
#[derive(Debug, Deserialize, Clone)]
pub struct Jwk {
	pub kty: String,
	pub kid: Option<String>,
	pub alg: Option<String>,
	pub n: Option<String>,
	pub e: Option<String>,
}

/// Claims structure for JWT validation
#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
	pub sub: String,
	pub iss: String,
	pub aud: String,
	pub exp: u64,
	pub iat: u64,
	pub azp: Option<String>,
	pub scope: Option<String>,
}

/// OIDC provider configuration and validation state
pub struct OidcProvider {
	discovery_url: String,
	client_id: String,
	client_secret: String,
	discovery_doc: Arc<RwLock<Option<OidcDiscoveryDocument>>>,
	jwks: Arc<RwLock<Option<Jwks>>>,
	client: Client,
}

impl OidcProvider {
	/// Create a new OIDC provider instance
	pub fn new(discovery_url: String, client_id: String, client_secret: String) -> Self {
		let client = Client::builder()
			.timeout(Duration::from_secs(30))
			.build()
			.expect("failed to build HTTP client for OIDC");

		Self {
			discovery_url,
			client_id,
			client_secret,
			discovery_doc: Arc::new(RwLock::new(None)),
			jwks: Arc::new(RwLock::new(None)),
			client,
		}
	}

	/// Fetch the OIDC discovery document from the provider
	pub async fn fetch_discovery(&self) -> Result<OidcDiscoveryDocument> {
		info!("Fetching OIDC discovery document from {}", self.discovery_url);

		let doc = self
			.client
			.get(&self.discovery_url)
			.send()
			.await
			.context("failed to fetch OIDC discovery document")?
			.json::<OidcDiscoveryDocument>()
			.await
			.context("failed to parse OIDC discovery document")?;

		debug!("OIDC issuer: {}", doc.issuer);
		debug!("OIDC jwks_uri: {}", doc.jwks_uri);

		let mut discovery = self.discovery_doc.write().await;
		*discovery = Some(doc.clone());

		Ok(doc)
	}

	/// Fetch the JSON Web Key Set (JWKS) from the provider
	pub async fn fetch_jwks(&self) -> Result<Jwks> {
		let discovery = self.discovery_doc.read().await;
		let doc = discovery
			.as_ref()
			.context("discovery document not loaded; call fetch_discovery first")?;

		info!("Fetching JWKS from {}", doc.jwks_uri);

		let jwks = self
			.client
			.get(&doc.jwks_uri)
			.send()
			.await
			.context("failed to fetch JWKS")?
			.json::<Jwks>()
			.await
			.context("failed to parse JWKS")?;

		debug!("Fetched {} JWKs", jwks.keys.len());

		let mut keys = self.jwks.write().await;
		*keys = Some(jwks.clone());

		Ok(jwks)
	}

	/// Initialize the OIDC provider by fetching discovery and JWKS
	pub async fn initialize(&self) -> Result<()> {
		self.fetch_discovery().await?;
		self.fetch_jwks().await?;
		info!("OIDC provider initialized successfully");
		Ok(())
	}

	/// Validate a JWT token and return the claims if valid
	pub async fn validate_token(&self, token: &str) -> Result<Claims> {
		// Decode header to get the key ID (kid)
		let header = decode_header(token).context("failed to decode JWT header")?;

		let kid = header.kid.context("JWT header missing 'kid' field")?;

		// Find the matching key in the JWKS
		let jwks = self.jwks.read().await;
		let keys = jwks
			.as_ref()
			.context("JWKS not loaded; call initialize first")?;

		let jwk = keys
			.keys
			.iter()
			.find(|k| k.kid.as_ref() == Some(&kid))
			.context("no matching key found in JWKS")?;

		// Validate that the key is RSA
		if jwk.kty != "RSA" {
			anyhow::bail!("unsupported key type: {}", jwk.kty);
		}

		let n = jwk.n.as_ref().context("JWK missing 'n' parameter")?;
		let e = jwk.e.as_ref().context("JWK missing 'e' parameter")?;

		// Construct the decoding key from RSA components
		let decoding_key =
			DecodingKey::from_rsa_components(n, e).context("failed to construct decoding key")?;

		// Set validation parameters
		let discovery = self.discovery_doc.read().await;
		let doc = discovery
			.as_ref()
			.context("discovery document not loaded")?;

		let mut validation = Validation::new(Algorithm::RS256);
		validation.set_issuer(&[&doc.issuer]);
		validation.set_audience(&[&self.client_id]);

		// Decode and validate the token
		let token_data = decode::<Claims>(token, &decoding_key, &validation)
			.context("failed to validate JWT")?;

		// Additional expiration check
		let now = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_secs();

		if token_data.claims.exp < now {
			anyhow::bail!("token has expired");
		}

		debug!("Token validated successfully for subject: {}", token_data.claims.sub);

		Ok(token_data.claims)
	}

	/// Obtain a machine-to-machine (M2M) access token using client credentials
	pub async fn get_client_credentials_token(&self, scope: Option<&str>) -> Result<String> {
		let discovery = self.discovery_doc.read().await;
		let doc = discovery
			.as_ref()
			.context("discovery document not loaded; call initialize first")?;

		info!("Requesting client credentials token from {}", doc.token_endpoint);

		let mut params = HashMap::new();
		params.insert("grant_type", "client_credentials");
		params.insert("client_id", self.client_id.as_str());
		params.insert("client_secret", self.client_secret.as_str());

		if let Some(s) = scope {
			params.insert("scope", s);
		}

		let response = self
			.client
			.post(&doc.token_endpoint)
			.form(&params)
			.send()
			.await
			.context("failed to request client credentials token")?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("token request failed with status {}: {}", status, body);
		}

		let token_response: serde_json::Value = response
			.json()
			.await
			.context("failed to parse token response")?;

		let access_token = token_response
			.get("access_token")
			.and_then(|v| v.as_str())
			.context("token response missing 'access_token' field")?;

		debug!("Successfully obtained client credentials token");

		Ok(access_token.to_string())
	}

	/// Refresh JWKS if needed (e.g., after a validation failure)
	pub async fn refresh_jwks(&self) -> Result<()> {
		warn!("Refreshing JWKS");
		self.fetch_jwks().await?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_oidc_provider_creation() {
		let provider = OidcProvider::new(
			"https://example.com/.well-known/openid-configuration".to_string(),
			"test-client".to_string(),
			"test-secret".to_string(),
		);

		assert_eq!(provider.discovery_url, "https://example.com/.well-known/openid-configuration");
		assert_eq!(provider.client_id, "test-client");
	}

	#[test]
	fn test_claims_serialization() {
		let claims = Claims {
			sub: "user123".to_string(),
			iss: "https://issuer.example.com".to_string(),
			aud: "client-id".to_string(),
			exp: 1234567890,
			iat: 1234567800,
			azp: Some("azp-value".to_string()),
			scope: Some("openid profile".to_string()),
		};

		let json = serde_json::to_string(&claims).unwrap();
		assert!(json.contains("user123"));
		assert!(json.contains("https://issuer.example.com"));
	}
}
