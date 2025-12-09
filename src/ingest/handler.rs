use axum::{body::Body, extract::State, http::Request, http::StatusCode, response::IntoResponse};
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use serde::Serialize;
use std::fs::File as StdFile;
use std::io::{BufRead, BufReader, Read};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::error::TrySendError;

/// A streaming HTTP handler that parses NDJSON from the request body without
/// buffering the entire payload in memory. It reads body chunks, splits them
/// on newlines, and normalizes each line as it arrives.
pub async fn ndjson_upload(
	State(state): State<crate::state::AppState>,
	req: Request<Body>,
) -> impl IntoResponse {
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
	// Enqueue normalized records to the background batcher. If the
	// persistence channel is full or closed we'll fall back to performing
	// the persistence synchronously to avoid data loss.
	let sender = state.persist_sender.clone();
	for rec in &records {
		// Only persist sanitized/normalized properties. Do NOT store the
		// original raw value here; keep raw payloads in temp files for
		// offline analysis if required.
		let props = serde_json::json!({
			"field_type": rec.field_type,
		});

		let job = crate::persist::PersistJob {
			label: "FieldValue".to_string(),
			key: rec.canonical.clone(),
			props: props.clone(),
		};

		match crate::persist::submit_job(&sender, job.clone()) {
			Ok(()) => {}
			Err(TrySendError::Full(returned)) | Err(TrySendError::Closed(returned)) => {
				// Channel unavailable; persist synchronously using the
				// normalized/sanitized job we received back from the channel.
				if let Err(e) = state
					.repo
					.merge_entity(&returned.label, &returned.key, &returned.props)
					.await
				{
					return (
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("failed to persist record: {}", e),
					)
						.into_response();
				}
			}
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

		// Build AppState for test
		use std::sync::Arc;
		use tokio::sync::mpsc;

		struct DummyRepo;
		#[async_trait::async_trait]
		impl crate::age_client::AgeRepo for DummyRepo {
			async fn merge_entity(
				&self,
				_label: &str,
				_key: &str,
				_props: &serde_json::Value,
			) -> anyhow::Result<()> {
				Ok(())
			}

			async fn ping(&self) -> anyhow::Result<()> {
				Ok(())
			}

			async fn merge_batch(
				&self,
				_items: &[(String, String, serde_json::Value)],
			) -> anyhow::Result<()> {
				Ok(())
			}
		}

		let (tx, _rx) = mpsc::channel(16);
		let repo: Arc<dyn crate::age_client::AgeRepo> = Arc::new(DummyRepo);
		let app_state = crate::state::AppState {
			repo,
			persist_sender: tx,
		};

		let resp = ndjson_upload(axum::extract::State(app_state), req)
			.await
			.into_response();
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

		// Build a minimal AppState for the handler's State extractor
		use std::sync::Arc;
		use tokio::sync::mpsc;

		struct DummyRepo;
		#[async_trait::async_trait]
		impl crate::age_client::AgeRepo for DummyRepo {
			async fn merge_entity(
				&self,
				_label: &str,
				_key: &str,
				_props: &serde_json::Value,
			) -> anyhow::Result<()> {
				Ok(())
			}

			async fn ping(&self) -> anyhow::Result<()> {
				Ok(())
			}

			async fn merge_batch(
				&self,
				_items: &[(String, String, serde_json::Value)],
			) -> anyhow::Result<()> {
				Ok(())
			}
		}

		let (tx, _rx) = mpsc::channel(16);
		let repo: Arc<dyn crate::age_client::AgeRepo> = Arc::new(DummyRepo);
		let app_state = crate::state::AppState {
			repo,
			persist_sender: tx,
		};

		let resp = bulk_dump_upload(axum::extract::State(app_state), req)
			.await
			.into_response();
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

		// Build AppState for test
		use std::sync::Arc;
		use tokio::sync::mpsc;

		struct DummyRepo;
		#[async_trait::async_trait]
		impl crate::age_client::AgeRepo for DummyRepo {
			async fn merge_entity(
				&self,
				_label: &str,
				_key: &str,
				_props: &serde_json::Value,
			) -> anyhow::Result<()> {
				Ok(())
			}

			async fn ping(&self) -> anyhow::Result<()> {
				Ok(())
			}

			async fn merge_batch(
				&self,
				_items: &[(String, String, serde_json::Value)],
			) -> anyhow::Result<()> {
				Ok(())
			}
		}

		let (tx, _rx) = mpsc::channel(16);
		let repo: Arc<dyn crate::age_client::AgeRepo> = Arc::new(DummyRepo);
		let app_state = crate::state::AppState {
			repo,
			persist_sender: tx,
		};

		let resp = ndjson_upload(axum::extract::State(app_state), req)
			.await
			.into_response();
		assert_eq!(resp.status(), StatusCode::OK);

		// Optionally parse and assert the returned JSON contains normalized entries
		// but for now ensure status OK and that handler didn't error on chunk boundaries.
	}
}

/// Bulk dump upload endpoint: accepts any raw data stream, writes it to a
/// temporary file, and attempts to determine the dump type (ndjson/csv/json/text/binary/compressed).
/// Returns a small JSON description including detected type, size, preview and the temp filename.
pub async fn bulk_dump_upload(
	State(state): State<crate::state::AppState>,
	req: Request<Body>,
) -> impl IntoResponse {
	// Peek up to this many bytes for detection
	const MAX_PEEK: usize = 64 * 1024;

	// Note: headers are intentionally not used here, kept in earlier
	// iterations for potential content-type based detection. Remove the
	// clone to avoid an unused-variable warning.

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

	// Optionally auto-process the uploaded dump in the background. This behavior
	// is gated by `HMD_AUTO_PROCESS_BULK` env var (default: disabled). When
	// enabled Heimdall will spawn a background task that will attempt to parse
	// and normalize supported formats (ndjson/json arrays) and enqueue
	// sanitized `PersistJob`s into the persistence batcher.
	if std::env::var("HMD_AUTO_PROCESS_BULK")
		.map(|v| v == "1" || v.to_lowercase() == "true")
		.unwrap_or(false)
	{
		let sender = state.persist_sender.clone();
		let path = tmp_path.clone();
		let compressed_flag = compressed;

		// Spawn a background task to process the file without blocking the
		// request/response lifecycle.
		tokio::spawn(async move {
			// Delegate to a blocking worker for file IO and decompression.
			let _ =
				tokio::task::spawn_blocking(move || {
					// Re-open the file for reading
					if let Ok(f) = StdFile::open(&path) {
						// Create reader (decompress if gzip)
						let reader: Box<dyn Read> = if compressed_flag {
							Box::new(GzDecoder::new(f))
						} else {
							Box::new(f)
						};

						let buf = BufReader::new(reader);
						let punct_re = regex::Regex::new(r"^[\W_]+|[\W_]+$").unwrap();

						for line_res in buf.lines() {
							match line_res {
								Ok(line) => {
									if let Some(rec) =
										crate::ingest::normalize_ndjson_line(&line, &punct_re)
									{
										let props =
											serde_json::json!({ "field_type": rec.field_type });
										let job = crate::persist::PersistJob {
											label: "FieldValue".to_string(),
											key: rec.canonical.clone(),
											props: props.clone(),
										};

										// Best-effort submission: try a few times before dropping the job.
										let mut attempts = 0;
										loop {
											match crate::persist::submit_job(&sender, job.clone()) {
						 Ok(()) => break,
						 Err(e) => match e {
							tokio::sync::mpsc::error::TrySendError::Full(_ret) => {
							 attempts += 1;
							 if attempts > 5 {
								eprintln!("persist channel full; dropping job {}", job.key);
								break;
							 }
							 std::thread::sleep(std::time::Duration::from_millis(200));
							}
							tokio::sync::mpsc::error::TrySendError::Closed(_ret) => {
							 eprintln!("persist channel closed; dropping job {}", job.key);
							 break;
							}
						 },
						}
										}
									}
								}
								Err(e) => {
									eprintln!("error reading dump file {}: {}", path.display(), e);
								}
							}
						}
					}
					// Note: we intentionally do not delete uploaded files here; retention
					// and archival policies should be handled externally or by a separate
					// cleanup worker.
				})
				.await;
		});
	}

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

/// Multipart upload endpoint: accepts multipart/form-data with streaming file uploads.
/// Detects format, routes to appropriate parser, and normalizes records incrementally.
pub async fn multipart_upload(
	State(state): State<crate::state::AppState>,
	mut multipart: axum::extract::Multipart,
) -> impl IntoResponse {
	use crate::ingest::format_detection::{detect_format, FormatType};
	use crate::ingest::parsers;
	use std::io::Cursor;

	// Process each field in the multipart request
	let mut format_hint: Option<String> = None;
	let mut file_data: Option<Vec<u8>> = None;

	while let Some(field) = multipart.next_field().await.transpose() {
		let field = match field {
			Ok(f) => f,
			Err(e) => {
				return (
					StatusCode::BAD_REQUEST,
					format!("failed to read multipart field: {}", e),
				)
					.into_response();
			}
		};

		let name = field.name().unwrap_or("").to_string();

		if name == "format" {
			// User-provided format hint
			let hint_bytes = match field.bytes().await {
				Ok(b) => b,
				Err(e) => {
					return (
						StatusCode::BAD_REQUEST,
						format!("failed to read format hint: {}", e),
					)
						.into_response();
				}
			};
			format_hint = Some(String::from_utf8_lossy(&hint_bytes).to_string());
		} else if name == "file" {
			// File data
			let bytes = match field.bytes().await {
				Ok(b) => b,
				Err(e) => {
					return (
						StatusCode::BAD_REQUEST,
						format!("failed to read file data: {}", e),
					)
						.into_response();
				}
			};
			file_data = Some(bytes.to_vec());
		}
	}

	let data = match file_data {
		Some(d) => d,
		None => {
			return (StatusCode::BAD_REQUEST, "no file data provided").into_response();
		}
	};

	// Detect format from peek
	const PEEK_SIZE: usize = 64 * 1024;
	let peek = if data.len() > PEEK_SIZE {
		&data[..PEEK_SIZE]
	} else {
		&data
	};

	let (format, compressed) = match detect_format(peek, format_hint.as_deref()) {
		Ok(f) => f,
		Err(e) => {
			return (
				StatusCode::BAD_REQUEST,
				format!("failed to detect format: {}", e),
			)
				.into_response();
		}
	};

	// Handle compressed data first
	let decompressed_data = if compressed {
		match format {
			FormatType::Gzip => match parsers::decompress_gzip(Cursor::new(&data)) {
				Ok(d) => d,
				Err(e) => {
					return (
						StatusCode::BAD_REQUEST,
						format!("failed to decompress gzip: {}", e),
					)
						.into_response();
				}
			},
			FormatType::Zip => match parsers::extract_first_zip_entry(Cursor::new(&data)) {
				Ok(d) => d,
				Err(e) => {
					return (
						StatusCode::BAD_REQUEST,
						format!("failed to extract zip: {}", e),
					)
						.into_response();
				}
			},
			_ => data.clone(),
		}
	} else {
		data.clone()
	};

	// Parse based on detected format
	let parse_result = match format {
		FormatType::Csv => parsers::parse_csv_stream(Cursor::new(&decompressed_data), None),
		FormatType::Tsv => parsers::parse_csv_stream(Cursor::new(&decompressed_data), Some(b'\t')),
		FormatType::Ndjson | FormatType::Json => {
			parsers::parse_ndjson_stream(Cursor::new(&decompressed_data))
		}
		FormatType::Xlsx => parsers::parse_xlsx_stream(Cursor::new(&decompressed_data)),
		_ => {
			return (
				StatusCode::BAD_REQUEST,
				format!("unsupported format: {}", format.as_str()),
			)
				.into_response();
		}
	};

	let records = match parse_result {
		Ok(r) => r,
		Err(e) => {
			return (
				StatusCode::BAD_REQUEST,
				format!("failed to parse data: {}", e),
			)
				.into_response();
		}
	};

	// Persist records using the background batcher
	let sender = state.persist_sender.clone();
	for rec in &records {
		let props = serde_json::json!({
			"field_type": rec.field_type,
		});

		let job = crate::persist::PersistJob {
			label: "FieldValue".to_string(),
			key: rec.canonical.clone(),
			props: props.clone(),
		};

		match crate::persist::submit_job(&sender, job.clone()) {
			Ok(()) => {}
			Err(tokio::sync::mpsc::error::TrySendError::Full(returned))
			| Err(tokio::sync::mpsc::error::TrySendError::Closed(returned)) => {
				// Channel unavailable; persist synchronously
				if let Err(e) = state
					.repo
					.merge_entity(&returned.label, &returned.key, &returned.props)
					.await
				{
					return (
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("failed to persist record: {}", e),
					)
						.into_response();
				}
			}
		}
	}

	#[derive(Serialize)]
	struct Response {
		format: String,
		compressed: bool,
		records_count: usize,
	}

	let resp = Response {
		format: format.as_str().to_string(),
		compressed,
		records_count: records.len(),
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

#[cfg(test)]
mod detect_tests {
	use super::*;

	#[test]
	fn detect_gzip() {
		let peek = [0x1f_u8, 0x8b_u8, 0x08, 0x00, 0x00];
		let (kind, _preview, compressed) = detect_dump_type(&peek);
		assert_eq!(kind, "gzip");
		assert!(compressed);
	}

	#[test]
	fn detect_binary() {
		// low printable ratio
		let peek = vec![
			0xff_u8, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
		];
		let (kind, _preview, compressed) = detect_dump_type(&peek);
		assert_eq!(kind, "binary");
		assert!(!compressed);
	}

	#[test]
	fn detect_ndjson_vs_json() {
		let ndjson_peek = b"{\"a\":1}\n{\"b\":2}\n";
		let (kind_nd, _preview, _c) = detect_dump_type(ndjson_peek);
		assert_eq!(kind_nd, "ndjson");

		let json_peek = b"{\"a\":1, \"b\":2}\n";
		let (kind_json, _preview, _c2) = detect_dump_type(json_peek);
		assert_eq!(kind_json, "json");
	}

	#[test]
	fn detect_csv_and_text() {
		let csv = b"col1,col2\n1,2\n";
		let (kind_csv, _p, _c) = detect_dump_type(csv);
		assert_eq!(kind_csv, "csv");

		let text = b"hello world\nthis is text\n";
		let (kind_text, _p2, _c2) = detect_dump_type(text);
		assert_eq!(kind_text, "text");
	}
}
