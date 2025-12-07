use anyhow::Result;
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedRecord {
	/// The type of the field (e.g. "ip", "domain", "hash", "email")
	pub field_type: String,
	/// Original/raw value as seen in the dump
	pub raw: String,
	/// Canonicalized value used as a merge key
	pub canonical: String,
}

/// Normalize a CSV input where each row is: `field_type,value`.
/// Returns a vector of `NormalizedRecord`.
pub fn normalize_csv(input: &str) -> Result<Vec<NormalizedRecord>> {
	let mut rdr = csv::ReaderBuilder::new()
		.has_headers(true)
		.trim(csv::Trim::All)
		.from_reader(input.as_bytes());

	let mut out = Vec::new();

	// simple regex for trimming surrounding punctuation from hashes/emails
	let punct_re = Regex::new(r"^[\W_]+|[\W_]+$").unwrap();

	for result in rdr.records() {
		let record = result?;
		if record.len() < 2 {
			continue;
		}
		let ftype = record.get(0).unwrap_or("").to_lowercase();
		let raw = record.get(1).unwrap_or("").to_string();

		let canonical = match ftype.as_str() {
			"domain" => {
				let mut v = raw.trim().to_lowercase();
				if v.ends_with('.') {
					v.pop();
				}
				v
			}
			"ip" => raw.trim().to_string(),
			"hash" => punct_re.replace_all(&raw.to_lowercase(), "").to_string(),
			"email" => raw.trim().to_lowercase().to_string(),
			_ => raw.trim().to_lowercase().to_string(),
		};

		out.push(NormalizedRecord {
			field_type: ftype,
			raw,
			canonical,
		});
	}

	Ok(out)
}

#[cfg(feature = "ingest-tests")]
mod tests {
	use super::*;

	#[test]
	fn normalizes_csv_rows() {
		let csv = "field_type,value\n\
domain,Example.COM\n\
ip, 192.0.2.1 \n\
hash, ABCDEF123456 \n\
email,USER@EXAMPLE.COM\n";

		let got = normalize_csv(csv).expect("normalize");
		assert_eq!(got.len(), 4);
		assert_eq!(got[0].field_type, "domain");
		assert_eq!(got[0].canonical, "example.com");
		assert_eq!(got[1].field_type, "ip");
		assert_eq!(got[1].canonical, "192.0.2.1");
		assert_eq!(got[2].field_type, "hash");
		assert_eq!(got[2].canonical, "abcdef123456");
		assert_eq!(got[3].field_type, "email");
		assert_eq!(got[3].canonical, "user@example.com");
	}
}
