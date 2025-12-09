use anyhow::Result;

/// Detected format type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatType {
	Csv,
	Tsv,
	Ndjson,
	Json,
	Xlsx,
	Gzip,
	Zip,
	Binary,
	Text,
}

impl FormatType {
	pub fn as_str(&self) -> &str {
		match self {
			FormatType::Csv => "csv",
			FormatType::Tsv => "tsv",
			FormatType::Ndjson => "ndjson",
			FormatType::Json => "json",
			FormatType::Xlsx => "xlsx",
			FormatType::Gzip => "gzip",
			FormatType::Zip => "zip",
			FormatType::Binary => "binary",
			FormatType::Text => "text",
		}
	}

	/// Parse a format hint string into a FormatType
	pub fn from_hint(hint: &str) -> Option<Self> {
		match hint.to_lowercase().as_str() {
			"csv" => Some(FormatType::Csv),
			"tsv" => Some(FormatType::Tsv),
			"ndjson" | "jsonl" => Some(FormatType::Ndjson),
			"json" => Some(FormatType::Json),
			"xlsx" | "excel" => Some(FormatType::Xlsx),
			"gzip" | "gz" => Some(FormatType::Gzip),
			"zip" => Some(FormatType::Zip),
			_ => None,
		}
	}
}

/// Detect the format of data from a sample (peek buffer).
/// Returns the detected format and whether the data is compressed.
pub fn detect_format(peek: &[u8], hint: Option<&str>) -> Result<(FormatType, bool)> {
	// If a hint is provided, trust it
	if let Some(h) = hint {
		if let Some(format) = FormatType::from_hint(h) {
			let compressed = matches!(format, FormatType::Gzip | FormatType::Zip);
			return Ok((format, compressed));
		}
	}

	// Check for gzip magic bytes
	if peek.len() >= 2 && peek[0] == 0x1f && peek[1] == 0x8b {
		return Ok((FormatType::Gzip, true));
	}

	// Check for ZIP magic bytes (PK)
	if peek.len() >= 4 && peek[0] == 0x50 && peek[1] == 0x4b && peek[2] == 0x03 && peek[3] == 0x04
	{
		return Ok((FormatType::Zip, true));
	}

	// Check for Excel (XLSX) magic bytes - XLSX is a ZIP file with specific structure
	// Note: XLSX files are ZIP archives. To distinguish XLSX from generic ZIP, we would
	// need to extract and check for Office Open XML structure ([Content_Types].xml, etc.).
	// For now, we treat all ZIP files as generic ZIP and rely on user hints to specify
	// "xlsx" format when uploading Excel files. This is documented as a known limitation.
	if peek.len() >= 4 && peek[0] == 0x50 && peek[1] == 0x4b {
		return Ok((FormatType::Zip, true));
	}

	// Check if data is mostly printable (text-based)
	let printable_count = peek.iter().filter(|&&b| is_printable(b)).count();
	let printable_ratio = if peek.is_empty() {
		1.0
	} else {
		printable_count as f64 / peek.len() as f64
	};

	if printable_ratio < 0.7 {
		// Likely binary
		return Ok((FormatType::Binary, false));
	}

	// Text-based format detection
	let text = String::from_utf8_lossy(peek);
	let text_trim = text.trim_start();

	// Check for JSON/NDJSON
	if text_trim.starts_with('{') || text_trim.starts_with('[') {
		let lines: Vec<&str> = text.lines().collect();
		if lines.len() > 1 {
			// Check if it's NDJSON (multiple lines starting with { or [)
			let ndjson_like = lines.iter().all(|l| {
				l.trim().is_empty() || l.trim().starts_with('{') || l.trim().starts_with('[')
			});
			if ndjson_like {
				return Ok((FormatType::Ndjson, false));
			}
		}
		return Ok((FormatType::Json, false));
	}

	// Check for CSV/TSV
	let first_line = text.lines().next().unwrap_or("");
	if first_line.contains(',') {
		return Ok((FormatType::Csv, false));
	}
	if first_line.contains('\t') {
		return Ok((FormatType::Tsv, false));
	}

	// Default to text
	Ok((FormatType::Text, false))
}

fn is_printable(b: u8) -> bool {
	match b {
		0x09 | 0x0A | 0x0D => true, // tab, lf, cr
		0x20..=0x7E => true,        // printable ASCII
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn detect_gzip() {
		let peek = [0x1f_u8, 0x8b_u8, 0x08, 0x00, 0x00];
		let (format, compressed) = detect_format(&peek, None).expect("detect");
		assert_eq!(format, FormatType::Gzip);
		assert!(compressed);
	}

	#[test]
	fn detect_zip() {
		let peek = [0x50_u8, 0x4b, 0x03, 0x04, 0x00];
		let (format, compressed) = detect_format(&peek, None).expect("detect");
		assert_eq!(format, FormatType::Zip);
		assert!(compressed);
	}

	#[test]
	fn detect_ndjson() {
		let ndjson = b"{\"a\":1}\n{\"b\":2}\n";
		let (format, compressed) = detect_format(ndjson, None).expect("detect");
		assert_eq!(format, FormatType::Ndjson);
		assert!(!compressed);
	}

	#[test]
	fn detect_json() {
		let json = b"{\"a\":1, \"b\":2}\n";
		let (format, compressed) = detect_format(json, None).expect("detect");
		assert_eq!(format, FormatType::Json);
		assert!(!compressed);
	}

	#[test]
	fn detect_csv() {
		let csv = b"col1,col2\n1,2\n";
		let (format, compressed) = detect_format(csv, None).expect("detect");
		assert_eq!(format, FormatType::Csv);
		assert!(!compressed);
	}

	#[test]
	fn detect_tsv() {
		let tsv = b"col1\tcol2\n1\t2\n";
		let (format, compressed) = detect_format(tsv, None).expect("detect");
		assert_eq!(format, FormatType::Tsv);
		assert!(!compressed);
	}

	#[test]
	fn detect_with_hint() {
		let data = b"some data";
		let (format, _) = detect_format(data, Some("csv")).expect("detect");
		assert_eq!(format, FormatType::Csv);

		let (format, _) = detect_format(data, Some("ndjson")).expect("detect");
		assert_eq!(format, FormatType::Ndjson);
	}

	#[test]
	fn detect_binary() {
		let binary = vec![0xff_u8, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
		let (format, compressed) = detect_format(&binary, None).expect("detect");
		assert_eq!(format, FormatType::Binary);
		assert!(!compressed);
	}
}
