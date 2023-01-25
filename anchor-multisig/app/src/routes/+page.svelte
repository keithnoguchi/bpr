<script lang="ts">
  import * as idl from "../../../target/idl/anchor_multisig.json";
  import * as AnchorMultisig from "../../../target/types/anchor_multisig";
  import { Connection, PublicKey, clusterApiUrl } from "@solana/web3.js";

  import { onMount } from 'svelte';

  let wallet;
  let account = "";
  $: account && console.log(`Connected to wallet: ${account}`);

  const handleConnectWallet = async () => {
    const resp = await wallet.connect();
  };

  onMount(async () => {
    console.log("mounted");
    const { solana } = window;
    wallet = solana;

    wallet.on("connect", () => (account = wallet.publicKey.toString()));
    wallet.on("disconnect", () => (account = ""));

    const resp = await wallet.connect({ onlyIfTrusted: true });
  });
</script>

<h1>anchor-multisig</h1>
{#if account}
<h3>You wallet ID</h3>
<p>{account}</p>
{:else if wallet}
  {#if wallet.isPhantom}
    <h2>Phantom Wallet found!</h2>
    <button on:click="{handleConnectWallet}">Connect wallet</button>
  {:else}
    <h2>Solana wallet found but not supported.</h2>
  {/if}
{:else}
  <h2>Solana wallet not found.</h2>
{/if}
