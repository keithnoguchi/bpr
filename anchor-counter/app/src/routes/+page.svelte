<script lang="ts">
	import { walletStore } from '@svelte-on-solana/wallet-adapter-core';
</script>

{#if $walletStore?.connected}
	<p>yes, you have a wallet</p>
{:else}
	<p>oh, no</p>
{/if}
