use axum::body::to_bytes;
use axum::{body::Body, http::Request, http::StatusCode, response::IntoResponse};
use futures_util::StreamExt;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;

/// A streaming HTTP handler that parses NDJSON from the request body without
/// buffering the entire payload in memory. It reads body chunks, splits them
/// on newlines, and normalizes each line as it arrives.
pub async fn ndjson_upload(req: Request<Body>) -> impl IntoResponse {
	// Stream the request body and process NDJSON line-by-line to avoid
	// buffering very large payloads in memory. We collect complete lines
	// by scanning for '\n' in the incoming byte stream and hand each line
	// to the permissive normalizer.

	use regex::Regex;

	let mut stream = req.into_body().into_data_stream();
	let mut buf: Vec<u8> = Vec::new();
	let mut records: Vec<crate::ingest::NormalizedRecord> = Vec::new();
	let punct_re = Regex::new(r"^[\W_]+|[\W_]+$").unwrap();

	while let Some(chunk_res) = stream.next().await {
		match chunk_res {
			Ok(bytes_chunk) => {
				let chunk = bytes_chunk.as_ref();
				buf.extend_from_slice(chunk);

				// Extract complete lines (terminated by '\n') and normalize each.
				while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
					let mut line_bytes = buf.drain(..=pos).collect::<Vec<u8>>();
					// remove trailing LF
					if line_bytes.ends_with(&[b'\n']) {
						line_bytes.pop();
					}
					// Optional: remove trailing CR if present
					if line_bytes.ends_with(&[b'\r']) {
						line_bytes.pop();
					}

					let line = String::from_utf8_lossy(&line_bytes);
					if let Some(rec) = crate::ingest::normalize_ndjson_line(&line, &punct_re) {
						records.push(rec);
					}
				}

				// Safety: guard against pathological single-line sizes
				if buf.len() > 10 * 1024 * 1024 {
					return (
						StatusCode::BAD_REQUEST,
						"line too long or streaming malformed",
					)
						.into_response();
				}
			}
			Err(e) => {
				return (
					StatusCode::BAD_REQUEST,
					format!("failed to read request body: {}", e),
				)
					.into_response();
			}
		}
	}

	// Process any trailing data after stream end
	if !buf.is_empty() {
		let line = String::from_utf8_lossy(&buf);
		if let Some(rec) = crate::ingest::normalize_ndjson_line(&line, &punct_re) {
			records.push(rec);
		}
	}

	match serde_json::to_string(&records) {
		Ok(body) => (StatusCode::OK, body).into_response(),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("failed to serialize response: {}", e),
		)
			.into_response(),
	}
}

#[cfg(feature = "ingest-tests")]
mod tests {
	use super::*;
	use axum::body::Body;
	use axum::http::Request;
	use futures_util::stream;

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

	#[tokio::test]
	async fn bulk_dump_streaming_writes_file() {
		// Simulate a chunked upload by building a TryStream of Bytes
		let s = stream::iter(vec![
			Ok::<_, std::io::Error>(b"first line\n".to_vec()),
			Ok::<_, std::io::Error>(b"second line\n".to_vec()),
		]);

		let body = Body::from_stream(s);

		let req = Request::builder()
			.method("POST")
			.uri("/")
			.body(body)
			.unwrap();

		let resp = bulk_dump_upload(req).await.into_response();
		assert_eq!(resp.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn ndjson_streaming_chunked_lines() {
		// Simulate a NDJSON upload where a single JSON object is split across chunks
		let s = stream::iter(vec![
			Ok::<_, std::io::Error>(b"{".to_vec()),
			Ok::<_, std::io::Error>(b"\"field_type\":\"domain\",\"value\":\"Exa".to_vec()),
			Ok::<_, std::io::Error>(b"mple.COM\"}\n{".to_vec()),
			Ok::<_, std::io::Error>(
				b"\"field_type\":\"email\",\"value\":\"USER@EXAMPLE.COM\"}\n".to_vec(),
			),
		]);

		let body = Body::from_stream(s);

		let req = Request::builder()
			.method("POST")
			.uri("/")
			.body(body)
			.unwrap();

		let resp = ndjson_upload(req).await.into_response();
		assert_eq!(resp.status(), StatusCode::OK);

		// Optionally parse and assert the returned JSON contains normalized entries
		// but for now ensure status OK and that handler didn't error on chunk boundaries.
	}
}

/// Bulk dump upload endpoint: accepts any raw data stream, writes it to a
/// temporary file, and attempts to determine the dump type (ndjson/csv/json/text/binary/compressed).
/// Returns a small JSON description including detected type, size, preview and the temp filename.
pub async fn bulk_dump_upload(req: Request<Body>) -> impl IntoResponse {
	// Peek up to this many bytes for detection
	const MAX_PEEK: usize = 64 * 1024;

	let headers = req.headers().clone();

	// Prepare temp file path early so we can stream into it
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

	// Stream the request body to the temp file while collecting a small peek buffer
	let mut stream = req.into_body().into_data_stream();
	let mut total: usize = 0;
	let mut peek_buf: Vec<u8> = Vec::with_capacity(std::cmp::min(MAX_PEEK, 4096));

	while let Some(chunk_res) = stream.next().await {
		match chunk_res {
			Ok(bytes_chunk) => {
				let chunk = bytes_chunk.as_ref();
				total = total.saturating_add(chunk.len());

				// Fill the peek buffer until full
				if peek_buf.len() < MAX_PEEK {
					let remaining = MAX_PEEK - peek_buf.len();
					let take = std::cmp::min(remaining, chunk.len());
					peek_buf.extend_from_slice(&chunk[..take]);
				}

				if let Err(e) = file.write_all(chunk).await {
					return (
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("failed writing to temp file: {}", e),
					)
						.into_response();
				}
			}
			Err(e) => {
				return (
					StatusCode::BAD_REQUEST,
					format!("failed to read request body chunk: {}", e),
				)
					.into_response();
			}
		}
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
	let peek = &peek_buf[..];
	let (kind, preview, compressed) = detect_dump_type(peek);

	#[derive(Serialize)]
	struct Resp {
		kind: String,
		preview: String,
		bytes: usize,
		filename: String,
		compressed: bool,
	}

	let resp = Resp {
		kind,
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
