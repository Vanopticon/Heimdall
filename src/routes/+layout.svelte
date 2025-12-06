<script lang="ts">
	import { page } from '$app/state';
	import Button from '$lib/controls/Button.svelte';
	import { ActionType } from '$lib/common.js';
	import '../styles/app.css';

	let { children } = $props<{
		data?: { title?: string; user?: { name?: string } };
		children?: () => any;
	}>();

	let title = page.data.title || '';
</script>

<svelte:head>
	<title>{title} - Vanopticon: Heimdall</title>
</svelte:head>

<header class="flex rounded-bl-4xl bg-blue-900 text-amber-200 pb-2 mb-1">
	<img src="/images/Heimdall.svg" alt="Heimdall Logo" class="h-12 m-2 mb-0 ml-4" />
	<div class="flex flex-col grow">
		<div class="flex rounded-bl-4xl bg-black text-white grow mt-0 w-full pt-2 pl-4">
			<h1 class="ml-4 grow">{page.data.title}</h1>
			{#if page.data?.['user']?.name}
				<div class="mr-4 self-center">{page.data?.['user']?.name}</div>
			{/if}
			<Button actionType={ActionType.Destructive} disabled={false} ariaLabel="Log Out">➩🚪</Button>
		</div>
	</div>
</header>
<div>
	{@render children?.()}
</div>
SurfaceColor
