/// Multi-Heimdall synchronization module.
///
/// This module provides continuous synchronization between Heimdall instances
/// using append-only change logs, TLS-secured replication, and configurable
/// merge rules.
///
/// See: `docs/design/features/SYNC-001-Multi-Heimdall-Synchronization.md`
pub mod merge;

pub use merge::{
	EntityVersion, MergeConfig, MergeResolver, MergeRule, MergeStrategy, VersionVector,
};

#[cfg(feature = "unit-tests")]
mod tests {
	#[test]
	fn module_loaded() {
		// Smoke test that module compiles and exports types
		let _ = std::mem::size_of::<crate::sync::MergeStrategy>();
		let _ = std::mem::size_of::<crate::sync::MergeConfig>();
		let _ = std::mem::size_of::<crate::sync::VersionVector>();
	}
}
