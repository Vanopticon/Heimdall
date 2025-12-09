use anyhow::{Result, anyhow};
use log::{error, info};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tokio::time::timeout;

/// Options to control compose start behavior.
#[derive(Debug, Clone)]
pub struct StartOptions {
	pub build: bool,
	pub force_recreate: bool,
	pub timeout_secs: u64,
	pub retries: u8,
	pub workdir: Option<PathBuf>,
}

impl Default for StartOptions {
	fn default() -> Self {
		Self {
			build: false,
			force_recreate: false,
			timeout_secs: 120,
			retries: 2,
			workdir: None,
		}
	}
}

/// Try to detect whether `docker compose` (v2) is available, otherwise fall back
/// to `docker-compose` (v1). Returns the program name and whether the first
/// arg should be `compose` (true for `docker compose`).
async fn detect_compose() -> Option<(String, bool)> {
	// Try `docker compose version`
	if let Ok(mut cmd) = Command::new("docker").arg("compose").arg("version").spawn() {
		if let Ok(status) = cmd.wait().await {
			if status.success() {
				return Some(("docker".to_string(), true));
			}
		}
	}

	// Try `docker-compose --version`
	if let Ok(mut cmd) = Command::new("docker-compose").arg("--version").spawn() {
		if let Ok(status) = cmd.wait().await {
			if status.success() {
				return Some(("docker-compose".to_string(), false));
			}
		}
	}

	None
}

async fn run_command_with_timeout(mut cmd: Command, timeout_secs: u64) -> Result<()> {
	let dur = Duration::from_secs(timeout_secs);
	info!("Running command with timeout: {:?}", cmd);
	let f = cmd.status();
	match timeout(dur, f).await {
		Ok(Ok(status)) => {
			if status.success() {
				Ok(())
			} else {
				Err(anyhow!("command exited with non-zero status"))
			}
		}
		Ok(Err(e)) => Err(anyhow!("failed to spawn command: {}", e)),
		Err(_) => Err(anyhow!("command timed out after {}s", timeout_secs)),
	}
}

// Capture command output with a timeout
async fn run_command_capture(mut cmd: Command, timeout_secs: u64) -> Result<String> {
	let dur = Duration::from_secs(timeout_secs);
	let f = cmd.output();
	match timeout(dur, f).await {
		Ok(Ok(output)) => {
			if output.status.success() {
				Ok(String::from_utf8_lossy(&output.stdout).to_string())
			} else {
				Err(anyhow!("command exited with non-zero status"))
			}
		}
		Ok(Err(e)) => Err(anyhow!("failed to spawn command: {}", e)),
		Err(_) => Err(anyhow!("command timed out after {}s", timeout_secs)),
	}
}

async fn get_db_container_id(
	prog: &str,
	is_docker_compose: bool,
	wd: &Option<std::path::PathBuf>,
) -> Result<Option<String>> {
	let mut cmd = if is_docker_compose {
		let mut c = Command::new(prog);
		c.arg("compose").arg("ps").arg("-q").arg("db");
		c
	} else {
		let mut c = Command::new(prog);
		c.arg("ps").arg("-q").arg("db");
		c
	};
	if let Some(d) = wd {
		cmd.current_dir(d);
	}

	let out = run_command_capture(cmd, 10).await;
	match out {
		Ok(s) => {
			let id = s.trim();
			if id.is_empty() {
				Ok(None)
			} else {
				Ok(Some(id.to_string()))
			}
		}
		Err(_) => Ok(None),
	}
}

async fn inspect_running(container_id: &str) -> Result<bool> {
	let mut cmd = Command::new("docker");
	cmd.arg("inspect")
		.arg("-f")
		.arg("{{.State.Running}}")
		.arg(container_id);
	let out = cmd
		.output()
		.await
		.map_err(|e| anyhow!("failed to inspect container: {}", e))?;
	if !out.status.success() {
		return Ok(false);
	}
	let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
	Ok(s == "true")
}

fn marker_path(wd: &Option<std::path::PathBuf>) -> std::path::PathBuf {
	if let Some(d) = wd {
		d.join(".heimdall_db_started")
	} else {
		std::env::current_dir()
			.unwrap_or_else(|_| std::path::PathBuf::from("."))
			.join(".heimdall_db_started")
	}
}

fn write_marker(wd: &Option<std::path::PathBuf>, container_id: &str) -> Result<()> {
	let p = marker_path(wd);
	std::fs::write(p, container_id).map_err(|e| anyhow!("failed to write marker file: {}", e))
}

