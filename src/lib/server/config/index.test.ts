import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { load, loadTLS } from './index.js';
import fs from 'fs';
import os from 'os';
import path from 'path';

describe('config module', () => {
	let originalEnv: NodeJS.ProcessEnv;
	let tempDir: string;
	let tempKeyPath: string;
	let tempCertPath: string;

	beforeEach(() => {
		// Save original environment
		originalEnv = { ...process.env };

		// Create temporary directory and dummy TLS files for testing
		tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'heimdall-config-test-'));
		tempKeyPath = path.join(tempDir, 'test.key');
		tempCertPath = path.join(tempDir, 'test.crt');
		fs.writeFileSync(tempKeyPath, 'dummy-key-content');
		fs.writeFileSync(tempCertPath, 'dummy-cert-content');

		// Set up minimal required env vars for testing
		process.env.HMD_TLS_KEY = tempKeyPath;
		process.env.HMD_TLS_CERT = tempCertPath;
		process.env.HMD_OAUTH_DISCOVERY_URL = 'https://example.com/.well-known/openid-configuration';
		process.env.HMD_OAUTH_H2M_ID = 'test-h2m-client-id';
		process.env.HMD_OAUTH_H2M_SECRET = 'test-h2m-secret';
		process.env.HMD_OAUTH_M2M_ID = 'test-m2m-client-id';
		process.env.HMD_OAUTH_M2M_SECRET = 'test-m2m-secret';
	});

	afterEach(() => {
		// Restore original environment
		process.env = originalEnv;

		// Clean up temporary files
		try {
			fs.rmSync(tempDir, { recursive: true, force: true });
		} catch (error) {
			// Ignore cleanup errors
		}
	});

	describe('load()', () => {
		it('should load configuration from environment variables', () => {
			const settings = load();

			expect(settings.oauth_discovery_url).toBe('https://example.com/.well-known/openid-configuration');
			expect(settings.oauth_h2m_id).toBe('test-h2m-client-id');
			expect(settings.oauth_h2m_secret).toBe('test-h2m-secret');
			expect(settings.oauth_m2m_id).toBe('test-m2m-client-id');
			expect(settings.oauth_m2m_secret).toBe('test-m2m-secret');
			expect(settings.tls_key_path).toBe(tempKeyPath);
			expect(settings.tls_cert_path).toBe(tempCertPath);
		});

		it('should use default values when optional env vars are not set', () => {
			const settings = load();

			expect(settings.host).toBe(os.hostname());
			expect(settings.port).toBe(443);
			expect(settings.oidc_scope).toBe('openid profile email');
			expect(settings.age_graph).toBe('heimdall_graph');
		});

		it('should override defaults with environment variables', () => {
			process.env.HMD_HOST = 'custom-host';
			process.env.HMD_PORT = '8443';
			process.env.HMD_OIDC_SCOPE = 'custom scope';
			process.env.HMD_AGE_GRAPH = 'custom_graph';

			const settings = load();

			expect(settings.host).toBe('custom-host');
			expect(settings.port).toBe(8443);
			expect(settings.oidc_scope).toBe('custom scope');
			expect(settings.age_graph).toBe('custom_graph');
		});

		it('should generate cookie_secret if not provided', () => {
			const settings = load();

			expect(settings.cookie_secret).toBeDefined();
			expect(settings.cookie_secret.length).toBeGreaterThan(0);
		});

		it('should use provided cookie_secret from env', () => {
			process.env.HMD_COOKIE_SECRET = 'my-secret-cookie-key';

			const settings = load();

			expect(settings.cookie_secret).toBe('my-secret-cookie-key');
		});

		it('should throw error when required env vars are missing', () => {
			delete process.env.HMD_OAUTH_DISCOVERY_URL;

			expect(() => load()).toThrow('Missing required environment variables');
		});

		it('should skip validation when validate option is false', () => {
			delete process.env.HMD_OAUTH_DISCOVERY_URL;

			expect(() => load({ validate: false })).not.toThrow();
		});

		it('should throw error when TLS key file does not exist', () => {
			process.env.HMD_TLS_KEY = '/nonexistent/path/tls.key';

			expect(() => load()).toThrow('TLS key file not found');
		});

		it('should throw error when TLS cert file does not exist', () => {
			process.env.HMD_TLS_CERT = '/nonexistent/path/tls.crt';

			expect(() => load()).toThrow('TLS certificate file not found');
		});

		it('should load config from file when configFile option is provided', () => {
			const configPath = path.join(tempDir, 'config.json');
			const configData = {
				host: 'file-host',
				port: 9443,
				oidc_scope: 'file scope',
			};
			fs.writeFileSync(configPath, JSON.stringify(configData));

			const settings = load({ configFile: configPath });

			// Env vars should take precedence
			expect(settings.oauth_discovery_url).toBe('https://example.com/.well-known/openid-configuration');
			// File config should be used for values not in env
			expect(settings.host).toBe('file-host');
			expect(settings.port).toBe(9443);
			expect(settings.oidc_scope).toBe('file scope');
		});

		it('should prioritize env vars over config file', () => {
			process.env.HMD_HOST = 'env-host';
			process.env.HMD_PORT = '7443';

			const configPath = path.join(tempDir, 'config.json');
			const configData = {
				host: 'file-host',
				port: 9443,
			};
			fs.writeFileSync(configPath, JSON.stringify(configData));

			const settings = load({ configFile: configPath });

			expect(settings.host).toBe('env-host');
			expect(settings.port).toBe(7443);
		});

		it('should throw error when config file does not exist', () => {
			expect(() => load({ configFile: '/nonexistent/config.json' })).toThrow(
				'Failed to load config file',
			);
		});

		it('should throw error when config file contains invalid JSON', () => {
			const configPath = path.join(tempDir, 'invalid.json');
			fs.writeFileSync(configPath, 'invalid json content');

			expect(() => load({ configFile: configPath })).toThrow('Failed to load config file');
		});
	});

	describe('loadTLS()', () => {
		it('should load TLS key and cert content', () => {
			const settings = load();
			const tls = loadTLS(settings);

			expect(tls.key.toString()).toBe('dummy-key-content');
			expect(tls.cert.toString()).toBe('dummy-cert-content');
		});

		it('should throw error when TLS files cannot be read', () => {
			const settings = load({ validate: false });
			settings.tls_key_path = '/nonexistent/tls.key';

			expect(() => loadTLS(settings)).toThrow('Failed to read TLS files');
		});
	});
});
