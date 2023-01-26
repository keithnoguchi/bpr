<script>
	import { onMount } from 'svelte';
	import {
		WalletMultiButton,
		WalletProvider
	} from '@svelte-on-solana/wallet-adapter-ui';
	import {
		AnchorConnectionProvider
	} from '@svelte-on-solana/wallet-adapter-anchor';
	import idl from '../../../target/idl/anchor_counter.json';

	const localStorageKey = 'walletAdapter';
	const network = 'http://127.0.0.1:8899';
	let wallets;

	onMount(async () => {
		const {
			PhantomWalletAdapter
		} = await import('@solana/wallet-adapter-wallets');

		const walletsMap = [
			new PhantomWalletAdapter(),
		];
		wallets = walletsMap
	});
</script>

<WalletProvider { localStorageKey } { wallets } autoConnect />
<AnchorConnectionProvider {network} {idl} />
<WalletMultiButton />

<div>
	<slot />
</div>
