import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import type { Config } from '@sveltejs/kit';

const config: Config = {
	preprocess: vitePreprocess({ script: true }),
	compilerOptions: { runes: true },
	kit: {
		adapter: adapter({ out: 'build', precompress: true }),
		csp: {
			mode: 'auto',
			directives: {
				'script-src': ['self', 'sha256-ND/+rxieXLFt0ahvvl8pYG+zHvsEAnfLiFL1Nblrz9g='],
			},
		},
	},
};

export default config;
