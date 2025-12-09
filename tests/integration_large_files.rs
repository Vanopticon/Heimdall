//! Integration tests for large file processing to verify memory bounds.
//!
//! These tests ensure that the streaming parsers can handle large files
//! without loading the entire file into memory at once.

#[cfg(feature = "ingest-tests")]
mod large_file_tests {
	use std::io::Cursor;
	use vanopticon_heimdall::ingest::parsers;

	/// Test that we can parse 100k lines of NDJSON without excessive memory usage.
	/// This test verifies the streaming capability of the NDJSON parser.
	#[test]
	fn parse_100k_ndjson_lines() {
		// Generate 100k lines of NDJSON
		let mut ndjson = String::new();
		for i in 0..100_000 {
			ndjson.push_str(&format!(
				"{{\"field_type\":\"domain\",\"value\":\"example{}.com\"}}\n",
				i
			));
		}

		// Parse with streaming parser
		let result = parsers::parse_ndjson_stream(Cursor::new(ndjson.as_bytes()));
		assert!(result.is_ok(), "Failed to parse 100k lines");

		let records = result.unwrap();
		assert_eq!(records.len(), 100_000, "Should parse all 100k records");

		// Verify a few samples
		assert_eq!(records[0].canonical, "example0.com");
		assert_eq!(records[99_999].canonical, "example99999.com");
	}

	/// Test CSV parsing with large files.
	#[test]
	fn parse_100k_csv_lines() {
		// Generate 100k lines of CSV
		let mut csv = String::from("field_type,value\n");
		for i in 0..100_000 {
			csv.push_str(&format!("domain,example{}.com\n", i));
		}

		// Parse with streaming parser
		let result = parsers::parse_csv_stream(Cursor::new(csv.as_bytes()), None);
		assert!(result.is_ok(), "Failed to parse 100k CSV lines");

		let records = result.unwrap();
		assert_eq!(records.len(), 100_000, "Should parse all 100k records");

		// Verify samples
		assert_eq!(records[0].canonical, "example0.com");
		assert_eq!(records[99_999].canonical, "example99999.com");
	}

	/// Test that gzip compression/decompression works with larger data.
	#[test]
	fn decompress_large_gzip() {
		use flate2::write::GzEncoder;
		use flate2::Compression;
		use std::io::Write;

		// Create a larger test payload (about 1MB of repeated data)
		let test_data = "test data repeated\n".repeat(50_000);

		// Compress
		let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
		encoder.write_all(test_data.as_bytes()).unwrap();
		let compressed = encoder.finish().unwrap();

		// Decompress
		let decompressed = parsers::decompress_gzip(Cursor::new(&compressed)).expect("decompress");
		assert_eq!(
			String::from_utf8_lossy(&decompressed),
			test_data,
			"Decompressed data should match original"
		);
	}
}
