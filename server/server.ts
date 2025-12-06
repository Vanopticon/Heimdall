import 'dotenv/config';
import { handler } from '../build/handler.js';
import * as openidClient from 'openid-client';
import compression from 'compression';
import cors from 'cors';
import crypto from 'crypto';
import express from 'express';
import fs from 'fs';
import helmet from 'helmet';
import https from 'https';
import morgan from 'morgan';
import path from 'path';
import rateLimit from 'express-rate-limit';
import session from 'express-session';
import os from 'os';

// ===== Environment Variable Checks =====
const requiredEnv = [
	'HMD_OAUTH_DISCOVERY_URL',
	'HMD_OAUTH_H2M_ID',
	'HMD_OAUTH_H2M_SECRET',
	'HMD_OAUTH_M2M_ID',
	'HMD_OAUTH_M2M_SECRET',
	'HMD_TLS_CERT',
	'HMD_TLS_KEY',
];
const missingEnv = requiredEnv.filter((key) => !process.env[key]);
if (missingEnv.length > 0) {
	console.error('Missing required environment variables:', missingEnv.join(', '));
	process.exit(1);
}

// ===== Load Configuration =====
const config = {
	host: process.env.HMD_HOST || os.hostname(),
	port: process.env.HMD_PORT ? parseInt(process.env.HMD_PORT) : 443,
	tls_key: fs.readFileSync(process.env.HMD_TLS_KEY || '/etc/tls/tls.key'),
	tls_cert: fs.readFileSync(process.env.HMD_TLS_CERT || '/etc/tls/tls.crt'),
	oauth_discovery_url: process.env.HMD_OAUTH_DISCOVERY_URL!,
	oauth_h2m_id: process.env.HMD_OAUTH_H2M_ID!,
	oauth_h2m_secret: process.env.HMD_H2M_AUTH_SECRET!,
	OIDC_Scope: process.env.HMD_OIDC_SCOPE || 'openid profile email',
	oauth_m2m_id: process.env.HMD_OAUTH_M2M_ID!,
	oauth_m2m_secret: process.env.HMD_OAUTH_M2M_SECRET!,
	session_secret: crypto.randomBytes(64).toString('hex'),
	cookie_secret: process.env.HMD_COOKIE_SECRET || crypto.randomBytes(32).toString('hex'),
};

// If we generated a cookie secret at startup, also expose it on process.env
// so SvelteKit server-side loads (which run in the same process) can
// access it via `process.env.HMD_COOKIE_SECRET` when decrypting the cookie.
if (!process.env.HMD_COOKIE_SECRET) {
	process.env.HMD_COOKIE_SECRET = config.cookie_secret;
}
process.env['COOKIE_SECRET'] = config.cookie_secret;

// Helper: encrypt JSON using AES-256-GCM with a server secret
function encryptForCookie(obj: unknown) {
	const secret = config.cookie_secret;
	const key = crypto.createHash('sha256').update(secret).digest();
	const iv = crypto.randomBytes(12);
	const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);
	const plaintext = Buffer.from(JSON.stringify(obj), 'utf8');
	const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
	const tag = cipher.getAuthTag();
	// store iv + tag + ciphertext
	return Buffer.concat([iv, tag, ciphertext]).toString('base64');
}

// ===== Express App Setup =====
const app = express();

// ===== Session Management =====
app.use(
	session({
		secret: config.session_secret,
		resave: false,
		saveUninitialized: false,
		cookie: {
			httpOnly: true,
			secure: true,
			sameSite: 'none',
			maxAge: 24 * 60 * 60 * 1000,
		},
	}),
);

// Extend Express types for session and user
declare module 'express-session' {
	interface SessionData {
		user?: any;
		code_verifier?: string;
	}
}
declare global {
	namespace Express {
		interface Request {
			isAuthenticated?: () => boolean;
		}
	}
}

