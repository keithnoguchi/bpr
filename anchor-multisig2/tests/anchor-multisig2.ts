import * as anchor from "@project-serum/anchor";
import { web3, Program, TransactionSignature } from "@project-serum/anchor";
import { AnchorMultisig2 } from "../target/types/anchor_multisig2";
import { expect } from "chai";

describe("anchor-multisig2", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorMultisig2 as Program<AnchorMultisig2>;
  const payer = provider.wallet;
  const signerA = web3.Keypair.generate();
  const signerB = web3.Keypair.generate();

  it("opens and closes the Multisig account", async () => {
    const [multisig, bump] = web3.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("multisig"), payer.publicKey.toBuffer()],
      program.programId
    );
    console.log(
      `Multisig: https://explorer.solana.com/address/${multisig}?cluster=custom&customUrl=http%3A%2F%2F127.0.0.1%3A8899`
    );

    const signers = [signerA.publicKey, signerB.publicKey];

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
    expect(account.signers[0]).to.eql(payer.publicKey);
    expect(account.signers[1]).to.eql(signerA.publicKey);
    expect(account.signers[2]).to.eql(signerB.publicKey);
    expect(account.signers.length).to.equal(11);

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
});
