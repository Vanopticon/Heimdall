use anyhow::Result;
use regex::Regex;
use std::io::Read;

use crate::ingest::NormalizedRecord;

/// Stream-parse CSV data from a reader and emit normalized records incrementally.
/// Supports CSV and TSV (tab-separated) formats by auto-detecting the delimiter.
pub fn parse_csv_stream<R: Read>(
	reader: R,
	delimiter: Option<u8>,
) -> Result<Vec<NormalizedRecord>> {
	let delim = delimiter.unwrap_or(b',');

	let mut rdr = csv::ReaderBuilder::new()
		.has_headers(true)
		.delimiter(delim)
		.trim(csv::Trim::All)
		.from_reader(reader);

	let mut out = Vec::new();
	let punct_re = Regex::new(r"^[\W_]+|[\W_]+$").unwrap();

	for result in rdr.records() {
		let record = result?;
		if record.len() < 2 {
			continue;
		}

		let ftype = record.get(0).unwrap_or("").to_lowercase();
		let raw = record.get(1).unwrap_or("").to_string();

		let canonical = canonicalize(&ftype, &raw, &punct_re);

		out.push(NormalizedRecord {
			field_type: ftype,
			raw,
			canonical,
		});
	}

	Ok(out)
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
	fn parse_csv_basic() {
		let csv = "field_type,value\ndomain,Example.COM\nip,192.0.2.1\n";
		let records = parse_csv_stream(csv.as_bytes(), None).expect("parse csv");
		assert_eq!(records.len(), 2);
		assert_eq!(records[0].canonical, "example.com");
		assert_eq!(records[1].canonical, "192.0.2.1");
	}

	#[test]
	fn parse_tsv_basic() {
		let tsv = "field_type\tvalue\ndomain\tExample.COM\nip\t192.0.2.1\n";
		let records = parse_csv_stream(tsv.as_bytes(), Some(b'\t')).expect("parse tsv");
		assert_eq!(records.len(), 2);
		assert_eq!(records[0].canonical, "example.com");
		assert_eq!(records[1].canonical, "192.0.2.1");
	}
}
