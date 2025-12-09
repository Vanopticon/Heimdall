use anyhow::{Context, Result};
use rustls_pemfile::{certs as pem_certs, pkcs8_private_keys, rsa_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::rustls::{self, Certificate, PrivateKey, server::ServerConfig};

/// Load PEM-encoded certificates from `path` and return them as `rustls::Certificate`.
pub fn load_certs(path: &Path) -> Result<Vec<Certificate>> {
	let f = File::open(path).with_context(|| format!("opening cert file {}", path.display()))?;
	let mut reader = BufReader::new(f);
	let raw =
		pem_certs(&mut reader).map_err(|e| anyhow::anyhow!("failed to parse PEM certs: {}", e))?;
	if raw.is_empty() {
		anyhow::bail!("no certificates found in {}", path.display());
	}
	Ok(raw.into_iter().map(Certificate).collect())
}

/// Load a private key (PKCS#8 preferred, RSA fallback) from `path` and return it as `rustls::PrivateKey`.
pub fn load_private_key(path: &Path) -> Result<PrivateKey> {
	// Try PKCS#8 first
	let f = File::open(path).with_context(|| format!("opening key file {}", path.display()))?;
	let mut reader = BufReader::new(&f);
	let pks = pkcs8_private_keys(&mut reader)
		.map_err(|e| anyhow::anyhow!("failed to parse PKCS#8 keys: {}", e))?;
	if let Some(k) = pks.into_iter().next() {
		return Ok(PrivateKey(k));
	}

	// PKCS#8 not found; try RSA
	let f = File::open(path)
		.with_context(|| format!("opening key file {} (rsa pass)", path.display()))?;
	let mut reader = BufReader::new(f);
	let rs = rsa_private_keys(&mut reader)
		.map_err(|e| anyhow::anyhow!("failed to parse RSA keys: {}", e))?;
	if let Some(k) = rs.into_iter().next() {
		return Ok(PrivateKey(k));
	}

	anyhow::bail!("no private key found in {}", path.display());
}

/// Parse the DER-encoded certificate bytes and return the x509 parser's `X509Certificate`.
pub fn parse_first_cert_x509<'a>(
	cert: &'a Certificate,
) -> Result<x509_parser::certificate::X509Certificate<'a>> {
	let res = x509_parser::parse_x509_certificate(&cert.0)
		.map_err(|e| anyhow::anyhow!("failed to parse x509 certificate: {:?}", e))?;
	Ok(res.1)
}

/// Return true if the certificate appears to be self-signed (subject == issuer).
pub fn is_self_signed(cert: &Certificate) -> Result<bool> {
	let parsed = parse_first_cert_x509(cert)?;
	Ok(parsed.tbs_certificate.subject == parsed.tbs_certificate.issuer)
}

/// Extract DNS names from the SubjectAlternativeName extension, if present.
pub fn dns_names_from_cert(cert: &Certificate) -> Result<Vec<String>> {
	let parsed = parse_first_cert_x509(cert)?;

	let mut out: Vec<String> = Vec::new();

	use x509_parser::extensions::GeneralName;
	use x509_parser::extensions::ParsedExtension;

	for ext in parsed.extensions().iter() {
		match ext.parsed_extension() {
			ParsedExtension::SubjectAlternativeName(san) => {
				for name in san.general_names.iter() {
					if let GeneralName::DNSName(d) = name {
						out.push(d.to_string());
					}
				}
			}
			_ => {}
		}
	}

	Ok(out)
}

/// Return the first Common Name (CN) from the subject, if present.
pub fn first_common_name(cert: &Certificate) -> Result<Option<String>> {
	let parsed = parse_first_cert_x509(cert)?;

	// The subject is an RDNSequence: iterate to find attributes with OID for CN
	for cn in parsed.subject().iter_common_name() {
		if let Ok(s) = cn.as_str() {
			return Ok(Some(s.to_string()));
		}
	}

	Ok(None)
}

/// Return true if the certificate is expired at the current UTC time.
pub fn is_cert_expired(cert: &Certificate) -> Result<bool> {
	let parsed = parse_first_cert_x509(cert)?;

	let not_after = parsed.validity().not_after.to_datetime();
	let now = x509_parser::time::ASN1Time::now().to_datetime();

	Ok(not_after <= now)
}

/// Build a rustls `ServerConfig` restricted to TLS1.3. Returns an `Arc<ServerConfig>` suitable for `tokio_rustls::TlsAcceptor::from(...)`.
pub fn build_server_config_tls13(
	certs: Vec<Certificate>,
	key: PrivateKey,
) -> Result<Arc<ServerConfig>> {
	// Policy: reject self-signed leaf certificates. This prevents starting
	// the server with a certificate that is not signed by a trusted CA.
	if certs.is_empty() {
		anyhow::bail!("no certificates provided to build server config");
	}

	// If the first (leaf) certificate is self-signed, refuse to build the
	// server config as a security policy.
	let first = &certs[0];
	if is_self_signed(first)? {
		anyhow::bail!("self-signed certificates are not allowed for server TLS");
	}

	// Note: use the server-side builder pattern from rustls 0.21
	let cfg_builder = ServerConfig::builder()
		.with_safe_default_cipher_suites()
		.with_safe_default_kx_groups()
		.with_protocol_versions(&[&rustls::version::TLS13])
		.map_err(|e| anyhow::anyhow!("failed to negotiate protocol versions: {:?}", e))?;

	let cfg = cfg_builder
		.with_no_client_auth()
		.with_single_cert(certs, key)
		.map_err(|e| anyhow::anyhow!("failed to build server config: {}", e))?;

	Ok(Arc::new(cfg))
}

#[cfg(test)]
mod tests {
	use super::*;
	// PathBuf not needed in these simple tests

	#[test]
	fn load_certs_missing_path_returns_err() {
		let p = Path::new("/this/path/does/not/exist/cert.pem");
		assert!(load_certs(p).is_err());
	}

	#[test]
	fn load_key_missing_path_returns_err() {
		let p = Path::new("/this/path/does/not/exist/key.pem");
		assert!(load_private_key(p).is_err());
	}
}
