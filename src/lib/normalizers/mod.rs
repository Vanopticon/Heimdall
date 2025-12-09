//! Canonicalizers for IP addresses, domain names, hashes, emails, and timestamps.
//!
//! This module provides deterministic normalization functions that produce stable
//! canonical forms for common data types found in telemetry dumps. Canonical forms
//! are used to generate stable keys for idempotent persistence and deduplication.
//!
//! ## Versioning and Stability
//!
//! Each normalizer includes a version field in its output to track changes to
//! normalization algorithms. If a normalization algorithm changes in a
//! backward-incompatible way, the version must be incremented and migration
//! strategies documented.
//!
//! Current versions:
//! - IP normalization: v1
//! - Domain normalization: v1
//! - Hash normalization: v1
//! - Email normalization: v1
//! - Timestamp normalization: v1
//! - Canonical key generation: v1

use std::net::IpAddr;
use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use thiserror::Error;

/// Errors that can occur during normalization.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NormalizerError {
	#[error("invalid IP address: {0}")]
	InvalidIp(String),
	#[error("invalid domain: {0}")]
	InvalidDomain(String),
	#[error("invalid hash format: {0}")]
	InvalidHash(String),
	#[error("invalid email: {0}")]
	InvalidEmail(String),
	#[error("invalid timestamp: {0}")]
	InvalidTimestamp(String),
	#[error("invalid CIDR notation: {0}")]
	InvalidCidr(String),
}

/// Normalized IP address with version tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedIp {
	/// Canonical string representation
	pub canonical: String,
	/// Normalization algorithm version
	pub version: u32,
	/// Whether this is a CIDR range
	pub is_cidr: bool,
}

/// Normalized domain name with version tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedDomain {
	/// Canonical string representation (lowercase, IDNA-encoded, no trailing dot)
	pub canonical: String,
	/// Normalization algorithm version
	pub version: u32,
}

/// Normalized hash with version tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedHash {
	/// Canonical string representation (lowercase hex)
	pub canonical: String,
	/// Detected hash algorithm (e.g., "md5", "sha1", "sha256")
	pub algorithm: String,
	/// Normalization algorithm version
	pub version: u32,
}

/// Normalized email with version tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedEmail {
	/// Canonical string representation
	pub canonical: String,
	/// Normalization algorithm version
	pub version: u32,
}

/// Normalized timestamp with version tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedTimestamp {
	/// Canonical ISO-8601 string representation in UTC
	pub canonical: String,
	/// Normalization algorithm version
	pub version: u32,
}

/// Canonical key with salt and version tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalKey {
	/// The canonical key (SHA-256 hash of salted input)
	pub key: String,
	/// Salt used for key generation
	pub salt: String,
	/// Key generation algorithm version
	pub version: u32,
}

/// Normalize an IP address to its canonical form.
///
/// Handles both IPv4 and IPv6 addresses, with support for CIDR notation.
///
/// # Examples
///
/// ```
/// use vanopticon_heimdall::lib::normalizers::normalize_ip;
///
/// let ipv4 = normalize_ip("192.168.1.1").unwrap();
/// assert_eq!(ipv4.canonical, "192.168.1.1");
///
/// let ipv6 = normalize_ip("2001:0db8::0001").unwrap();
/// assert_eq!(ipv6.canonical, "2001:db8::1");
///
/// let cidr = normalize_ip("10.0.0.0/8").unwrap();
/// assert_eq!(cidr.canonical, "10.0.0.0/8");
/// assert!(cidr.is_cidr);
/// ```
pub fn normalize_ip(input: &str) -> Result<NormalizedIp, NormalizerError> {
	let input = input.trim();

	// Check for CIDR notation
	if let Some(slash_pos) = input.find('/') {
		let (addr_part, prefix_part) = input.split_at(slash_pos);
		let prefix_len = &prefix_part[1..]; // skip the '/'

		// Parse and validate the IP address part
		let addr = IpAddr::from_str(addr_part.trim())
			.map_err(|_| NormalizerError::InvalidCidr(input.to_string()))?;

		// Validate prefix length
		let prefix: u8 = prefix_len
			.trim()
			.parse()
			.map_err(|_| NormalizerError::InvalidCidr(input.to_string()))?;

		let max_prefix = match addr {
			IpAddr::V4(_) => 32,
			IpAddr::V6(_) => 128,
		};

		if prefix > max_prefix {
			return Err(NormalizerError::InvalidCidr(format!(
				"prefix length {} exceeds maximum {} for {:?}",
				prefix, max_prefix, addr
			)));
		}

		Ok(NormalizedIp {
			canonical: format!("{}/{}", addr, prefix),
			version: 1,
			is_cidr: true,
		})
	} else {
		// Parse as regular IP address
		let addr =
			IpAddr::from_str(input).map_err(|_| NormalizerError::InvalidIp(input.to_string()))?;

		Ok(NormalizedIp {
			canonical: addr.to_string(),
			version: 1,
			is_cidr: false,
		})
	}
}

