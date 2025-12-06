<script lang="ts">
	import Panel from './Panel.svelte';
	let { title, children } = $props();
</script>

<Panel {title} color="error">
	{@render children?.()}
</Panel>
