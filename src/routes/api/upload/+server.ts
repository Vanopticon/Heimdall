import fs from 'fs';
import os from 'os';
import path from 'path';
import { pipeline } from 'stream/promises';
import { Readable } from 'stream';
import type { RequestHandler } from '@sveltejs/kit';

async function streamToFile(webStream: ReadableStream, outPath: string) {
	// Convert web ReadableStream to Node Readable if possible, otherwise create a passthrough
	let nodeStream: NodeJS.ReadableStream;
	if ((Readable as any).fromWeb) {
		nodeStream = (Readable as any).fromWeb(webStream);
	} else {
		const reader = (webStream as any).getReader();
		const pass = new Readable({ read() {} });
		(async () => {
			try {
				while (true) {
					const { done, value } = await reader.read();
					if (done) break;
					pass.push(Buffer.from(value));
				}
				pass.push(null);
			} catch (e) {
				pass.destroy(e as Error);
			}
		})();
		nodeStream = pass;
	}

	const ws = fs.createWriteStream(outPath);
	await pipeline(nodeStream as any, ws);
}

export const POST: RequestHandler = async ({ request, url }) => {
	const contentType = request.headers.get('content-type') || '';
	const filenameParam = url.searchParams.get('filename');
	const filename = filenameParam || `dump-${Date.now()}.bin`;
	const outPath = path.join(os.tmpdir(), filename);

	try {
		if (contentType.startsWith('multipart/form-data')) {
			// Expect a `file` field in the multipart form
			const form = await request.formData();
			const file = form.get('file') as unknown as Blob | null;
			if (!file) {
				return new Response(JSON.stringify({ ok: false, error: 'missing `file` form field' }), { status: 400 });
			}
			await streamToFile(file.stream(), outPath);
		} else {
			const body = request.body;
			if (!body) return new Response(JSON.stringify({ ok: false, error: 'empty request body' }), { status: 400 });
			await streamToFile(body as ReadableStream, outPath);
		}

		const stats = fs.statSync(outPath);
		return new Response(JSON.stringify({ ok: true, path: outPath, size: stats.size }), { status: 201 });
	} catch (err) {
		return new Response(JSON.stringify({ ok: false, error: String(err) }), { status: 500 });
	}
};

export const GET: RequestHandler = async () => {
	return new Response(
		JSON.stringify({
			ok: true,
			message: 'Upload endpoint (POST) — send raw body or multipart/form-data with `file` field',
		}),
		{
			status: 200,
		},
	);
};
