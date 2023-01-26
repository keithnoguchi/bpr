<script>
	import { onMount } from 'svelte';
	import { WalletProvider } from '@svelte-on-solana/wallet-adapter-ui';
	import { AnchorConnectionProvider } from '@svelte-on-solana/wallet-adapter-anchor';

	const network = 'http://127.0.0.1:8899';

	onMount(async () => {
		console.log(`${network}`);
	});
</script>
<slot />