// ===== OIDC Setup =====
let oidcH2MClient: openidClient.Configuration | undefined = undefined;
let oidcM2MClient: openidClient.Configuration | undefined = undefined;
let oidcReady = false;
const callbackUrl = `https://${config.host}:${config.port}/auth/callback`;
(async () => {
	try {
		const oidcM2MIssuer = await openidClient.discovery(
			new URL(config.oauth_discovery_url),
			config.oauth_m2m_id,
			config.oauth_m2m_secret,
		);
		oidcM2MClient = oidcM2MIssuer;

		const oidcH2MIssuer = await openidClient.discovery(
			new URL(config.oauth_discovery_url),
			config.oauth_h2m_id,
			config.oauth_h2m_secret,
		);
		oidcH2MClient = oidcH2MIssuer;
		oidcReady = true;
	} catch (err) {
		console.error('OIDC discovery failed:', err);
		process.exit(1);
	}
})();

// ===== Check and Force authorization =====
app.use((req, res, next) => {
	req.isAuthenticated = function () {
		return !!req.session?.user;
	};
	next();
});

// ===== OIDC Authentication Routes =====
app.get('/auth/login', (req, res) => {
	if (!oidcReady) return res.status(503).send('OIDC not ready');

	const code_verifier = openidClient.randomPKCECodeVerifier();
	openidClient.calculatePKCECodeChallenge(code_verifier).then((code_challenge) => {
		req.session.code_verifier = code_verifier;
		const url = openidClient.buildAuthorizationUrl(oidcH2MClient!, {
			redirect_uri: callbackUrl,
			scope: config.OIDC_Scope,
			response_mode: 'query',
			code_challenge,
			code_challenge_method: 'S256',
		});
		res.redirect(url.href);
	});
});

// ===== OIDC Callback Route =====
app.get('/auth/callback', async (req, res, next) => {
	if (!oidcReady) return res.status(503).send('OIDC not ready');
	try {
		const params = req.query;
		const tokenSet = await openidClient.authorizationCodeGrant(
			oidcH2MClient!,
			new URL(req.originalUrl, callbackUrl),
			{
				pkceCodeVerifier: req.session.code_verifier,
			},
		);
		delete req.session.code_verifier;
		if (!tokenSet.id_token) {
			throw new Error('No ID token returned');
		}

		const userinfoEndpoint = oidcH2MClient!.serverMetadata().userinfo_endpoint;
		const userinfoResponse = await fetch(userinfoEndpoint!, {
			method: 'GET',
			headers: {
				Authorization: `Bearer ${tokenSet.access_token}`,
				Accept: 'application/json',
			},
		});
		if (!userinfoResponse.ok) {
			throw new Error('Failed to fetch userinfo');
		}
		const userinfo = await userinfoResponse.json();
		if (!userinfo || !userinfo.sub) {
			throw new Error('Invalid userinfo');
		}
		req.session.regenerate((err: any) => {
			if (err) return next(err);
			req.session.user = userinfo;

			// Encrypt and set full user JSON in an HttpOnly cookie.
			try {
				const token = encryptForCookie(userinfo);
				res.cookie('hmd_user', token, {
					httpOnly: true,
					secure: true,
					sameSite: 'none',
					maxAge: 24 * 60 * 60 * 1000,
					path: '/',
				});
			} catch (e) {
				console.error('Failed to set encrypted hmd_user cookie:', e);
			}

			res.redirect('/');
		});
	} catch (err) {
		// Log error details for debugging
		if (err instanceof Error) {
			console.error('OIDC callback error:', err.message);
			if ((err as any).error) {
				console.error('OIDC error:', (err as any).error);
			}
			if ((err as any).error_description) {
				console.error('OIDC error description:', (err as any).error_description);
			}
			if ((err as any).response) {
				const response = (err as any).response;
				console.error('OIDC error response status:', response.status);
				console.error('OIDC error response headers:', response.headers);
			}
			console.error(err.stack);
		} else {
			console.error('OIDC callback error:', err);
		}
		next(err);
	}
});