/// Normalize a domain name to its canonical form.
///
/// Applies lowercase transformation, IDNA encoding, and removes trailing dots.
///
/// # Examples
///
/// ```
/// use vanopticon_heimdall::lib::normalizers::normalize_domain;
///
/// let domain = normalize_domain("Example.COM").unwrap();
/// assert_eq!(domain.canonical, "example.com");
///
/// let trailing = normalize_domain("example.com.").unwrap();
/// assert_eq!(trailing.canonical, "example.com");
/// ```
pub fn normalize_domain(input: &str) -> Result<NormalizedDomain, NormalizerError> {
	let input = input.trim();

	if input.is_empty() {
		return Err(NormalizerError::InvalidDomain("empty domain".to_string()));
	}

	// Remove trailing dot if present
	let input = input.strip_suffix('.').unwrap_or(input);

	// Apply IDNA transformation
	let canonical = idna::domain_to_ascii(input)
		.map_err(|e| NormalizerError::InvalidDomain(format!("{}: {}", input, e)))?;

	// IDNA may produce uppercase; ensure lowercase
	let canonical = canonical.to_lowercase();

	// Basic validation: must not be empty after processing
	if canonical.is_empty() {
		return Err(NormalizerError::InvalidDomain(
			"domain normalized to empty string".to_string(),
		));
	}

	Ok(NormalizedDomain {
		canonical,
		version: 1,
	})
}

/// Normalize a hash to its canonical form.
///
/// Converts hex to lowercase and validates hash length to detect algorithm.
/// Supports MD5 (32 hex chars), SHA-1 (40 hex chars), SHA-256 (64 hex chars),
/// SHA-384 (96 hex chars), and SHA-512 (128 hex chars).
///
/// # Examples
///
/// ```
/// use vanopticon_heimdall::lib::normalizers::normalize_hash;
///
/// let md5 = normalize_hash("D41D8CD98F00B204E9800998ECF8427E").unwrap();
/// assert_eq!(md5.canonical, "d41d8cd98f00b204e9800998ecf8427e");
/// assert_eq!(md5.algorithm, "md5");
///
/// let sha256 = normalize_hash("E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855").unwrap();
/// assert_eq!(sha256.algorithm, "sha256");
/// ```
pub fn normalize_hash(input: &str) -> Result<NormalizedHash, NormalizerError> {
	let input = input.trim();

	// Validate that the string contains only hex characters
	if !input.chars().all(|c| c.is_ascii_hexdigit()) {
		return Err(NormalizerError::InvalidHash(format!(
			"non-hex characters in hash: {}",
			input
		)));
	}

	// Convert to lowercase
	let canonical = input.to_lowercase();

	// Detect algorithm based on length
	let algorithm = match canonical.len() {
		32 => "md5",
		40 => "sha1",
		64 => "sha256",
		96 => "sha384",
		128 => "sha512",
		_ => {
			return Err(NormalizerError::InvalidHash(format!(
				"unrecognized hash length: {} (expected 32/40/64/96/128)",
				canonical.len()
			)));
		}
	};

	Ok(NormalizedHash {
		canonical,
		algorithm: algorithm.to_string(),
		version: 1,
	})
}