/// Start the development DB service defined in `docker-compose.yml` (service `db`).
/// Returns Ok(true) if this call started the DB, Ok(false) if the DB was
/// already running and we did not start it.
pub async fn start_dev_db_with_opts(opts: StartOptions) -> Result<bool> {
	let (prog, is_docker_compose) = detect_compose()
		.await
		.ok_or_else(|| anyhow!("neither 'docker compose' nor 'docker-compose' found in PATH"))?;

	let wd = opts.workdir.or_else(|| env::current_dir().ok());

	// Optionally build first
	if opts.build {
		let mut build_cmd = if is_docker_compose {
			let mut c = Command::new(&prog);
			c.arg("compose").arg("build").arg("db");
			c
		} else {
			let mut c = Command::new(&prog);
			c.arg("build").arg("db");
			c
		};
		if let Some(ref d) = wd {
			build_cmd.current_dir(d);
		}

		run_command_with_timeout(build_cmd, opts.timeout_secs).await?;
	}

	// Before attempting to start, check whether the DB container is already
	// present and running. If it is running, return early and indicate we
	// did not start it. If it's present but stopped, attempt to start it.
	// We'll return a boolean to indicate whether we started the DB (true)
	// or it was already running (false).
	//
	// Note: The public wrapper `start_dev_db()` will preserve the previous
	// signature by returning a Result<bool> value.

	// Use the module-level helper functions (run_command_capture, get_db_container_id,
	// inspect_running, marker_path, write_marker) declared above.

	// Check current state
	if let Ok(Some(id)) = get_db_container_id(&prog, is_docker_compose, &wd).await {
		if let Ok(true) = inspect_running(&id).await {
			info!("dev DB container already running (id={})", id);
			return Ok(false);
		}
		// Container exists but not running — fall through and attempt to start it
	}

	// Prepare up command (attempt to start or recreate as before)
	let mut attempts = 0u8;
	let mut last_err = None;
	while attempts <= opts.retries {
		let mut up_cmd = if is_docker_compose {
			let mut c = Command::new(&prog);
			c.arg("compose").arg("up").arg("-d").arg("db");
			if opts.force_recreate {
				c.arg("--force-recreate");
			}
			c
		} else {
			let mut c = Command::new(&prog);
			c.arg("up").arg("-d").arg("db");
			if opts.force_recreate {
				c.arg("--force-recreate");
			}
			c
		};

		if let Some(ref d) = wd {
			up_cmd.current_dir(d);
		}

		match run_command_with_timeout(up_cmd, opts.timeout_secs).await {
			Ok(()) => {
				info!("docker compose up succeeded");
				// After a successful up, capture the container id and write a marker
				if let Ok(Some(id)) = get_db_container_id(&prog, is_docker_compose, &wd).await {
					if let Err(e) = write_marker(&wd, &id) {
						error!("failed to write marker file: {}", e);
					}
				}
				return Ok(true);
			}
			Err(e) => {
				error!("attempt {}: docker compose up failed: {}", attempts + 1, e);
				last_err = Some(e);
				attempts += 1;
				sleep(Duration::from_secs(2)).await;
			}
		}
	}

	Err(last_err.unwrap_or_else(|| anyhow!("docker compose up failed after retries")))
}

/// Stop (bring down) the development compose stack, but only if this tool
/// started the DB container (determined by the presence of a marker file).
pub async fn stop_dev_db() -> Result<()> {
	let (prog, is_docker_compose) = detect_compose()
		.await
		.ok_or_else(|| anyhow!("neither 'docker compose' nor 'docker-compose' found in PATH"))?;

	let wd = env::current_dir().ok();

	let marker = marker_path(&wd);
	if !marker.exists() {
		info!("marker file not found; will not stop DB that was not started by this tool");
		return Ok(());
	}

	// Read container id from marker if possible
	let container_id = std::fs::read_to_string(&marker)
		.ok()
		.map(|s| s.trim().to_string());

	// If marker exists but the container is already gone, remove marker and exit
	if let Some(ref id) = container_id {
		if let Ok(false) = inspect_running(id).await {
			let _ = std::fs::remove_file(&marker);
			info!(
				"marker existed but container {} not running; removed marker",
				id
			);
			return Ok(());
		}
	}

	// Prepare down/stop command — prefer stopping/removing only the 'db' service
	let mut cmd = if is_docker_compose {
		let mut c = Command::new(&prog);
		c.arg("compose").arg("stop").arg("db");
		c
	} else {
		let mut c = Command::new(&prog);
		c.arg("stop").arg("db");
		c
	};

	if let Some(ref d) = wd {
		cmd.current_dir(d);
	}

	// Attempt to stop the service
	match run_command_with_timeout(cmd, 60).await {
		Ok(()) => {
			// Remove the container instance (rm -f) to ensure a clean state
			let mut rm_cmd = if is_docker_compose {
				let mut c = Command::new(&prog);
				c.arg("compose").arg("rm").arg("-f").arg("db");
				c
			} else {
				let mut c = Command::new(&prog);
				c.arg("rm").arg("-f").arg("db");
				c
			};
			if let Some(ref d) = wd {
				rm_cmd.current_dir(d);
			}
			let _ = run_command_with_timeout(rm_cmd, 60).await;

			let _ = std::fs::remove_file(&marker);
			info!("dev DB stopped and marker removed");
			Ok(())
		}
		Err(e) => Err(e),
	}
}

/// Convenience wrapper to maintain compatibility with previous API.
pub async fn start_dev_db() -> Result<bool> {
	start_dev_db_with_opts(StartOptions::default()).await
}

#[cfg(feature = "devops-tests")]
mod tests {
	use super::*;

	// These tests are limited to compile-time and non-Docker environments.
	#[tokio::test]
	async fn detect_no_crash() {
		// detect_compose should not panic; it may return None if docker isn't installed.
		let _ = detect_compose().await;
	}
}
