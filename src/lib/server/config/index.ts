import crypto from 'crypto';
import fs from 'fs';
import os from 'os';
import path from 'path';

/**
 * Settings interface defining all configuration options for Heimdall.
 * Configuration is read from HMD_* environment variables with optional file-based overrides.
 */
export interface Settings {
	/** Server host binding (defaults to hostname) */
	host: string;
	/** Server port (defaults to 443) */
	port: number;
	/** Path to TLS private key file */
	tls_key_path: string;
	/** Path to TLS certificate file */
	tls_cert_path: string;
	/** OAuth/OIDC discovery URL */
	oauth_discovery_url: string;
	/** OAuth client ID for human-to-machine (H2M) flow */
	oauth_h2m_id: string;
	/** OAuth client secret for H2M flow */
	oauth_h2m_secret: string;
	/** OAuth client ID for machine-to-machine (M2M) flow */
	oauth_m2m_id: string;
	/** OAuth client secret for M2M flow */
	oauth_m2m_secret: string;
	/** OIDC scope (defaults to "openid profile email") */
	oidc_scope: string;
	/** Database connection URL (optional, can use PG* vars instead) */
	database_url?: string;
	/** Cookie encryption secret (generated if not provided) */
	cookie_secret: string;
	/** AGE graph name (defaults to "dumps_graph" for consistency with ageClient.ts) */
	age_graph: string;
}

/**
 * Configuration loading options
 */
export interface LoadOptions {
	/** Optional path to JSON config file to merge with env vars */
	configFile?: string;
	/** Whether to validate required fields (defaults to true) */
	validate?: boolean;
}

/**
 * Required environment variables that must be present
 */
const REQUIRED_ENV_VARS = [
	'HMD_OAUTH_DISCOVERY_URL',
	'HMD_OAUTH_H2M_ID',
	'HMD_OAUTH_H2M_SECRET',
	'HMD_OAUTH_M2M_ID',
	'HMD_OAUTH_M2M_SECRET',
	'HMD_TLS_CERT',
	'HMD_TLS_KEY',
];

/**
 * Load configuration from environment variables and optional config file.
 *
 * Environment variables follow the HMD_* naming convention:
 * - HMD_HOST: Server host (default: hostname)
 * - HMD_PORT: Server port (default: 443)
 * - HMD_TLS_KEY: Path to TLS private key
 * - HMD_TLS_CERT: Path to TLS certificate
 * - HMD_OAUTH_DISCOVERY_URL: OAuth/OIDC discovery endpoint
 * - HMD_OAUTH_H2M_ID: OAuth client ID for human-to-machine flow
 * - HMD_OAUTH_H2M_SECRET: OAuth client secret for H2M
 * - HMD_OAUTH_M2M_ID: OAuth client ID for machine-to-machine flow
 * - HMD_OAUTH_M2M_SECRET: OAuth client secret for M2M
 * - HMD_OIDC_SCOPE: OIDC scope (default: "openid profile email")
 * - HMD_DATABASE_URL: Database connection URL
 * - HMD_COOKIE_SECRET: Cookie encryption secret (generated if not provided)
 * - HMD_AGE_GRAPH: AGE graph name (default: "heimdall_graph")
 *
 * @param options Configuration loading options
 * @returns Settings object with all configuration values
 * @throws Error if required configuration is missing
 */
export function load(options: LoadOptions = {}): Settings {
	const { configFile, validate = true } = options;

	// Load optional config file
	let fileConfig: Partial<Settings> = {};
	if (configFile) {
		try {
			const configPath = path.resolve(configFile);
			const fileContent = fs.readFileSync(configPath, 'utf-8');
			fileConfig = JSON.parse(fileContent);
		} catch (error) {
			throw new Error(
				`Failed to load config file "${configFile}": ${error instanceof Error ? error.message : String(error)}`,
			);
		}
	}

	// Validate required environment variables
	if (validate) {
		const missing = REQUIRED_ENV_VARS.filter((key) => !process.env[key]);
		if (missing.length > 0) {
			throw new Error(`Missing required environment variables: ${missing.join(', ')}`);
		}
	}

	// Generate cookie secret if not provided
	const cookieSecret =
		process.env.HMD_COOKIE_SECRET || fileConfig.cookie_secret || crypto.randomBytes(32).toString('hex');

	// Parse and validate port number
	let port = 443; // default
	if (process.env.HMD_PORT) {
		const parsedPort = parseInt(process.env.HMD_PORT, 10);
		if (isNaN(parsedPort) || parsedPort < 1 || parsedPort > 65535) {
			throw new Error(
				`Invalid HMD_PORT value: "${process.env.HMD_PORT}". Port must be a number between 1 and 65535.`,
			);
		}
		port = parsedPort;
	} else if (fileConfig.port !== undefined) {
		port = fileConfig.port;
	}

	// Build settings object with env vars taking precedence over file config
	const settings: Settings = {
		host: process.env.HMD_HOST || fileConfig.host || os.hostname(),
		port,
		tls_key_path: process.env.HMD_TLS_KEY || fileConfig.tls_key_path || '/etc/tls/tls.key',
		tls_cert_path: process.env.HMD_TLS_CERT || fileConfig.tls_cert_path || '/etc/tls/tls.crt',
		oauth_discovery_url: process.env.HMD_OAUTH_DISCOVERY_URL || fileConfig.oauth_discovery_url || '',
		oauth_h2m_id: process.env.HMD_OAUTH_H2M_ID || fileConfig.oauth_h2m_id || '',
		oauth_h2m_secret: process.env.HMD_OAUTH_H2M_SECRET || fileConfig.oauth_h2m_secret || '',
		oauth_m2m_id: process.env.HMD_OAUTH_M2M_ID || fileConfig.oauth_m2m_id || '',
		oauth_m2m_secret: process.env.HMD_OAUTH_M2M_SECRET || fileConfig.oauth_m2m_secret || '',
		oidc_scope: process.env.HMD_OIDC_SCOPE || fileConfig.oidc_scope || 'openid profile email',
		database_url: process.env.HMD_DATABASE_URL || fileConfig.database_url,
		cookie_secret: cookieSecret,
		age_graph: process.env.HMD_AGE_GRAPH || fileConfig.age_graph || 'dumps_graph',
	};

	// Validate TLS paths exist if validate is enabled
	if (validate) {
		if (!fs.existsSync(settings.tls_key_path)) {
			throw new Error(`TLS key file not found: ${settings.tls_key_path}`);
		}
		if (!fs.existsSync(settings.tls_cert_path)) {
			throw new Error(`TLS certificate file not found: ${settings.tls_cert_path}`);
		}
	}

	return settings;
}

/**
 * Load TLS key and certificate content from paths in settings.
 * This is a helper function for reading the actual file contents.
 *
 * @param settings Settings object with TLS paths
 * @returns Object with key and cert Buffer contents
 */
export function loadTLS(settings: Settings): { key: Buffer; cert: Buffer } {
	try {
		return {
			key: fs.readFileSync(settings.tls_key_path),
			cert: fs.readFileSync(settings.tls_cert_path),
		};
	} catch (error) {
		throw new Error(`Failed to read TLS files: ${error instanceof Error ? error.message : String(error)}`);
	}
}
