import crypto from 'crypto';

function decryptFromCookie(token: string) {
	const secret = process.env['HMD_COOKIE_SECRET'];
	if (!secret) throw new Error('HMD_COOKIE_SECRET not set');
	const key = crypto.createHash('sha256').update(secret).digest();
	const buf = Buffer.from(token, 'base64');
	const iv = buf.slice(0, 12);
	const tag = buf.slice(12, 28);
	const ciphertext = buf.slice(28);
	const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv);
	decipher.setAuthTag(tag);
	const plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]);
	return JSON.parse(plaintext.toString('utf8'));
}

export const load = async (event: { locals: App.Locals }) => {
	try {
		const user = event.locals.user ?? null;
		return { user };
	} catch (err) {
		console.error('Error reading user from locals in +layout.server.ts:', err);
		return {};
	}
};
