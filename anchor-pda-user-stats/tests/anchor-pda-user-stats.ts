import * as anchor from "@project-serum/anchor";
import { AnchorError, Program, web3 } from "@project-serum/anchor";
import { AnchorPdaUserStats } from "../target/types/anchor_pda_user_stats";
import { expect } from 'chai';

describe("anchor-pda-user-stats", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace
    .AnchorPdaUserStats as Program<AnchorPdaUserStats>;

  it("open a user stats", async () => {
    const [userStatsPda, _] = web3.PublicKey
      .findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("user-stats"),
          provider.wallet.publicKey.toBuffer()
        ],
        program.programId
      );

    await program.methods
      .open("keith")
      .accounts({
        user: provider.wallet.publicKey,
        userStats: userStatsPda,
      })
      .rpc();

    expect((await program.account.userStats.fetch(userStatsPda)).name)
      .to.equal("keith");

    // close the stats.
    let tx = await program.methods
      .close()
      .accounts({
        user: provider.wallet.publicKey,
        userStats: userStatsPda,
      })
      .rpc();

    console.log("sig to close pda", tx);

    // I hope I know a better to way to catch the
    // exception.  to.throw doesn't work with Promise.
    let resp;
    try {
      await program.account.userStats.fetch(userStatsPda);
    } catch (e) {
      resp = e;
    }
    expect(resp).to.be.instanceof(Error);
  });
});
