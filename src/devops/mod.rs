pub mod docker_manager;

pub use docker_manager::{start_dev_db, stop_dev_db};

#[cfg(feature = "devops-tests")]
mod tests {
	use super::*;

	#[test]
	fn mod_compiles() {
		// smoke compile test
		let _ = 1 + 1;
	}
}