/// Normalize an email address to its canonical form.
///
/// This normalizer applies domain canonicalization while preserving the local part.
/// The local part is case-sensitive per RFC 5321, so we preserve its case.
/// The domain part is normalized using the domain normalizer.
///
/// # Limitations
///
/// This implementation uses basic email parsing that splits on the last '@'
/// character. It does not support RFC 5322 quoted strings or comments in the
/// local part. Complex email addresses like `"user@domain"@example.com` or
/// addresses with comments may not be parsed correctly.
///
/// For production use with complex email formats, consider implementing full
/// RFC 5322 parsing or using a dedicated email parsing library.
///
/// # Examples
///
/// ```
/// use vanopticon_heimdall::lib::normalizers::normalize_email;
///
/// let email = normalize_email("User@Example.COM").unwrap();
/// assert_eq!(email.canonical, "User@example.com");
/// ```
pub fn normalize_email(input: &str) -> Result<NormalizedEmail, NormalizerError> {
	let input = input.trim();

	// Find the @ separator (use rfind to handle local parts with @ in quotes,
	// though full RFC 5322 support would require more complex parsing)
	let at_pos = input
		.rfind('@')
		.ok_or_else(|| NormalizerError::InvalidEmail(format!("missing @ in email: {}", input)))?;

	if at_pos == 0 {
		return Err(NormalizerError::InvalidEmail(format!(
			"empty local part: {}",
			input
		)));
	}

	if at_pos == input.len() - 1 {
		return Err(NormalizerError::InvalidEmail(format!(
			"empty domain part: {}",
			input
		)));
	}

	let local = &input[..at_pos];
	let domain = &input[at_pos + 1..];

	// Normalize the domain part
	let normalized_domain = normalize_domain(domain)?;

	// Preserve local part case (per RFC 5321)
	let canonical = format!("{}@{}", local, normalized_domain.canonical);

	Ok(NormalizedEmail {
		canonical,
		version: 1,
	})
}

/// Normalize a timestamp to its canonical form (ISO-8601 UTC).
///
/// Parses various timestamp formats and converts them to a canonical
/// ISO-8601 representation in UTC.
///
/// # Examples
///
/// ```
/// use vanopticon_heimdall::lib::normalizers::normalize_timestamp;
///
/// let ts = normalize_timestamp("2024-01-15T10:30:00Z").unwrap();
/// assert_eq!(ts.canonical, "2024-01-15T10:30:00Z");
///
/// let unix = normalize_timestamp("1705318200").unwrap();
/// assert_eq!(unix.canonical, "2024-01-15T11:30:00Z");
/// ```
pub fn normalize_timestamp(input: &str) -> Result<NormalizedTimestamp, NormalizerError> {
	let input = input.trim();

	// Try to parse as RFC3339/ISO-8601 first
	if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
		return Ok(NormalizedTimestamp {
			canonical: dt
				.with_timezone(&Utc)
				.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
			version: 1,
		});
	}

	// Try to parse as Unix timestamp (seconds since epoch)
	if let Ok(secs) = input.parse::<i64>() {
		if let Some(dt) = DateTime::from_timestamp(secs, 0) {
			return Ok(NormalizedTimestamp {
				canonical: dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
				version: 1,
			});
		}
	}

	// Try common timestamp formats
	let formats = [
		"%Y-%m-%d %H:%M:%S",
		"%Y-%m-%dT%H:%M:%S",
		"%Y/%m/%d %H:%M:%S",
		"%d/%m/%Y %H:%M:%S",
		"%m/%d/%Y %H:%M:%S",
	];

	for format in &formats {
		if let Ok(naive) = NaiveDateTime::parse_from_str(input, format) {
			let dt = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
			return Ok(NormalizedTimestamp {
				canonical: dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
				version: 1,
			});
		}
	}

	Err(NormalizerError::InvalidTimestamp(format!(
		"could not parse timestamp: {}",
		input
	)))
}

