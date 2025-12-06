/// <reference types="svelte" />
/// <reference types="vite/client" />

declare namespace App {
	interface Locals {
		user?: any | null;
	}
	interface PageData {
		title?: string;
	}
}
