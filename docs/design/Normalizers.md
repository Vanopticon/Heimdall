# Normalizers — Canonical Key Generation

## Overview

Heimdall's normalizers provide deterministic canonicalization for common data types found in telemetry dumps. Canonical forms enable stable key generation for idempotent persistence and deduplication across multiple data sources.

## Supported Data Types

### IP Addresses

**Module**: `normalize_ip`

**Features**:

- IPv4 and IPv6 address normalization
- CIDR notation support (e.g., `10.0.0.0/8`, `2001:db8::/32`)
- Automatic compression of IPv6 addresses (e.g., `2001:0db8::0001` → `2001:db8::1`)
- Validation of CIDR prefix lengths

**Examples**:

```rust
use vanopticon_heimdall::lib::normalizers::normalize_ip;

// IPv4
let ipv4 = normalize_ip("192.168.1.1").unwrap();
assert_eq!(ipv4.canonical, "192.168.1.1");

// IPv6 with compression
let ipv6 = normalize_ip("2001:0db8::0001").unwrap();
assert_eq!(ipv6.canonical, "2001:db8::1");

// CIDR notation
let cidr = normalize_ip("10.0.0.0/8").unwrap();
assert!(cidr.is_cidr);
```

### Domain Names

**Module**: `normalize_domain`

**Features**:

- Lowercase transformation
- IDNA (Internationalized Domain Names in Applications) encoding
- Trailing dot removal
- Unicode domain name support

**Examples**:

```rust
use vanopticon_heimdall::lib::normalizers::normalize_domain;

let domain = normalize_domain("Example.COM").unwrap();
assert_eq!(domain.canonical, "example.com");

// IDNA encoding for internationalized domains
let idna = normalize_domain("münchen.de").unwrap();
assert_eq!(idna.canonical, "xn--mnchen-3ya.de");
```

### Hash Values

**Module**: `normalize_hash`

**Features**:

- Automatic algorithm detection (MD5, SHA-1, SHA-256, SHA-384, SHA-512)
- Lowercase hex normalization
- Length validation

**Supported Algorithms**:

| Algorithm | Length (hex chars) |
| --------- | ------------------ |
| MD5       | 32                 |
| SHA-1     | 40                 |
| SHA-256   | 64                 |
| SHA-384   | 96                 |
| SHA-512   | 128                |

**Examples**:

```rust
use vanopticon_heimdall::lib::normalizers::normalize_hash;

let md5 = normalize_hash("D41D8CD98F00B204E9800998ECF8427E").unwrap();
assert_eq!(md5.canonical, "d41d8cd98f00b204e9800998ecf8427e");
assert_eq!(md5.algorithm, "md5");
```

### Email Addresses

**Module**: `normalize_email`

**Features**:

- Domain canonicalization (using domain normalizer)
- Local-part case preservation (per RFC 5321)
- Basic structural validation

**Examples**:

```rust
use vanopticon_heimdall::lib::normalizers::normalize_email;

let email = normalize_email("User@Example.COM").unwrap();
// Local part case is preserved, domain is lowercased
assert_eq!(email.canonical, "User@example.com");
```

### Timestamps

**Module**: `normalize_timestamp`

**Features**:

- Multiple input format support
- Conversion to ISO-8601 UTC
- Unix timestamp support (seconds since epoch)

**Supported Input Formats**:

- RFC3339/ISO-8601 (e.g., `2024-01-15T10:30:00Z`)
- Unix timestamps (e.g., `1705318200`)
- Common date-time patterns:
	- `YYYY-MM-DD HH:MM:SS`
	- `YYYY-MM-DDTHH:MM:SS`
	- `YYYY/MM/DD HH:MM:SS`
	- `DD/MM/YYYY HH:MM:SS`
	- `MM/DD/YYYY HH:MM:SS`

**Examples**:

```rust
use vanopticon_heimdall::lib::normalizers::normalize_timestamp;

// RFC3339
let ts = normalize_timestamp("2024-01-15T10:30:00Z").unwrap();
assert_eq!(ts.canonical, "2024-01-15T10:30:00Z");

// Unix timestamp
let unix = normalize_timestamp("1705318200").unwrap();
assert_eq!(unix.canonical, "2024-01-15T11:30:00Z");
```

## Canonical Key Generation

**Module**: `generate_canonical_key`

Generates stable canonical keys for idempotent persistence using:

- Input: normalized value
- Salt: deployment-specific salt string
- Version: algorithm version tracking
- Output: deterministic hash key

**Key Properties**:

