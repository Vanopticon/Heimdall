use anyhow::Result;
use regex::Regex;
use serde_json::Value;

use crate::ingest::NormalizedRecord;

/// Normalize a NDJSON (newline-delimited JSON) payload where each line is an object
/// describing a single field/value pair. The function is intentionally permissive
/// about the exact JSON schema and will attempt to extract `field_type` (or `type`)
/// and `value` (or `raw`) from each object. It also supports simple array rows
/// like `["domain", "example.com"]` and `"type,value"` strings as a last resort.
pub fn normalize_ndjson(input: &str) -> Result<Vec<NormalizedRecord>> {
	let mut out = Vec::new();

	let punct_re = Regex::new(r"^[\W_]+|[\W_]+$").unwrap();

	for line in input.lines() {
		let line = line.trim();
		if line.is_empty() {
			continue;
		}

		// Try to parse JSON value
		let v: Value = match serde_json::from_str(line) {
			Ok(v) => v,
			Err(_) => {
				// fallback: allow simple CSV-like `type,value` strings
				if let Some((ft, val)) = line.split_once(',') {
					let ftype = ft.trim().to_lowercase();
					let raw = val.trim().to_string();
					let canonical = canonicalize(&ftype, &raw, &punct_re);
					out.push(NormalizedRecord {
						field_type: ftype,
						raw,
						canonical,
					});
					continue;
				}
				// skip unparseable line
				continue;
			}
		};

		// Extract field_type and value from the JSON value
		if let Some((ftype, raw)) = extract_field_and_value(&v) {
			let canonical = canonicalize(&ftype, &raw, &punct_re);
			out.push(NormalizedRecord {
				field_type: ftype,
				raw,
				canonical,
			});
		}
	}

	Ok(out)
}

/// Normalize a single NDJSON line (object/array/string/csv fallback) into
/// a single `NormalizedRecord`. Returns `Some(NormalizedRecord)` if the line
/// could be parsed and normalized, otherwise `None`.
pub fn normalize_ndjson_line(line: &str, punct_re: &Regex) -> Option<NormalizedRecord> {
	let line = line.trim();
	if line.is_empty() {
		return None;
	}

	// Try to parse JSON value
	let v: Value = match serde_json::from_str(line) {
		Ok(v) => v,
		Err(_) => {
			// fallback: allow simple CSV-like `type,value` strings
			if let Some((ft, val)) = line.split_once(',') {
				let ftype = ft.trim().to_lowercase();
				let raw = val.trim().to_string();
				let canonical = canonicalize(&ftype, &raw, punct_re);
				return Some(NormalizedRecord {
					field_type: ftype,
					raw,
					canonical,
				});
			}
			return None;
		}
	};

	if let Some((ftype, raw)) = extract_field_and_value(&v) {
		let canonical = canonicalize(&ftype, &raw, punct_re);
		return Some(NormalizedRecord {
			field_type: ftype,
			raw,
			canonical,
		});
	}

	None
}

fn extract_field_and_value(v: &Value) -> Option<(String, String)> {
	match v {
		Value::Object(map) => {
			// Prefer explicit keys
			let mut ftype: Option<String> = None;
			let mut raw: Option<String> = None;

			for (k, val) in map.iter() {
				match k.as_str() {
					"field_type" | "type" | "field" => {
						if let Some(s) = val.as_str() {
							ftype = Some(s.trim().to_lowercase());
						}
					}
					"value" | "v" | "raw" | "val" => {
						if let Some(s) = val.as_str() {
							raw = Some(s.trim().to_string());
						}
					}
					_ => {}
				}
			}

			if ftype.is_none() {
				// Heuristic: if object has one key that looks like a type, and the other is value
				if map.len() == 1 {
					if let Some((_k, val)) = map.iter().next() {
						if let Some(s) = val.as_str() {
							// not enough info
							return None;
						}
					}
				}
			}

			if let (Some(ft), Some(rv)) = (ftype, raw) {
				return Some((ft, rv));
			}

			None
		}
		Value::Array(arr) => {
			if arr.len() >= 2 {
				if let (Some(ft), Some(vv)) = (arr[0].as_str(), arr[1].as_str()) {
					return Some((ft.trim().to_lowercase(), vv.trim().to_string()));
				}
			}
			None
		}
		Value::String(s) => {
			if let Some((ft, val)) = s.split_once(',') {
				return Some((ft.trim().to_lowercase(), val.trim().to_string()));
			}
			None
		}
		_ => None,
	}
}

fn canonicalize(ftype: &str, raw: &str, punct_re: &Regex) -> String {
	match ftype {
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
	}
}

#[cfg(feature = "ingest-tests")]
mod tests {
	use super::*;

	#[test]
	fn normalizes_ndjson_rows() {
		let ndjson = r#"{"field_type":"domain","value":"Example.COM"}
{"field_type":"ip","value":" 192.0.2.1 "}
{"field_type":"hash","value":" ABCDEF123456 "}
{"field_type":"email","value":"USER@EXAMPLE.COM"}
"#;

		let got = normalize_ndjson(ndjson).expect("normalize");
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

	#[test]
	fn supports_array_and_csv_line() {
		let ndjson = "[\"domain\", \"Example.COM\"]\nemail,user@EXAMPLE.COM\n";
		let got = normalize_ndjson(ndjson).expect("normalize");
		assert_eq!(got.len(), 2);
		assert_eq!(got[0].canonical, "example.com");
		assert_eq!(got[1].canonical, "user@example.com");
	}
}
