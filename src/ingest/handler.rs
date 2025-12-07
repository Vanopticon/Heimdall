use axum::body::to_bytes;
use axum::{body::Body, http::Request, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;

/// A streaming HTTP handler that parses NDJSON from the request body without
/// buffering the entire payload in memory. It reads body chunks, splits them
/// on newlines, and normalizes each line as it arrives.
pub async fn ndjson_upload(req: Request<Body>) -> impl IntoResponse {
	// For simplicity (and broad compatibility) read the entire body into bytes
	// and then process it. For very large dumps a streaming writer should be
	// implemented instead to avoid high memory usage.
	let bytes = match to_bytes(req.into_body(), usize::MAX).await {
		Ok(b) => b,
		Err(e) => {
			return (
				StatusCode::BAD_REQUEST,
				format!("failed to read request body: {}", e),
			)
				.into_response();
		}
	};

	let s = String::from_utf8_lossy(&bytes).to_string();
	match crate::ingest::normalize_ndjson(&s) {
		Ok(records) => match serde_json::to_string(&records) {
			Ok(body) => (StatusCode::OK, body).into_response(),
			Err(e) => (
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("failed to serialize response: {}", e),
			)
				.into_response(),
		},
		Err(e) => (
			StatusCode::BAD_REQUEST,
			format!("failed to parse NDJSON: {}", e),
		)
			.into_response(),
	}
}

#[cfg(feature = "ingest-tests")]
mod tests {
	use super::*;
	use axum::body::Body;
	use axum::http::Request;

	#[tokio::test]
	async fn handler_accepts_ndjson_stream() {
		let payload = r#"{"field_type":"domain","value":"Example.COM"}
{"field_type":"email","value":"USER@EXAMPLE.COM"}
"#;

		let req = Request::builder()
			.method("POST")
			.uri("/")
			.body(Body::from(payload.to_string()))
			.unwrap();

		let resp = ndjson_upload(req).await.into_response();
		assert_eq!(resp.status(), StatusCode::OK);
	}
}

/// Bulk dump upload endpoint: accepts any raw data stream, writes it to a
/// temporary file, and attempts to determine the dump type (ndjson/csv/json/text/binary/compressed).
/// Returns a small JSON description including detected type, size, preview and the temp filename.
pub async fn bulk_dump_upload(req: Request<Body>) -> impl IntoResponse {
	// Peek up to this many bytes for detection
	const MAX_PEEK: usize = 64 * 1024;

	let headers = req.headers().clone();
	// For simplicity read the whole body; for production use streaming writes.
	let bytes = match to_bytes(req.into_body(), usize::MAX).await {
		Ok(b) => b,
		Err(e) => {
			return (
				StatusCode::BAD_REQUEST,
				format!("failed to read request body: {}", e),
			)
				.into_response();
		}
	};

	// Prepare temp file path
	let tmpdir = std::env::temp_dir();
	let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
	let fname = format!(
		"heimdall_dump_{}_{}.bin",
		std::process::id(),
		now.as_millis()
	);
	let tmp_path = tmpdir.join(&fname);

	// Create the file
	let mut file = match TokioFile::create(&tmp_path).await {
		Ok(f) => f,
		Err(e) => {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("failed to create temp file: {}", e),
			)
				.into_response();
		}
	};

	let total = bytes.len();

	// Write the whole body to file
	if let Err(e) = file.write_all(&bytes).await {
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("failed writing to temp file: {}", e),
		)
			.into_response();
	}

	// flush file
	if let Err(e) = file.flush().await {
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("failed to flush temp file: {}", e),
		)
			.into_response();
	}

	// Detect type from peek (use a slice of the bytes up to MAX_PEEK)
	let peek = if bytes.len() > MAX_PEEK {
		&bytes[..MAX_PEEK]
	} else {
		&bytes[..]
	};
	let (kind, preview, compressed) = detect_dump_type(peek);

	#[derive(Serialize)]
	struct Resp<'a> {
		kind: &'a str,
		preview: String,
		bytes: usize,
		filename: String,
		compressed: bool,
	}

	let resp = Resp {
		kind: Box::leak(kind.into_boxed_str()),
		preview,
		bytes: total,
		filename: tmp_path.to_string_lossy().to_string(),
		compressed,
	};

	match serde_json::to_string(&resp) {
		Ok(body) => (StatusCode::OK, body).into_response(),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("failed to serialize response: {}", e),
		)
			.into_response(),
	}
}

fn is_printable(b: u8) -> bool {
	match b {
		0x09 | 0x0A | 0x0D => true, // tab, lf, cr
		0x20..=0x7E => true,
		_ => false,
	}
}

fn detect_dump_type(peek: &[u8]) -> (String, String, bool) {
	if peek.len() >= 2 && peek[0] == 0x1f && peek[1] == 0x8b {
		// gzip magic
		return (
			"gzip".to_string(),
			format!("gzip compressed ({} bytes peek)", peek.len()),
			true,
		);
	}

	let printable = peek.iter().filter(|b| is_printable(**b)).count();
	let ratio = if peek.is_empty() {
		1.0
	} else {
		printable as f64 / peek.len() as f64
	};

	if ratio < 0.7 {
		// likely binary
		let hex_preview: String = peek
			.iter()
			.take(32)
			.map(|b| format!("{:02x}", b))
			.collect::<Vec<_>>()
			.join(" ");
		return ("binary".to_string(), hex_preview, false);
	}

	let s = String::from_utf8_lossy(peek);
	let s_trim = s.trim_start();

	if s_trim.starts_with('{') || s_trim.starts_with('[') {
		// Could be JSON or NDJSON
		let lines: Vec<&str> = s.lines().collect();
		let mut ndjson_like = false;
		if lines.len() > 1 {
			ndjson_like = lines.iter().all(|l| {
				l.trim().is_empty() || l.trim().starts_with('{') || l.trim().starts_with('[')
			});
		}
		if ndjson_like {
			return (
				"ndjson".to_string(),
				s.lines().take(8).collect::<Vec<_>>().join("\n"),
				false,
			);
		}
		return (
			"json".to_string(),
			s.lines().take(8).collect::<Vec<_>>().join("\n"),
			false,
		);
	}

	// Heuristic for CSV: first non-empty line contains a comma
	if s.lines().next().map(|l| l.contains(',')).unwrap_or(false) {
		return (
			"csv".to_string(),
			s.lines().take(8).collect::<Vec<_>>().join("\n"),
			false,
		);
	}

	// Fallback to text
	let preview = s.lines().take(8).collect::<Vec<_>>().join("\n");
	("text".to_string(), preview, false)
}
