import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { AnchorMultisig3 } from "../target/types/anchor_multisig3";

describe("anchor-multisig3", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AnchorMultisig3 as Program<AnchorMultisig3>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
