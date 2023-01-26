<script lang="ts">
	import { walletStore } from '@svelte-on-solana/wallet-adapter-core';
	import { workSpace } from '@svelte-on-solana/wallet-adapter-anchor';

	let counter;

	$: console.log('counter: ', counter);

	async function initialize() {
		try {
			const tx = await $workSpace
				.program
				.methods
				.initialize()
				.accounts({
					state: $workSpace.baseAccount.publicKey,
					authority: $walletStore.publicKey,
				})
				.signers([$workSpace.baseAccount])
				.rpc();
			console.log("Init done", tx);

			const account = await $workSpace
				.program
				.account
				.state
				.fetch($workSpace.baseAccount.publicKey);
			counter = account.count.toString();
		} catch (e) {
			console.log('Error: ', e);
		}
	}
</script>

{#if $walletStore?.connected}
	{#if counter}
		Counter is {counter}
	{:else}
		<button on:click={initialize}>Initialize counter</button>
	{/if}
{/if}
