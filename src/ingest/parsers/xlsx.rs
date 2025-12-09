use anyhow::{anyhow, Result};
use calamine::{open_workbook_auto_from_rs, Reader};
use regex::Regex;
use std::io::{Read, Seek};

use crate::ingest::NormalizedRecord;

/// Stream-parse Excel (XLSX) data from a reader and emit normalized records.
/// Expects the first row to be headers (field_type, value) and subsequent rows to contain data.
pub fn parse_xlsx_stream<R: Read + Seek + Clone>(reader: R) -> Result<Vec<NormalizedRecord>> {
	let mut workbook = open_workbook_auto_from_rs(reader)
		.map_err(|e| anyhow!("failed to open Excel workbook: {}", e))?;

	// Get the first worksheet
	let sheet_names = workbook.sheet_names().to_vec();
	if sheet_names.is_empty() {
		return Err(anyhow!("Excel workbook has no sheets"));
	}

	let sheet_name = &sheet_names[0];
	let range = workbook
		.worksheet_range(sheet_name)
		.map_err(|e| anyhow!("failed to read worksheet: {}", e))?;

	let mut out = Vec::new();
	let punct_re = Regex::new(r"^[\W_]+|[\W_]+$").unwrap();

	// Skip the header row (index 0) and process data rows
	let mut rows = range.rows();
	let _header = rows.next(); // Skip header

	for row in rows {
		if row.len() < 2 {
			continue;
		}

		let ftype = row[0].to_string().trim().to_lowercase();
		let raw = row[1].to_string().trim().to_string();

		if ftype.is_empty() || raw.is_empty() {
			continue;
		}

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
	#[test]
	fn xlsx_parser_compiles() {
		// Smoke test to ensure the module compiles
		assert!(true);
	}
}
