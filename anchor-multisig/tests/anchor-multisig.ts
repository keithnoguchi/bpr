import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { AnchorMultisig } from "../target/types/anchor_multisig";
import { expect } from "chai";

describe("anchor-multisig", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AnchorMultisig as Program<AnchorMultisig>;

  // A cache some of the account to share the multiple
  // test below.
  let _multisig;
  let _multisigSigner;
  let _ownerA;
  let _ownerB;
  let _ownerC;
  let _ownerD;
  let _ownerE;

  it("creates a multisig account", async () => {
    const multisig = anchor.web3.Keypair.generate();

    // Get the nonce for the PDA based on the multisig
    // address.  The nonce is stored in the multisig
    // data account and the multisigSigner will be used
    // for the transaction creation below.
    const [multisigSigner, nonce] =
      await anchor.web3.PublicKey.findProgramAddress(
        [multisig.publicKey.toBuffer()],
        program.programId
      );

    // We don't need this big size for the multisig
    // account.  I'll come back to this size later.
    const multisigSize = 200;

    // 3/5 multisig example.
    const ownerA = anchor.web3.Keypair.generate();
    const ownerB = anchor.web3.Keypair.generate();
    const ownerC = anchor.web3.Keypair.generate();
    const ownerD = anchor.web3.Keypair.generate();
    const ownerE = anchor.web3.Keypair.generate();
    const owners = [
      ownerA.publicKey,
      ownerB.publicKey,
      ownerC.publicKey,
      ownerD.publicKey,
      ownerE.publicKey,
    ];
    const threshold = new anchor.BN(3);

    const tx = await program.rpc.createMultisig(owners, threshold, nonce, {
      accounts: {
        multisig: multisig.publicKey,
      },
      instructions: [
        await program.account.multisig.createInstruction(
          multisig,
          multisigSize
        ),
      ],
      signers: [multisig],
    });

    console.log("Multisig account had been created", tx);

    let multisigAccount = await program.account.multisig.fetch(
      multisig.publicKey
    );

    // nonce should be strict equal, e.g. '==='
    //
    // We can treat the `multisigAccount.nonce`
    // as the standard integer because it's u8.
    expect(multisigAccount.nonce).is.eql(nonce);
    expect(multisigAccount.threshold.eq(new anchor.BN(3)));
    expect(multisigAccount.owners).is.eql(owners);
    expect(multisigAccount.ownerSetSeqno).is.eql(0);

    // Caches the multisig account for the following
    // test.
    _multisig = multisig;
    _multisigSigner = multisigSigner;
    _ownerA = ownerA;
    _ownerB = ownerB;
    _ownerC = ownerC;
    _ownerD = ownerD;
    _ownerE = ownerE;
  });

  it("creates a transaction", async () => {
    const multisig = _multisig;
    const multisigSigner = _multisigSigner;
    const ownerA = _ownerA;
    const ownerB = _ownerB;
    const ownerC = _ownerC;

    const accounts = [
      {
        pubkey: multisig.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: multisigSigner,
        isWritable: false,
        isSigner: true,
      }
    ];
    const data = program
      .coder
      .instruction
      .encode("set_owners", {
        owners: [ownerA, ownerB, ownerC]
      });

    // A new transaction keypair.
    const transaction = anchor.web3.Keypair.generate();

    // A transaction account size.
    //
    // I'll come back here for the proper sizing.
    const txSize = 1000;

    const tx = await program
      .rpc
      .createTransaction(
        program.publicKey,
        accounts,
        data,
        {
          accounts: {
            multisig: multisig.publicKey,
            transaction: transaction.publicKey,
            proposer: ownerA.publicKey,
          },
          instructions: [
            await program.account.transaction
            .createInstruction(
              transaction,
              txSize
            ),
          ],
          signers: [transaction, ownerA]
        });

    console.log("Transaction under Multisig account had been created", tx);
  });
});
