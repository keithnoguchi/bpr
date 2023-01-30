import * as anchor from "@project-serum/anchor";
import { web3, Program } from "@project-serum/anchor";
import { AnchorMultisig3 } from "../target/types/anchor_multisig3";
import { expect } from "chai";

describe("anchor-multisig3", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorMultisig3 as Program<AnchorMultisig3>;
  const wallet = provider.wallet;
  const [multisig, bump] = web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("multisig"), wallet.publicKey.toBuffer()],
    program.programId
  );

  const threshold = 2;
  const signers = [];
  signers.push(wallet.payer);
  signers.push(web3.Keypair.generate());
  signers.push(web3.Keypair.generate());
  const payees = [];
  payees.push(web3.Keypair.generate());
  payees.push(web3.Keypair.generate());
  payees.push(web3.Keypair.generate());

  beforeEach(async () => {
    await program.methods
      .open(
        bump,
        threshold,
        signers.map((pair) => pair.publicKey)
      )
      .accounts({ funder: wallet.publicKey, multisig })
      .signers([wallet.payer])
      .rpc();
  });

  afterEach(async () => {
    try {
      await program.methods
        .close(bump)
        .accounts({ funder: wallet.publicKey, multisig })
        .signers([wallet.payer])
        .rpc();
    } catch (_) {
      // ignore the closing errors.
    }
  });

  it("Opens the account", async () => {
    const ms = await program.account.multisig.fetch(multisig);
    expect(ms.bump).to.equal(bump);
    expect(ms.m).to.equal(threshold);
    expect(ms.n).to.equal(signers.length);
    expect(ms.signers).to.include.deep.members(
      signers.map((pair) => pair.publicKey)
    );
    expect(ms.signers).to.have.lengthOf(5);
    expect(ms.txs).to.have.lengthOf(0);
  });

  it("Funds 50 SOL to the account", async () => {
    const before = await provider.connection.getBalance(multisig);
    await program.methods
      .fund(bump, new anchor.BN(50 * web3.LAMPORTS_PER_SOL))
      .accounts({ funder: wallet.publicKey, multisig })
      .signers([wallet.payer])
      .rpc();

    const lamports = await provider.connection.getBalance(multisig);
    expect(lamports - before).to.equal(50 * web3.LAMPORTS_PER_SOL);
  });

  it("Closes the multisig account", async () => {
    await program.methods
      .close(bump)
      .accounts({ funder: wallet.publicKey, multisig })
      .signers([wallet.payer])
      .rpc();

    try {
      await program.account.multisig.fetch(multisig);
      expect.fail("it should throw");
    } catch (e) {
      expect(e.message).to.contain("Account does not exist");
    }
  });
});
