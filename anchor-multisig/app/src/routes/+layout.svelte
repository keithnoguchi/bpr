<script lang="ts">
  import * as idl from "../../../target/idl/anchor_multisig.json";
  import * as AnchorMultisig from "../../../target/types/anchor_multisig";
  import { Connection, PublicKey, clusterApiUrl } from "@solana/web3.js";

  import { onMount } from 'svelte';

  let wallet;
  let address = "";
  $: address && console.log(`Connected to wallet: ${address}`);

  const handleConnectWallet = async () => {
    const resp = await wallet.connect();
  };

  onMount(async () => {
    console.log("mounted");
    const { solana } = window;
    wallet = solana;

    wallet.on("connect", () => (address = wallet.publicKey.toString()));
    wallet.on("disconnect", () => (address = ""));

    const resp = await wallet.connect({ onlyIfTrusted: true });
  });

</script>

<nav>
  <a href="/">home</a>
  <a href="/about">about</a>
  {#if address}
  <a href="https://explorer.solana.com/address/{address}?cluster=custom&customUrl=http%3A%2F%2F127.0.0.1%3A8899">{address}</a>
  {:else if wallet}
    {#if wallet.isPhantom}
      <button on:click="{handleConnectWallet}">Connect wallet</button>
    {:else}
      Solana wallet found but not supported
    {/if}
  {:else}
    Solana wallet not found
  {/if}
</nav>

<slot />
