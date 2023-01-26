<script lang="ts">
	import { WalletMultiButton } from '@svelte-on-solana/wallet-adapter-ui';
	import { walletStore } from '@svelte-on-solana/wallet-adapter-core';
</script>

<h1>anchor-counter</h1>

<div class="address">
	<WalletMultiButton />
</div>

{#if $walletStore?.connected}
	<p>wallet connected</p>
{:else}
	<p>wallet not connected</p>
{/if}
