import type { PageServerLoad } from './$types.js';

export const load: PageServerLoad = async () => {
	// Compute server-side so the page doesn't require client JS for this value.
	return {
		now: new Date().toLocaleString(),
	};
};
