import * as anchor from "@project-serum/anchor";
import { web3, Program } from "@project-serum/anchor";
import { AnchorMultisig3 } from "../target/types/anchor_multisig3";
import { expect } from "chai";

describe("anchor-multisig3", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Prepares for the multisig PDAs.
  const program = anchor.workspace.AnchorMultisig3 as Program<AnchorMultisig3>;
  const wallet = provider.wallet;
  const [state, stateBump] = web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("state"), wallet.publicKey.toBuffer()],
    program.programId
  );
  const [fund, fundBump] = web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("fund"), state.toBuffer()],
    program.programId
  );

  // 3/5 multisig with 100 max transaction queue.
  const threshold = 3;
  const signers = [];
  signers.push(wallet.payer);
  for (let i = 0; i < 4; i++) {
    signers.push(web3.Keypair.generate());
  }
  const queueDepth = 100;

  // 10 different payees.
  const payees = [];
  for (let i = 0; i < 10; i++) {
    payees.push(web3.Keypair.generate());
  }

  before(async () => {
    // Make sure all the signers have enough SOL to
    // create transfers.
    for (let signer of signers) {
      const tx = await provider.connection.requestAirdrop(
        signer.publicKey,
        1 * web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(tx);
    }
  });

  beforeEach(async () => {
    await program.methods
      .create(
        threshold,
        signers.map((pair) => pair.publicKey),
        queueDepth,
        stateBump,
        fundBump
      )
      .accounts({
        funder: wallet.publicKey,
        state,
        fund,
      })
      .signers([wallet.payer])
      .rpc();
  });

  afterEach(async () => {
    try {
      await program.methods
        .close(stateBump, fundBump)
        .accounts({
          funder: wallet.publicKey,
          state,
          fund,
        })
        .signers([wallet.payer])
        .rpc();
    } catch (_e) {
      // ignore the error.
    }
  });

  it("Checks the multisig state account state", async () => {
    const ms = await program.account.state.fetch(state);
    expect(ms.m).to.equal(threshold);
    expect(ms.q).to.equal(queueDepth);
    expect(ms.fund).to.deep.equal(fund);
    expect(ms.signers).to.have.lengthOf(signers.length);
    expect(ms.signers).to.include.deep.members(
      signers.map((pair) => pair.publicKey)
    );
    for (let signed of ms.signed) {
      expect(signed).to.be.false;
    }
    expect(ms.queue).to.have.lengthOf(0);
  });

  it("Checks the account closure", async () => {
    await program.methods
      .close(stateBump, fundBump)
      .accounts({
        funder: wallet.publicKey,
        state,
        fund,
      })
      .signers([wallet.payer])
      .rpc();

    try {
      await program.account.state.fetch(state);
      expect.fail("it should throw");
    } catch (e) {
      expect(e.message).to.contain("Account does not exist");
    }
  });

  it("Checks 1,000,000 SOL funding", async () => {
    const before = await provider.connection.getBalance(fund);
    const lamports = 1000000 * web3.LAMPORTS_PER_SOL;
    await program.methods
      .fund(new anchor.BN(lamports), stateBump, fundBump)
      .accounts({
        funder: wallet.publicKey,
        state,
        fund,
      })
      .signers([wallet.payer])
      .rpc();

    const ms = await program.account.state.fetch(state);
    expect(ms.balance.eq(new anchor.BN(lamports))).to.be.true;

    // check the multisig fund native lamports as well.
    const balance = await provider.connection.getBalance(fund);
    expect(balance - before).to.equal(lamports);
  });

  it("Checks multiple queued transactions", async () => {
    let balance = 1000000 * web3.LAMPORTS_PER_SOL;
    await program.methods
      .fund(new anchor.BN(balance), stateBump, fundBump)
      .accounts({
        funder: wallet.publicKey,
        state,
        fund,
      })
      .signers([wallet.payer])
      .rpc();

    for (const [index, payee] of payees.entries()) {
      const transfer = web3.Keypair.generate();
      const lamports = 100 * index * web3.LAMPORTS_PER_SOL;
      const lamportsBN = new anchor.BN(lamports);
      const signer = signers[index % signers.length];
      const tx = await program.methods
        .createTransfer(payee.publicKey, lamportsBN, fundBump)
        .accounts({
          creator: signer.publicKey,
          state,
          fund,
          transfer: transfer.publicKey,
        })
        .signers([signer, transfer])
        .rpc();

      balance -= lamports;
    }

    const ms = await program.account.state.fetch(state);
    expect(ms.queue).to.have.lengthOf(payees.length);
    expect(ms.balance.eq(new anchor.BN(balance))).to.be.true;
  });

  it("Checks the approval and the transfer execution", async () => {
    let balance = 1000000 * web3.LAMPORTS_PER_SOL;
    await program.methods
      .fund(new anchor.BN(balance), stateBump, fundBump)
      .accounts({
        funder: wallet.publicKey,
        state,
        fund,
      })
      .signers([wallet.payer])
      .rpc();

    for (const [index, payee] of payees.entries()) {
      const transfer = web3.Keypair.generate();
      const lamports = 1000 * index * web3.LAMPORTS_PER_SOL;
      const lamportsBN = new anchor.BN(lamports);
      const signer = signers[index % signers.length];
      const tx = await program.methods
        .createTransfer(payee.publicKey, lamportsBN, fundBump)
        .accounts({
          creator: signer.publicKey,
          state,
          fund,
          transfer: transfer.publicKey,
        })
        .signers([signer, transfer])
        .rpc();
    }

    let ms = await program.account.state.fetch(state);
    expect(ms.queue).to.have.lengthOf(payees.length);
    expect(ms.signed.filter((signed) => signed)).to.have.lengthOf(0);

    // We need both the transfer as well as the
    // payee account information to make the
    // transfer executed.
    //
    // This should be abstructed by SDK.
    const remainingAccounts = payees
      .map((payee) => {
        return {
          pubkey: payee.publicKey,
          isWritable: true,
          isSigner: false,
        };
      })
      .concat(
        ms.queue.map((transfer) => {
          return {
            pubkey: transfer,
            isWritable: true,
            isSigner: false,
          };
        })
      );

    // 3/3 approval.
    for (let i = 0; i < threshold; i++) {
      await program.methods
        .approve(fundBump)
        .accounts({
          signer: signers[i].publicKey,
          state,
          fund,
        })
        .remainingAccounts(remainingAccounts)
        .signers([signers[i]])
        .rpc();
    }

    // The queue is empty as well as the approval
    // state was reset altogether.
    ms = await program.account.state.fetch(state);
    expect(ms.signed.filter(Boolean)).to.have.lengthOf(0);
    expect(ms.queue).to.have.lengthOf(0);
  });
});
