use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use std::io::Read;
use zip::ZipArchive;

/// Decompress gzip data and return the uncompressed bytes.
pub fn decompress_gzip<R: Read>(reader: R) -> Result<Vec<u8>> {
	let mut decoder = GzDecoder::new(reader);
	let mut decompressed = Vec::new();
	decoder
		.read_to_end(&mut decompressed)
		.map_err(|e| anyhow!("failed to decompress gzip: {}", e))?;
	Ok(decompressed)
}

/// Extract the first file from a ZIP archive and return its contents.
/// If the archive contains multiple files, only the first one is extracted.
pub fn extract_first_zip_entry<R: Read + std::io::Seek>(reader: R) -> Result<Vec<u8>> {
	let mut archive =
		ZipArchive::new(reader).map_err(|e| anyhow!("failed to open zip archive: {}", e))?;

	if archive.is_empty() {
		return Err(anyhow!("zip archive is empty"));
	}

	// Get the first file
	let mut file = archive
		.by_index(0)
		.map_err(|e| anyhow!("failed to read zip entry: {}", e))?;

	let mut contents = Vec::new();
	file.read_to_end(&mut contents)
		.map_err(|e| anyhow!("failed to read zip file contents: {}", e))?;

	Ok(contents)
}

#[cfg(test)]
mod tests {
	use super::*;
	use flate2::write::GzEncoder;
	use flate2::Compression;
	use std::io::{Cursor, Write};
	use zip::write::{FileOptions, ZipWriter};

	#[test]
	fn decompress_gzip_basic() {
		let test_data = b"Hello, World!";

		// Compress
		let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
		encoder.write_all(test_data).unwrap();
		let compressed = encoder.finish().unwrap();

		// Decompress
		let decompressed = decompress_gzip(Cursor::new(compressed)).expect("decompress");
		assert_eq!(&decompressed, test_data);
	}

	#[test]
	fn extract_zip_basic() {
		let test_data = b"Hello, ZIP!";

		// Create a simple ZIP archive
		let mut zip_buf = Vec::new();
		{
			let mut zip = ZipWriter::new(Cursor::new(&mut zip_buf));
			let options: FileOptions<()> = FileOptions::default();
			zip.start_file("test.txt", options).unwrap();
			zip.write_all(test_data).unwrap();
			zip.finish().unwrap();
		}

		// Extract
		let extracted = extract_first_zip_entry(Cursor::new(zip_buf)).expect("extract");
		assert_eq!(&extracted, test_data);
	}
}
