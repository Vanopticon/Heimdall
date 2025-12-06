<script lang="ts">
	import { ActionType } from '$lib/common.js';
	import type { Snippet } from 'svelte';

	let {
		actionType = ActionType.Positive,
		disabled = false,
		ariaLabel = '',
		children,
	}: { actionType: ActionType; disabled: boolean; ariaLabel: string; children: Snippet } = $props();
</script>

<button
	class="{actionType} button shrink border-4 border-l-slate-600 border-b-slate-600 border-t-slate-400 border-r-slate-400 p-1 m-2 rounded-full"
	{disabled}
	aria-label={ariaLabel}
	aria-disabled={disabled}
>
	{@render children?.()}
</button>
