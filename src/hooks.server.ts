import type { Handle } from '@sveltejs/kit';
import crypto from 'crypto';

declare namespace App {
	interface Locals {
		user?: any | null;
	}
}

function decryptFromCookie(token: string) {
	const secret = process.env['HMD_COOKIE_SECRET'];
	if (!secret) throw new Error('HMD_COOKIE_SECRET not set');
	const key = crypto.createHash('sha256').update(secret).digest();
	const buf = Buffer.from(token, 'base64');
	if (buf.length < 28) throw new Error('invalid cookie payload');
	const iv = buf.slice(0, 12);
	const tag = buf.slice(12, 28);
	const ciphertext = buf.slice(28);
	const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv);
	decipher.setAuthTag(tag);
	const plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]);
	return JSON.parse(plaintext.toString('utf8'));
}

export const handle: Handle = async ({ event, resolve }) => {
	try {
		const raw = event.cookies.get('hmd_user');
		if (raw) {
			try {
				const user = decryptFromCookie(raw);
				event.locals.user = user;
			} catch (err) {
				// If cookie is invalid or tampered, clear it so we don't keep failing.
				console.error('Failed to decrypt hmd_user cookie in hook:', err);
				event.cookies.delete('hmd_user', { path: '/' });
				event.locals.user = null;
			}
		} else {
			event.locals.user = null;
		}
	} catch (err) {
		console.error('Unexpected error in auth hook:', err);
		event.locals.user = null;
	}

	const response = await resolve(event);
	return response;
};
