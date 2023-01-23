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
  let _multisigKeypair;
  let _multisigSigner;
  let _ownerA;
  let _ownerB;
  let _ownerC;
  let _ownerD;
  let _ownerE;

  it("creates and initialize a multisig account", async () => {
    // Here is the multisig account keypair and the size.
    //
    // The size is not tuned yet and will come back how
    // to adjust to the actual size, e.g. number of
    // owners of this multisig account.
    const accountKeypair = anchor.web3.Keypair.generate();
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

    const tx = await program.rpc.initializeMultisig(owners, threshold, nonce, {
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
    _multisigKeypair = accountKeypair;
    _multisigSigner = accountSigner;
    _ownerA = ownerA;
    _ownerB = ownerB;
    _ownerC = ownerC;
    _ownerD = ownerD;
    _ownerE = ownerE;
  });

  it("creates and initializes a transaction", async () => {
    const multisigKeypair = _multisigKeypair;
    const multisigSigner = _multisigSigner;
    const ownerA = _ownerA;
    const ownerB = _ownerB;
    const ownerC = _ownerC;

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
    const data = program.coder.instruction.encode("set_owners", {
      owners: [ownerA, ownerB, ownerC],
    });

    // A new transaction keypair.
    const transaction = anchor.web3.Keypair.generate();

    // A transaction account size.
    //
    // I'll come back here for the proper sizing.
    const txSize = 1000;

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
            transaction,
            txSize
          ),
        ],
        signers: [transaction, ownerA],
      }
    );

    console.log("Transaction under Multisig account had been created", tx);
  });
});
