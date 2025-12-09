pub mod bulk_normalizer;
pub mod format_detection;
pub mod handler;
pub mod ndjson;
pub mod parsers;

#[cfg(test)]
pub mod test_utils;

pub use bulk_normalizer::NormalizedRecord;
pub use format_detection::{detect_format, FormatType};
pub use handler::{bulk_dump_upload, multipart_upload, ndjson_upload};
pub use ndjson::{normalize_ndjson, normalize_ndjson_line};

#[cfg(feature = "unit-tests")]
mod tests {
	#[test]
	fn module_loaded() {
		// smoke test that module compiles and exports `NormalizedRecord`
		let _ = std::mem::size_of::<crate::ingest::NormalizedRecord>();
	}
}
