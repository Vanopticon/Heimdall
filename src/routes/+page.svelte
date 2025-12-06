<script lang="ts">
	import { page } from '$app/state';
	import Panel from '$lib/controls/Panel.svelte';

	page.data.title = 'Dashboard';
</script>

<Panel title="Demo">Content goes here</Panel>