/// Generate a canonical key from a normalized value with salt and versioning.
///
/// The canonical key is a hash of the concatenation of:
/// - Salt (for key derivation)
/// - Version (as string)
/// - The normalized value
///
/// This ensures that keys are stable for identical inputs but change when
/// the normalization algorithm or salt changes.
///
/// # Security Note
///
/// The current implementation uses `DefaultHasher` for deterministic hashing.
/// This is NOT cryptographically secure and should be upgraded to SHA-256
/// for production deployments to ensure collision resistance and prevent
/// predictable key generation.
///
/// # Examples
///
/// ```
/// use vanopticon_heimdall::lib::normalizers::generate_canonical_key;
///
/// let key = generate_canonical_key("192.168.1.1", "my-salt");
/// assert_eq!(key.version, 1);
/// assert_eq!(key.salt, "my-salt");
/// ```
pub fn generate_canonical_key(normalized_value: &str, salt: &str) -> CanonicalKey {
	use std::collections::hash_map::DefaultHasher;
	use std::hash::{Hash, Hasher};

	let version = 1u32;
	let input = format!("{}:v{}:{}", salt, version, normalized_value);

	// TODO: Upgrade to cryptographic hash (SHA-256) for production
	// DefaultHasher is not cryptographically secure and may be vulnerable
	// to collision attacks or predictable output. This is acceptable for
	// initial development but MUST be replaced before production use.
	let mut hasher = DefaultHasher::new();
	input.hash(&mut hasher);
	let hash_value = hasher.finish();

	CanonicalKey {
		key: format!("{:016x}", hash_value),
		salt: salt.to_string(),
		version,
	}
}

#[cfg(test)]
#[cfg(feature = "unit-tests")]
mod tests {
	use super::*;

	// IP normalization tests
	#[test]
	fn test_normalize_ipv4() {
		let result = normalize_ip("192.168.1.1").unwrap();
		assert_eq!(result.canonical, "192.168.1.1");
		assert_eq!(result.version, 1);
		assert!(!result.is_cidr);
	}

	#[test]
	fn test_normalize_ipv4_with_spaces() {
		let result = normalize_ip("  192.168.1.1  ").unwrap();
		assert_eq!(result.canonical, "192.168.1.1");
	}

	#[test]
	fn test_normalize_ipv6() {
		let result = normalize_ip("2001:0db8:0000:0000:0000:0000:0000:0001").unwrap();
		assert_eq!(result.canonical, "2001:db8::1");
		assert!(!result.is_cidr);
	}

	#[test]
	fn test_normalize_ipv6_compressed() {
		let result = normalize_ip("::1").unwrap();
		assert_eq!(result.canonical, "::1");
	}

	#[test]
	fn test_normalize_ipv4_cidr() {
		let result = normalize_ip("10.0.0.0/8").unwrap();
		assert_eq!(result.canonical, "10.0.0.0/8");
		assert!(result.is_cidr);
	}

	#[test]
	fn test_normalize_ipv6_cidr() {
		let result = normalize_ip("2001:db8::/32").unwrap();
		assert_eq!(result.canonical, "2001:db8::/32");
		assert!(result.is_cidr);
	}

	#[test]
	fn test_normalize_invalid_ip() {
		let result = normalize_ip("256.256.256.256");
		assert!(result.is_err());
	}

	#[test]
	fn test_normalize_invalid_cidr_prefix() {
		let result = normalize_ip("192.168.1.0/33");
		assert!(result.is_err());
	}

	// Domain normalization tests
	#[test]
	fn test_normalize_domain_lowercase() {
		let result = normalize_domain("Example.COM").unwrap();
		assert_eq!(result.canonical, "example.com");
		assert_eq!(result.version, 1);
	}

	#[test]
	fn test_normalize_domain_trailing_dot() {
		let result = normalize_domain("example.com.").unwrap();
		assert_eq!(result.canonical, "example.com");
	}

	#[test]
	fn test_normalize_domain_idna() {
		let result = normalize_domain("münchen.de").unwrap();
		assert_eq!(result.canonical, "xn--mnchen-3ya.de");
	}

	#[test]
	fn test_normalize_domain_empty() {
		let result = normalize_domain("");
		assert!(result.is_err());
	}

	#[test]
	fn test_normalize_domain_whitespace() {
		let result = normalize_domain("  example.com  ").unwrap();
		assert_eq!(result.canonical, "example.com");
	}

