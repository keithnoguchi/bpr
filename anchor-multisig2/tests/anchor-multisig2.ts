import * as anchor from "@project-serum/anchor";
import { web3, Program, TransactionSignature } from "@project-serum/anchor";
import { AnchorMultisig2 } from "../target/types/anchor_multisig2";
import { expect } from "chai";

describe("anchor-multisig2", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorMultisig2 as Program<AnchorMultisig2>;
  const payer = provider.wallet.payer;
  const signerA = web3.Keypair.generate();
  const signerB = web3.Keypair.generate();
  const signers = [
    // The payer is automatically added to the signers
    // by the program.
    payer.publicKey,
    signerA.publicKey,
    signerB.publicKey,
  ];
  const threshold = 2;
  const [multisig, bump] = web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("multisig"), payer.publicKey.toBuffer()],
    program.programId
  );

  beforeEach(async () => {
    // Opens an account.
    await program.methods
      .open(bump, threshold, signers)
      .accounts({ payer: payer.publicKey, multisig })
      .rpc();
  });

  afterEach(async () => {
    // Closes the account.
    try {
      await program.methods
        .close()
        .accounts({ payer: payer.publicKey, multisig })
        .rpc();
    } catch (_) {
      // ignore the error...
    }
  });

  it("Check the account initial state", async () => {
    // check the on-chain multisig account.
    const account = await program.account.multisig.fetch(multisig);
    expect(account.bump).to.equal(bump);
    expect(account.m).to.equal(threshold);
    expect(account.n).to.equal(signers.length);
    expect(account.signers).to.include.deep.members(signers);
    expect(account.signers).to.have.lengthOf(11);
    expect(account.txQueued).to.equal(0);
    expect(account.txs).to.have.lengthOf(10);
  });

  it("Creates a transaction", async () => {
    // Creates a transfer instruction.
    const ixA = web3.SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: signerA.publicKey,
      lamports: 10,
    });

    // Queue the transaction instruction.
    const txKeypair = web3.Keypair.generate();
    const tx = await program.rpc.enqueue(ixA.programId, ixA.keys, ixA.data, {
      accounts: {
        payer: payer.publicKey,
        multisig,
        transaction: txKeypair.publicKey,
      },
      instructions: [
        web3.SystemProgram.createAccount({
          fromPubkey: payer.publicKey,
          lamports: web3.LAMPORTS_PER_SOL,
          newAccountPubkey: txKeypair.publicKey,
          programId: program.programId,
          space: 300,
        }),
      ],
      signers: [payer, txKeypair],
    });
  });
});
