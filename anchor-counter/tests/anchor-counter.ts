import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Keypair } from "@solana/web3.js";
import { AnchorCounter } from "../target/types/anchor_counter";
import { expect } from "chai";

describe("anchor-counter", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const counter = anchor.workspace.AnchorCounter as Program<AnchorCounter>;
  const counterState = Keypair.generate();

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await counter
      .methods
      .initialize()
      .accounts({
        state: counterState.publicKey,
        authority: provider.wallet.publicKey,
      })
      .signers([counterState])
      .rpc();

    console.log("Initialization transaction signature", tx);

    const state = await counter.account.state.fetch(counterState.publicKey);
    expect(state.count).to.equal(0);
  });

  it("is incremented", async () => {
    const count = 5;
    for (let i = 0; i < count; i++) {
      const tx = await counter
        .methods
        .increment()
        .accounts({
          state: counterState.publicKey,
        })
        .rpc();

        console.log("Increment transaction signature", tx);
    }
    const state = await counter.account.state.fetch(counterState.publicKey);
    expect(state.count).to.equal(count);
  });
});