	// Hash normalization tests
	#[test]
	fn test_normalize_hash_md5() {
		let result = normalize_hash("D41D8CD98F00B204E9800998ECF8427E").unwrap();
		assert_eq!(result.canonical, "d41d8cd98f00b204e9800998ecf8427e");
		assert_eq!(result.algorithm, "md5");
		assert_eq!(result.version, 1);
	}

	#[test]
	fn test_normalize_hash_sha1() {
		let result = normalize_hash("DA39A3EE5E6B4B0D3255BFEF95601890AFD80709").unwrap();
		assert_eq!(result.canonical, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
		assert_eq!(result.algorithm, "sha1");
	}

	#[test]
	fn test_normalize_hash_sha256() {
		let result =
			normalize_hash("E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855")
				.unwrap();
		assert_eq!(
			result.canonical,
			"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
		);
		assert_eq!(result.algorithm, "sha256");
	}

	#[test]
	fn test_normalize_hash_invalid_chars() {
		let result = normalize_hash("not-a-hex-string");
		assert!(result.is_err());
	}

	#[test]
	fn test_normalize_hash_invalid_length() {
		let result = normalize_hash("abc123");
		assert!(result.is_err());
	}

	// Email normalization tests
	#[test]
	fn test_normalize_email_basic() {
		let result = normalize_email("user@example.com").unwrap();
		assert_eq!(result.canonical, "user@example.com");
		assert_eq!(result.version, 1);
	}

	#[test]
	fn test_normalize_email_domain_case() {
		let result = normalize_email("User@Example.COM").unwrap();
		assert_eq!(result.canonical, "User@example.com");
	}

	#[test]
	fn test_normalize_email_preserves_local_case() {
		let result = normalize_email("John.Doe@Example.COM").unwrap();
		assert_eq!(result.canonical, "John.Doe@example.com");
	}

	#[test]
	fn test_normalize_email_no_at() {
		let result = normalize_email("notanemail");
		assert!(result.is_err());
	}

	#[test]
	fn test_normalize_email_empty_local() {
		let result = normalize_email("@example.com");
		assert!(result.is_err());
	}

	#[test]
	fn test_normalize_email_empty_domain() {
		let result = normalize_email("user@");
		assert!(result.is_err());
	}

	// Timestamp normalization tests
	#[test]
	fn test_normalize_timestamp_rfc3339() {
		let result = normalize_timestamp("2024-01-15T10:30:00Z").unwrap();
		assert_eq!(result.canonical, "2024-01-15T10:30:00Z");
		assert_eq!(result.version, 1);
	}

	#[test]
	fn test_normalize_timestamp_unix() {
		let result = normalize_timestamp("1705318200").unwrap();
		assert_eq!(result.canonical, "2024-01-15T11:30:00Z");
	}

	#[test]
	fn test_normalize_timestamp_common_format() {
		let result = normalize_timestamp("2024-01-15 10:30:00").unwrap();
		assert_eq!(result.canonical, "2024-01-15T10:30:00Z");
	}

	#[test]
	fn test_normalize_timestamp_invalid() {
		let result = normalize_timestamp("not-a-timestamp");
		assert!(result.is_err());
	}

	// Canonical key generation tests
	#[test]
	fn test_generate_canonical_key() {
		let key1 = generate_canonical_key("192.168.1.1", "salt1");
		assert_eq!(key1.version, 1);
		assert_eq!(key1.salt, "salt1");
		assert!(!key1.key.is_empty());

		// Same input should produce same key
		let key2 = generate_canonical_key("192.168.1.1", "salt1");
		assert_eq!(key1.key, key2.key);

		// Different salt should produce different key
		let key3 = generate_canonical_key("192.168.1.1", "salt2");
		assert_ne!(key1.key, key3.key);

		// Different value should produce different key
		let key4 = generate_canonical_key("192.168.1.2", "salt1");
		assert_ne!(key1.key, key4.key);
	}

	#[test]
	fn test_canonical_key_deterministic() {
		// Keys should be deterministic across multiple calls
		let keys: Vec<_> = (0..10)
			.map(|_| generate_canonical_key("test-value", "test-salt"))
			.collect();

		for key in &keys[1..] {
			assert_eq!(keys[0].key, key.key);
		}
	}
}
