import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { AnchorMultisig } from "../target/types/anchor_multisig";
import { assert, expect } from "chai";

describe("anchor-multisig", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AnchorMultisig as Program<AnchorMultisig>;

  // Dummy owners for the tests.
  const ownerA = anchor.web3.Keypair.generate();
  const ownerB = anchor.web3.Keypair.generate();
  const ownerC = anchor.web3.Keypair.generate();
  const ownerD = anchor.web3.Keypair.generate();
  const ownerE = anchor.web3.Keypair.generate();

  // Test multisig and the transaction keypair.
  const multisigKeypair = anchor.web3.Keypair.generate();
  const transactionKeypair = anchor.web3.Keypair.generate();

  // Get the bump for the PDA based on the multisig
  // address.  The bump is stored in the multisig
  // data account and the accountSigner will be used
  // for the transaction creation below.
  const [multisigSigner, bump] = anchor.web3.PublicKey.findProgramAddressSync(
    [multisigKeypair.publicKey.toBuffer()],
    program.programId
  );

  it("Creates and initialize a multisig account", async () => {
    // The size is not tuned yet and will come back how
    // to adjust to the actual size, e.g. number of
    // owners of this multisig account.
    const accountKeypair = multisigKeypair;
    const accountSize = 200;

    // A, B, and C is the original owner.
    const owners = [ownerA.publicKey, ownerB.publicKey, ownerC.publicKey];
    const threshold = new anchor.BN(2);

    const tx = await program.rpc.initializeMultisig(owners, threshold, bump, {
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

    const got = await program.account.multisig.fetch(accountKeypair.publicKey);

    assert.strictEqual(got.bump, bump);
    /// Threshold is in BN as it's a u64.
    assert.isTrue(got.threshold.eq(new anchor.BN(2)));
    assert.deepEqual(got.owners, owners);
    assert.strictEqual(got.ownerSetSeqno, 0);
  });

  it("Creates and initializes a transaction", async () => {
    // A new transaction keypair and size.
    //
    // We don't need this huge account size and will
    // come back here for the proper sizing.
    const accountKeypair = transactionKeypair;
    const accountSize = 1000;

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

    // Change the owner to A, B, and D, instead of C.
    const data = program.coder.instruction.encode("set_owners", {
      owners: [ownerA.publicKey, ownerB.publicKey, ownerD.publicKey],
    });

    const tx = await program.rpc.initializeTransaction(
      program.programId,
      accounts,
      data,
      {
        accounts: {
          multisig: multisigKeypair.publicKey,
          transaction: accountKeypair.publicKey,
          proposer: ownerA.publicKey,
        },
        instructions: [
          await program.account.transaction.createInstruction(
            accountKeypair,
            accountSize
          ),
        ],
        signers: [accountKeypair, ownerA],
      }
    );

    console.log("Transaction under Multisig account had been created", tx);

    const got = await program.account.transaction.fetch(
      accountKeypair.publicKey
    );

    assert.isTrue(got.multisig.equals(multisigKeypair.publicKey));
    assert.isTrue(got.programId.equals(program.programId));
    assert.deepEqual(got.accounts, accounts);
    assert.deepEqual(got.data, data);
    assert.strictEqual(got.signers.length, 3);
    assert.isTrue(got.signers[0]); // ownerA.
    assert.isNotTrue(got.signers[1]); // ownerB.
    assert.isNotTrue(got.signers[2]); // ownerC.
    assert.isNotTrue(got.executed);
    assert.strictEqual(got.ownerSetSeqno, 0);
  });

  it("Approves the transaction", async () => {
    const accounts = {
      multisig: multisigKeypair.publicKey,
      transaction: transactionKeypair.publicKey,
      owner: ownerB.publicKey,
    };
    const signers = [ownerB];

    const tx = await program.rpc.approveTransaction({
      accounts,
      signers,
    });
    console.log("Approve transaction succeeded", tx);

    const got = await program.account.transaction.fetch(
      transactionKeypair.publicKey
    );

    assert.isTrue(got.multisig.equals(multisigKeypair.publicKey));
    assert.isTrue(got.programId.equals(program.programId));
    assert.isTrue(got.signers[0]); // ownerA.
    assert.isTrue(got.signers[1]); // ownerB.
    assert.isNotTrue(got.signers[2]); // ownerC.
    assert.isNotTrue(got.executed);
  });
});