1. **Deterministic**: Same input always produces the same key
2. **Versioned**: Keys include version metadata for migration tracking
3. **Salted**: Keys are unique per deployment/configuration
4. **Stable**: Keys remain constant unless normalization algorithm changes

**Examples**:

```rust
use vanopticon_heimdall::lib::normalizers::generate_canonical_key;

let key = generate_canonical_key("192.168.1.1", "my-deployment-salt");
assert_eq!(key.version, 1);
assert_eq!(key.salt, "my-deployment-salt");
// key.key contains the stable hash
```

## Versioning Strategy

Each normalizer tracks its algorithm version. When a normalization algorithm changes in a backward-incompatible way:

1. Increment the version number in the normalizer
2. Document the change in this file
3. Plan and execute data migration if needed

### Current Versions

| Normalizer | Version | Last Changed | Notes                    |
| ---------- | ------- | ------------ | ------------------------ |
| IP         | 1       | 2024-12-09   | Initial implementation   |
| Domain     | 1       | 2024-12-09   | Initial implementation   |
| Hash       | 1       | 2024-12-09   | Initial implementation   |
| Email      | 1       | 2024-12-09   | Initial implementation   |
| Timestamp  | 1       | 2024-12-09   | Initial implementation   |
| Key        | 1       | 2024-12-09   | Initial implementation   |

## Salt Strategy

### Recommended Salt Configuration

1. **Generate**: Use a cryptographically secure random string (32+ characters)
2. **Store**: Configure via environment variable (e.g., `HMD_CANONICAL_KEY_SALT`)
3. **Persist**: Document the salt in secure configuration management
4. **Rotate**: Only rotate salts during planned migrations with full data reprocessing

### Salt Rotation Impact

Changing the salt will change all canonical keys, requiring:

- Reprocessing all existing data
- Regenerating all canonical keys
- Updating all cross-references

**Do not rotate salts** unless you have a documented migration plan.

## Edge Cases and Limitations

### IP Addresses

- IPv4-mapped IPv6 addresses (`::ffff:192.0.2.1`) are not automatically converted to IPv4
- Zone identifiers in IPv6 addresses (e.g., `fe80::1%eth0`) are not supported

### Domain Names

- Punycode domains are accepted and remain in punycode form
- Very long domain labels (>63 characters) may be rejected by IDNA encoding
- Domains must conform to DNS naming rules

### Hash Values

- Only recognizes hashes by exact length (32, 40, 64, 96, 128 hex characters)
- Does not validate that hash values are correct for their algorithm
- Non-standard hash formats (e.g., base64-encoded) are not supported

### Email Addresses

- Basic structural validation only (presence of `@` and non-empty parts)
- Does not support RFC 5322 quoted strings or comments in local parts
- Complex email addresses (e.g., `"user@domain"@example.com`) may not parse correctly
- Does not validate domain existence or MX records
- Uses `rfind('@')` to split email, which handles most common cases but not all RFC 5322 edge cases

### Timestamps

- Limited format support (common patterns only)
- Assumes UTC for formats without timezone information
- Does not handle dates before Unix epoch (1970-01-01) or far future dates

## Usage in Ingest Pipeline

Normalizers are used in the ingest pipeline to:

1. **Canonicalize** incoming field values
2. **Generate** stable keys for deduplication
3. **Enable** idempotent writes to the graph database
4. **Support** cross-dump correlation

See `docs/design/features/ING-001-Bulk-Dump-Normalization.md` for integration details.

## Testing

Comprehensive unit tests cover:

- Valid inputs for all data types
- Edge cases (empty strings, whitespace, special characters)
- Invalid inputs and error handling
- Unicode and internationalization
- Canonical key determinism

Run tests with:

```bash
cargo test --lib --features unit-tests normalizers
```

## Security Considerations

1. **PII Handling**: Normalizers preserve input values; apply PII policies before normalization
2. **Hash Algorithm**: Current key generation uses `DefaultHasher` which is NOT cryptographically secure
	- **CRITICAL**: Upgrade to SHA-256 before production deployment
	- DefaultHasher is vulnerable to collision attacks and predictable output
	- Current implementation is acceptable for development only
3. **Salt Protection**: Protect salt values as sensitive configuration
4. **Input Validation**: All inputs are validated before processing to prevent injection attacks

## Future Enhancements

Potential improvements for future versions:

- [ ] Support for IPv4-mapped IPv6 address conversion
- [ ] Additional timestamp format parsers
- [ ] URL/URI normalization
- [ ] MAC address normalization
- [ ] Cryptographic hash upgrade for canonical keys
- [ ] Base64-encoded hash support
- [ ] CIDR range containment checks
