use clap::{Parser, Subcommand};
use vanopticon_heimdall::{config, devops, run};

#[derive(Parser)]
#[command(name = "heimdall", about = "Heimdall - ETL and normalization hub")]
struct Cli {
	#[command(subcommand)]
	command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
	/// Start the development Postgres+AGE container (docker compose up -d db)
	StartDb {
		/// Build the image before bringing up the service
		#[arg(long)]
		build: bool,
		/// Force recreate containers
		#[arg(long)]
		force_recreate: bool,
		/// Timeout in seconds for docker commands
		#[arg(long, default_value_t = 120)]
		timeout: u64,
		/// Number of retry attempts on failure
		#[arg(long, default_value_t = 2u8)]
		retries: u8,
		/// Optional working directory where docker-compose.yml lives
		#[arg(long)]
		workdir: Option<String>,
	},
	/// Stop the development container (docker compose down)
	StopDb,
	/// Run the application (default)
	Run,
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();

	match cli.command.unwrap_or(Commands::Run) {
		Commands::StartDb {
			build,
			force_recreate,
			timeout,
			retries,
			workdir,
		} => {
			let mut opts = devops::docker_manager::StartOptions::default();
			opts.build = build;
			opts.force_recreate = force_recreate;
			opts.timeout_secs = timeout;
			opts.retries = retries;
			opts.workdir = workdir.map(|s| std::path::PathBuf::from(s));

			match devops::docker_manager::start_dev_db_with_opts(opts).await {
				Ok(true) => println!("Postgres+AGE dev container started (heimdall will stop it)."),
				Ok(false) => println!("Postgres+AGE dev container already running; not started."),
				Err(e) => eprintln!("Failed to start dev DB: {}", e),
			}
		}
		Commands::StopDb => match devops::stop_dev_db().await {
			Ok(()) => println!("Postgres+AGE dev container stopped."),
			Err(e) => eprintln!("Failed to stop dev DB: {}", e),
		},
		Commands::Run => {
			match config::load() {
				Ok(settings) => println!(
					"Loaded settings: host={} port={}",
					settings.host, settings.port
				),
				Err(e) => eprintln!("Warning: failed to load config: {}", e),
			}

			run().await;
		}
	}
}
