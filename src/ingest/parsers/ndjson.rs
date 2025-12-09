use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};

use crate::ingest::NormalizedRecord;

/// Stream-parse NDJSON (newline-delimited JSON) from a reader and emit normalized records.
pub fn parse_ndjson_stream<R: Read>(reader: R) -> Result<Vec<NormalizedRecord>> {
	let buf_reader = BufReader::new(reader);
	let mut out = Vec::new();
	let punct_re = Regex::new(r"^[\W_]+|[\W_]+$").unwrap();

	for line_result in buf_reader.lines() {
		let line = line_result?;
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

fn extract_field_and_value(v: &Value) -> Option<(String, String)> {
	match v {
		Value::Object(map) => {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_ndjson_basic() {
		let ndjson = r#"{"field_type":"domain","value":"Example.COM"}
{"field_type":"ip","value":"192.0.2.1"}
"#;
		let records = parse_ndjson_stream(ndjson.as_bytes()).expect("parse ndjson");
		assert_eq!(records.len(), 2);
		assert_eq!(records[0].canonical, "example.com");
		assert_eq!(records[1].canonical, "192.0.2.1");
	}

	#[test]
	fn parse_ndjson_array_format() {
		let ndjson = "[\"domain\", \"Example.COM\"]\n[\"ip\", \"192.0.2.1\"]\n";
		let records = parse_ndjson_stream(ndjson.as_bytes()).expect("parse ndjson");
		assert_eq!(records.len(), 2);
		assert_eq!(records[0].canonical, "example.com");
		assert_eq!(records[1].canonical, "192.0.2.1");
	}

	#[test]
	fn parse_ndjson_csv_fallback() {
		let ndjson = "domain,Example.COM\nip,192.0.2.1\n";
		let records = parse_ndjson_stream(ndjson.as_bytes()).expect("parse ndjson");
		assert_eq!(records.len(), 2);
		assert_eq!(records[0].canonical, "example.com");
		assert_eq!(records[1].canonical, "192.0.2.1");
	}
}
