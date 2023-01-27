import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { AnchorPdaUserStats } from "../target/types/anchor_pda_user_stats";

describe("anchor-pda-user-stats", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AnchorPdaUserStats as Program<AnchorPdaUserStats>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
