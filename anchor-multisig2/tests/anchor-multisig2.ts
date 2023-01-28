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
  const signers = [signerA.publicKey, signerB.publicKey];

  const [multisig, bump] = web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("multisig"), payer.publicKey.toBuffer()],
    program.programId
  );
  console.log(
    `Multisig: https://explorer.solana.com/address/${multisig}?cluster=custom&customUrl=http%3A%2F%2F127.0.0.1%3A8899`
  );

  it("opens and closes the Multisig account", async () => {
    let tx = await program.methods
      .open(bump, 2, signers)
      .accounts({ payer: payer.publicKey, multisig })
      .rpc();
    // TransactionSignature is the type alias of string.
    expect(tx).to.be.a("string");

    // check the multisig account on-chain.
    const account = await program.account.multisig.fetch(multisig);
    expect(account.bump).to.equal(bump);
    expect(account.m).to.equal(2);
    expect(account.n).to.equal(3);
    expect(account.signers).to.include.deep.members([
      payer.publicKey, signerA.publicKey, signerB.publicKey
    ]);
    expect(account.signers).to.have.lengthOf(11);
    expect(account.txQueued).to.equal(0);
    expect(account.txs).to.have.lengthOf(10);

    tx = await program.methods
      .close()
      .accounts({ payer: payer.publicKey, multisig })
      .rpc();
    // TransactionSignature is the type alias of string.
    expect(tx).to.be.a("string");

    // The account should be closed on-chain.
    let error;
    try {
      await program.account.multisig.fetch(multisig);
    } catch (e) {
      error = e;
    }
    expect(error).to.be.an("error");
  });

  it("enqueues transactions", async () => {
    let tx = await program.methods
      .open(bump, 2, signers)
      .accounts({ payer: payer.publicKey, multisig })
      .rpc();
    // TransactionSignature is the type alias of string.
    expect(tx).to.be.a("string");

    // Creates transfer instructions.
    const ixA = web3.SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: signerA.publicKey,
      lamports: 10,
    });
    console.log(ixA)

    // Queue the transaction instruction.
    const txKeypair = web3.Keypair.generate();
    tx = await program.rpc.enqueue(
      ixA.programId,
      ixA.keys,
      ixA.data,
      {
        accounts: {
          payer: payer.publicKey,
          multisig,
          transaction: txKeypair.publicKey
        },
        instructions: [
          web3.SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            lamports: web3.LAMPORTS_PER_SOL,
            newAccountPubkey: txKeypair.publicKey,
            programId: program.programId,
            space: 300,
          })
        ],
        signers: [payer, txKeypair],
      }
    );
    console.log("enqueue tx", tx);

    tx = await program.methods
      .close()
      .accounts({ payer: payer.publicKey, multisig })
      .rpc();
    // TransactionSignature is the type alias of string.
    expect(tx).to.be.a("string");
  });
});
