import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { AnchorMultisig } from "../target/types/anchor_multisig";
import { expect } from "chai";

describe("anchor-multisig", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AnchorMultisig as Program<AnchorMultisig>;

  // Candidate owners for 2/3 multisig account.
  const ownerA = anchor.web3.Keypair.generate();
  const ownerB = anchor.web3.Keypair.generate();
  const ownerC = anchor.web3.Keypair.generate();
  const ownerD = anchor.web3.Keypair.generate();
  const ownerE = anchor.web3.Keypair.generate();

  // This is the 2/3 multisig account keypair.
  const multisigKeypair = anchor.web3.Keypair.generate();

  // A cache some of the account to share the multiple
  // test below.
  let _multisigSigner;

  it("creates and initialize a multisig account", async () => {
    // The size is not tuned yet and will come back how
    // to adjust to the actual size, e.g. number of
    // owners of this multisig account.
    const accountKeypair = multisigKeypair;
    const accountSize = 200;

    // Get the nonce for the PDA based on the multisig
    // address.  The nonce is stored in the multisig
    // data account and the accountSigner will be used
    // for the transaction creation below.
    const [accountSigner, nonce] =
      await anchor.web3.PublicKey.findProgramAddress(
        [accountKeypair.publicKey.toBuffer()],
        program.programId
    );

    // A, B, and C is the original owner.
    const originalOwners = [
      ownerA.publicKey,
      ownerB.publicKey,
      ownerC.publicKey,
    ];
    const threshold = new anchor.BN(2);

    const tx = await program.rpc.initializeMultisig(originalOwners, threshold, nonce, {
      accounts: {
        multisig: accountKeypair.publicKey,
      },
      instructions: [
        await program.account.multisig.createInstruction(
          accountKeypair,
          accountSize
        ),
      ],
      signers: [accountKeypair],
    });

    console.log("Multisig account had been created", tx);

    // nonce should be strict equal, e.g. '==='
    //
    // We can treat the `multisigAccount.nonce`
    // as the standard integer because it's u8.
    const got = await program.account.multisig.fetch(accountKeypair.publicKey);
    expect(got.nonce).is.eql(nonce);
    expect(got.threshold.eq(new anchor.BN(3)));
    expect(got.owners).is.eql(owners);
    expect(got.ownerSetSeqno).is.eql(0);

    // Caches the multisig account for the following
    // test.
    _multisigSigner = accountSigner;
  });

  it("creates and initializes a transaction", async () => {
    // A new transaction keypair and size.
    //
    // We don't need this huge account size and will
    // come back here for the proper sizing.
    const accountKeypair = anchor.web3.Keypair.generate();
    const accountSize = 1000;

    const multisigSigner = _multisigSigner;

    const accounts = [
      {
        pubkey: multisigKeypair.publicKey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: multisigSigner,
        isWritable: false,
        isSigner: true,
      },
    ];

    // Change the owner to A, B, and D, instead of D.
    const data = program.coder.instruction.encode("set_owners", {
      owners: [ownerA, ownerB, ownerD],
    });

    const tx = await program.rpc.initializeTransaction(
      program.publicKey,
      accounts,
      data,
      {
        accounts: {
          multisig: multisigKeypair.publicKey,
          transaction: transaction.publicKey,
          proposer: ownerA.publicKey,
        },
        instructions: [
          await program.account.transaction.createInstruction(
            accountKeypair,
            accountSize,
          ),
        ],
        signers: [transaction, ownerA],
      }
    );

    console.log("Transaction under Multisig account had been created", tx);
  });
});
