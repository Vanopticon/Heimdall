<script lang="ts">
	import { page } from '$app/state';
	import WarningPanel from '$lib/controls/WarningPanel.svelte';
	let { title = 'Error' } = $props();

	// This error is disabled because the title needs to be set only once, when the component is constructed
	// svelte-ignore state_referenced_locally
	page.data.title = title || 'Error';
</script>

<WarningPanel {title}>
	<h1>An unexpected error has occurred.</h1>
	<p class="text-2xl">{page.error?.message}</p>
</WarningPanel>
