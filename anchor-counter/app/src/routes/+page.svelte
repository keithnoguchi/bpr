<script lang="ts">
	import { walletStore } from '@svelte-on-solana/wallet-adapter-core';
	import { workSpace } from '@svelte-on-solana/wallet-adapter-anchor';
</script>

{#if $walletStore?.connected}
	<p>Your wallet address is {$walletStore.publicKey}</p>
	<p>Counter's program ID is {$workSpace.program.programId}</p>
{/if}