// ===== Logout Route =====
app.get('/auth/logout', (req, res) => {
	req.session.destroy((err) => {
		if (err) {
			console.error('Session destruction error:', err);
		}
		res.clearCookie('connect.sid');
		res.clearCookie('hmd_user');
		res.redirect('/');
	});
});

// ===== Global Redirect for Unauthenticated Users =====
app.use((req, res, next) => {
	if (
		req.path.startsWith('/auth') ||
		req.path.startsWith('/robots.txt') ||
		req.path.startsWith('/static') ||
		req.path.startsWith('/fonts')
	) {
		return next();
	}
	if (!req.isAuthenticated?.()) {
		return res.redirect('/auth/login');
	}
	next();
});

// ===== Hardening: Disable X-Powered-By Header =====
app.disable('x-powered-by');

// ===== Hardening: Helmet =====
app.use(
	helmet({
		contentSecurityPolicy: {
			useDefaults: true,
			directives: {
				defaultSrc: ["'self'"],
				scriptSrc: ["'self'"],
				styleSrc: ["'self'"],
				imgSrc: ["'self'", 'data:'],
				connectSrc: ["'self'", config.host],
				frameAncestors: ["'none'"],
			},
		},
		referrerPolicy: { policy: 'strict-origin-when-cross-origin' },
		crossOriginResourcePolicy: { policy: 'same-origin' },
		crossOriginEmbedderPolicy: true,
		crossOriginOpenerPolicy: { policy: 'same-origin' },
		hsts: {
			maxAge: 60 * 60 * 24,
			includeSubDomains: true,
			preload: true,
		},
	}),
);

// ===== Compression =====
app.use(compression());

// ===== Hardening: CORS =====
app.use(
	cors({
		origin: [`https://${config.host}:${config.port}`],
		methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
		allowedHeaders: ['Content-Type', 'Authorization'],
		credentials: true,
		maxAge: 86400,
	}),
);

// ===== Hardening: Request Size Limits =====
app.use(express.json({ limit: '1mb' }));
app.use(express.urlencoded({ extended: true, limit: '1mb' }));

// ===== Hardening: Rate Limiting =====
app.use(
	rateLimit({
		windowMs: 15 * 60 * 1000, // 15 minute window
		max: 100 * 15 * 60 * 1000, // 100 requests per minute
		standardHeaders: true,
		legacyHeaders: false,
	}),
);

// ===== Disable Caching =====
app.use((req, res, next) => {
	res.set('Cache-Control', 'no-store');
	res.set('Pragma', 'no-cache');
	res.set('Expires', '0');
	res.set('Surrogate-Control', 'no-store');
	next();
});
app.set('etag', false);

// ===== Static Assets =====
app.use(
	'/',
	express.static(path.join(process.cwd(), 'build/client'), {
		maxAge: '1m',
		immutable: true,
	}),
);

// ===== SvelteKit SSR Handler =====
// SvelteKit handler: server-side loads will read the `hmd_user` cookie
// set after successful login. Do not expose full userinfo in headers.
app.use(handler);

// ===== HTTPS Server with TLS 1.3 =====
const tlsOptions: https.ServerOptions = {
	key: config.tls_key,
	cert: config.tls_cert,
	minVersion: 'TLSv1.3',
	requestCert: false,
	rejectUnauthorized: true,
	ciphers: ['TLS_AES_256_GCM_SHA384', 'TLS_CHACHA20_POLY1305_SHA256'].join(':'),
	ecdhCurve: 'X25519:P-256:P-384:P-521',
};

// ===== Start HTTPS Server =====
https.createServer(tlsOptions, app).listen(config.port, config.host, () => {
	console.log(`Heimdall server running with TLS 1.3 on ${config.host}:${config.port}`);
});

// ===== General Error Handler =====
app.use((err: unknown, req: express.Request, res: express.Response, next: express.NextFunction) => {
	console.error(err instanceof Error ? err.stack : err);
	res.status(500).send('Internal Server Error');
});
