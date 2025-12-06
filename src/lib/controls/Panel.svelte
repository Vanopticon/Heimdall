<script lang="ts">
	import { page } from '$app/state';

	let { color = 'primary', title, children } = $props();
</script>

<div class="panel {color} border-black border-2 border-solid rounded-l-4xl pl-1 pb-1 flex flex-col content-top">
	<h2 class="pl-4">{title}</h2>
	<div class="grow surface rounded-l-4xl p-2 pl-4 pt-4 pb-4">
		{@render children?.()}
	</div>
</div>
