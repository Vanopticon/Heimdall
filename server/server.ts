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

// ===== Environment Variable Checks =====
const requiredEnv = [
	'VOH_HOST',
	'VOH_PORT',
	'VOH_TLS_KEY',
	'VOH_TLS_CERT',
	'VOH_OAUTH_DISCOVERY_URL',
	'VOH_OAUTH_AUTH_ID',
	'VOH_OAUTH_AUTH_SECRET',
	'VOH_OAUTH_SCOPES',
];
const missingEnv = requiredEnv.filter((key) => !process.env[key]);
if (missingEnv.length > 0) {
	console.error('Missing required environment variables:', missingEnv.join(', '));
	process.exit(1);
}

// ===== Express App Setup =====
const app = express();

const sessionSecret =
	process.env.NODE_ENV === 'development' ?
		'8675309deadbeefcafe11223344556677889900aabbccddeeff001122334455667788'
	:	crypto.randomBytes(64).toString('hex');

app.use(
	session({
		secret: sessionSecret,
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
let oidcClient: any;
let oidcReady = false;
const callbackUrl = `https://${process.env.VOH_HOST}:${process.env.VOH_PORT}/auth/callback`;
(async () => {
	try {
		const oidcIssuer = await openidClient.discovery(
			new URL(process.env.VOH_OAUTH_DISCOVERY_URL!),
			process.env.VOH_OAUTH_AUTH_ID!,
			process.env.VOH_OAUTH_AUTH_SECRET!,
		);
		oidcClient = oidcIssuer;
		oidcReady = true;
	} catch (err) {
		console.error('OIDC discovery failed:', err);
		process.exit(1);
	}
})();

app.use((req, res, next) => {
	req.isAuthenticated = function () {
		return !!req.session?.user;
	};
	next();
});

app.get('/auth/login', (req, res) => {
	if (!oidcReady) return res.status(503).send('OIDC not ready');
	// PKCE code_verifier and code_challenge
	const code_verifier = openidClient.randomPKCECodeVerifier();
	openidClient.calculatePKCECodeChallenge(code_verifier).then((code_challenge) => {
		req.session.code_verifier = code_verifier;
		const url = openidClient.buildAuthorizationUrl(oidcClient, {
			redirect_uri: callbackUrl,
			scope: process.env.VOH_OAUTH_SCOPES || 'openid profile email',
			response_mode: 'query',
			code_challenge,
			code_challenge_method: 'S256',
		});
		res.redirect(url.href);
	});
});

app.get('/auth/callback', async (req, res, next) => {
	if (!oidcReady) return res.status(503).send('OIDC not ready');
	try {
		const params = req.query;
		// Use PKCE code_verifier from session
		const tokenSet = await openidClient.authorizationCodeGrant(oidcClient, new URL(req.originalUrl, callbackUrl), {
			pkceCodeVerifier: req.session.code_verifier,
		});
		// Remove code_verifier from session after use
		delete req.session.code_verifier;
		// Validate ID token and userinfo
		if (!tokenSet.id_token) {
			throw new Error('No ID token returned');
		}
		// Fetch userinfo manually
		const userinfoEndpoint = oidcClient.serverMetadata().userinfo_endpoint;
		const userinfoResponse = await fetch(userinfoEndpoint, {
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
		req.session.regenerate((err) => {
			if (err) return next(err);
			req.session.user = userinfo;
			// Always redirect to root after login
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

app.get('/auth/logout', (req, res) => {
	req.session.destroy((err) => {
		if (err) {
			console.error('Session destruction error:', err);
		}
		res.clearCookie('connect.sid');
		res.redirect('/');
	});
});

// ===== Global Redirect for Unauthenticated Users =====
app.use((req, res, next) => {
	if (
		req.path.startsWith('/auth') ||
		req.path.startsWith('/robots.txt') ||
		req.path.startsWith('/static') ||
		req.path.startsWith('/fonts') ||
		req.path.startsWith('/images')
	) {
		return next();
	}
	if (!req.isAuthenticated?.()) {
		return res.redirect('/auth/login');
	}
	next();
});
// ===== Disable X-Powered-By Header =====
app.disable('x-powered-by');

// ===== Security: Helmet =====
app.use(
	helmet({
		contentSecurityPolicy: {
			useDefaults: true,
			directives: {
				defaultSrc: ["'self'"],
				scriptSrc: ["'self'"],
				styleSrc: ["'self'", "'unsafe-inline'"],
				imgSrc: ["'self'", 'data:'],
				connectSrc: ["'self'", process.env.VOH_HOST || 'https://127.0.0.1'],
				frameAncestors: ["'none'"],
			},
		},
		referrerPolicy: { policy: 'strict-origin-when-cross-origin' },
		crossOriginResourcePolicy: { policy: 'same-origin' },
		crossOriginEmbedderPolicy: true,
		crossOriginOpenerPolicy: { policy: 'same-origin' },
		hsts: {
			maxAge: 63072000,
			includeSubDomains: true,
			preload: true,
		},
	}),
);

// ===== Compression =====
app.use(compression());

// ===== Logging to stdout/stderr =====
app.use(morgan('combined'));

// ===== CORS =====
app.use(
	cors({
		origin: [process.env.VOH_HOST || 'https://127.0.0.1'],
		methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
		allowedHeaders: ['Content-Type', 'Authorization'],
		credentials: true,
		maxAge: 86400,
	}),
);

// ===== Request Size Limits =====
app.use(express.json({ limit: '1mb' }));
app.use(express.urlencoded({ extended: true, limit: '1mb' }));

// ===== Rate Limiting =====
app.use(
	rateLimit({
		windowMs: 15 * 60 * 1000,
		max: 100,
		standardHeaders: true,
		legacyHeaders: false,
	}),
);

// ===== No Cache =====
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
		maxAge: '1y',
		immutable: true,
	}),
);

// ===== SvelteKit SSR Handler =====
app.use(handler);

// ===== HTTPS Server with TLS 1.3 =====
const keyPath = process.env.VOH_TLS_KEY || '/etc/tls/tls.key';
const certPath = process.env.VOH_TLS_CERT || '/etc/tls/tls.crt';
const key = fs.readFileSync(keyPath);
const cert = fs.readFileSync(certPath);

const host = process.env.VOH_HOST || '127.0.0.1';
const port = process.env.VOH_PORT ? parseInt(process.env.VOH_PORT) : 443;

const tlsOptions: https.ServerOptions = {
	key,
	cert,
	minVersion: 'TLSv1.3',
	requestCert: false,
	rejectUnauthorized: true,
	ciphers: ['TLS_AES_256_GCM_SHA384', 'TLS_CHACHA20_POLY1305_SHA256'].join(':'),
	ecdhCurve: 'X25519:P-256:P-384:P-521',
};

https.createServer(tlsOptions, app).listen(port, host, () => {
	console.log(`VOH server running with TLS 1.3 on ${host}:${port}`);
});

// ===== Basic Error Handler =====
app.use((err: unknown, req: express.Request, res: express.Response, next: express.NextFunction) => {
	console.error(err instanceof Error ? err.stack : err);
	res.status(500).send('Internal Server Error');
});
