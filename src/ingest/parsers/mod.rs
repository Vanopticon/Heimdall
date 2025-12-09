pub mod compressed;
pub mod csv;
pub mod ndjson;
pub mod xlsx;

pub use compressed::{decompress_gzip, extract_first_zip_entry};
pub use csv::parse_csv_stream;
pub use ndjson::parse_ndjson_stream;
pub use xlsx::parse_xlsx_stream;
