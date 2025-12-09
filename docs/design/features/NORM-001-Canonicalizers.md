# Feature: NORM-001 -> Canonicalizers for Data Normalization

## Related Items

- Priority: Top
- See: `docs/design/Implementation-Roadmap.md`, `docs/design/Normalizers.md`, `docs/design/features/ING-001-Bulk-Dump-Normalization.md`

## Story

As a Full-Stack operator, I want deterministic canonicalizers for IP addresses, domain names, hashes, emails, and timestamps so that Heimdall can generate stable canonical keys for idempotent persistence and deduplication across multiple data sources.

## Overview

Implement normalizers that produce stable canonical forms for common data types found in telemetry dumps:

- **IP addresses**: Normalize IPv4/IPv6 addresses with CIDR support
- **Domain names**: Apply lowercase, IDNA encoding, and remove trailing dots
- **Hashes**: Detect algorithm and normalize to lowercase hex
- **Email addresses**: Preserve local-part case, normalize domain
- **Timestamps**: Parse various formats and convert to ISO-8601 UTC
- **Canonical keys**: Generate stable keys with salt and versioning

Each normalizer includes version tracking to support algorithm evolution and data migration.

## Out of Scope

- Advanced PII redaction (handled by separate PII policy module)
- Format-specific parsers for structured data types
- Semantic validation beyond structural correctness

## Implementation Details

### Module Structure

```
src/lib/normalizers/mod.rs
├── normalize_ip()         - IPv4/IPv6 with CIDR
├── normalize_domain()     - Lowercase, IDNA, trailing dot removal
├── normalize_hash()       - Algorithm detection, hex normalization
├── normalize_email()      - Domain normalization, local-part preservation
├── normalize_timestamp()  - Multi-format parsing, UTC conversion
└── generate_canonical_key() - Salted, versioned key generation
```

### Dependencies

- `chrono`: Timestamp parsing and manipulation
- `idna`: Internationalized domain name encoding
- Standard library: IP address parsing, hashing

### Key Design Decisions

1. **Version Tracking**: Each normalizer embeds a version number in its output to enable migration when algorithms change
2. **Salt Strategy**: Canonical keys use a deployment-specific salt (configured via environment) for key uniqueness
3. **Case Preservation**: Email local parts preserve case per RFC 5321; domains are lowercased
4. **CIDR Support**: IP normalizer handles both single addresses and CIDR ranges
5. **Algorithm Detection**: Hash normalizer detects algorithm by hex length (32/40/64/96/128)

### Testing Strategy

Comprehensive unit tests cover:

- Valid inputs for all data types
- Edge cases (whitespace, empty strings, special characters)
- Invalid inputs and error handling
- Unicode and internationalization (IDNA)
- IPv6 compression and CIDR validation
- Canonical key determinism and salt uniqueness

## Acceptance Criteria

- [x] All normalizers implemented with version tracking
- [x] IP normalizer handles IPv4, IPv6, and CIDR notation
- [x] Domain normalizer applies IDNA encoding for internationalized domains
- [x] Hash normalizer detects MD5, SHA-1, SHA-256, SHA-384, SHA-512
- [x] Email normalizer preserves local-part case, normalizes domain
- [x] Timestamp normalizer parses RFC3339, Unix timestamps, and common formats
- [x] Canonical key generation is deterministic with salt and versioning
- [x] Unit tests pass for all edge cases
- [x] Documentation describes behavior, versioning, and salt strategy

## Integration Points

### Ingest Pipeline

Normalizers are invoked during the "Scrub & Normalize Phase" of bulk dump ingestion:

1. Parser emits raw field values
2. PII policy determines if value should be processed
3. Normalizer produces canonical form
4. Canonical key generator creates stable key
5. Key used for idempotent MERGE in Postgres+AGE

See `docs/design/features/ING-001-Bulk-Dump-Normalization.md` for full pipeline details.

### Persistence Layer

Canonical keys enable idempotent writes:

- Same canonical value (e.g., IP address) always produces same key
- Keys serve as node identifiers in the graph
- Multiple sightings of the same value link to the same canonical node

## Migration and Evolution

### Algorithm Changes

When a normalization algorithm must change:

1. Increment version number in the normalizer
2. Document the change and rationale in `docs/design/Normalizers.md`
3. Plan data migration:
	- Option A: Reprocess all data with new algorithm
	- Option B: Support parallel versions during transition
	- Option C: Accept coexistence of old and new canonical forms

### Salt Rotation

Changing the canonical key salt requires full data reprocessing:

1. Export current canonical mappings
2. Rotate salt in configuration
3. Reprocess all dumps with new salt
4. Update all canonical keys and cross-references

**Salt rotation is expensive and should be rare.**

## Security Considerations

1. **PII Handling**: Normalizers preserve input values; apply PII policies before normalization
2. **Salt Protection**: Canonical key salt is sensitive configuration; protect accordingly
3. **Input Validation**: All inputs are validated to prevent injection attacks
4. **Hash Algorithm**: Consider upgrading from DefaultHasher to SHA-256 for production canonical keys

## Future Enhancements

- [ ] Support IPv4-mapped IPv6 address conversion
- [ ] Additional timestamp format parsers (e.g., syslog formats)
- [ ] URL/URI normalization
- [ ] MAC address normalization
- [ ] Base64-encoded hash support
- [ ] Cryptographic upgrade for canonical key generation

## Notes

- This feature implements Milestone 1, Task 1.1 from the Implementation Roadmap
- Normalizers are a foundational component for idempotent ingest and deduplication
- Canonical key stability is critical for long-term data integrity
- Version tracking enables backward-compatible algorithm evolution
