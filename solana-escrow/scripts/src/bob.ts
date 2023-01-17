import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import BN = require("bn.js");
import {
  getEscrowTerms,
  EscrowLayout,
  ESCROW_ACCOUNT_DATA_LAYOUT,
} from "./escrow";
import {
  getKeypair,
  getPublicKey,
  getTokenBalance,
} from "./utils";

const bob = async () => {
  const conn = new Connection("http://127.0.0.1:8899", "confirmed");
  const terms = getEscrowTerms();

  const escrowProgramId = getPublicKey("program");
  const bobKeypair = getKeypair("bob");
  const bobXTokenAccountPubkey = getPublicKey("bob_x");
  const bobYTokenAccountPubkey = getPublicKey("bob_y");
  const escrowStateAccountPubkey = getPublicKey("escrow");

  const escrowAccount = await conn.getAccountInfo(escrowStateAccountPubkey);
  if (escrowAccount === null) {
    console.error("Could not find escrow at given address!");
    process.exit(1);
  }

  // Decode the escrow state account to prepare for the exchange transaction.
  const encodedEscrowState = escrowAccount.data;
  const decodedEscrowLayout = ESCROW_ACCOUNT_DATA_LAYOUT.decode(
    encodedEscrowState,
  ) as EscrowLayout;
  const escrowState = {
    escrowAccountPubkey: escrowStateAccountPubkey,
    isInitialized: !!decodedEscrowLayout.isInitialized,
    initializerAccountPubkey: new PublicKey(
      decodedEscrowLayout.initializerPubkey,
    ),
    XTokenTempAccountPubkey: new PublicKey(
      decodedEscrowLayout.initializerTempTokenAccountPubkey,
    ),
    initializerYTokenAccount: new PublicKey(
      decodedEscrowLayout.initializerReceivingTokenAccountPubkey,
    ),
    expectedAmount: new BN(decodedEscrowLayout.expectedAmount, 10, "le"),
  };
  const PDA = await PublicKey.findProgramAddress(
    [Buffer.from("escrow")],
    escrowProgramId,
  );

  // Let's create a exchange instruction.
  const exchangeInstruction = new TransactionInstruction({
    programId: escrowProgramId,
    data: Buffer.from(
      Uint8Array.of(1, ...new BN(terms.bobExpectedAmount).toArray("le", 8)),
    ),
    keys: [
      {
        pubkey: bobKeypair.publicKey,
        isSigner: true,
        isWritable: false,
      },
      {
        pubkey: bobYTokenAccountPubkey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: bobXTokenAccountPubkey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: escrowState.XTokenTempAccountPubkey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: escrowState.initializerAccountPubkey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: escrowState.initializerYTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: escrowStateAccountPubkey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: PDA[0],
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  console.log("Sending Bob's transaction...");
  const tx = await conn.sendTransaction(
    new Transaction().add(exchangeInstruction),
    [bobKeypair],
    {
      skipPreflight: false,
      preflightCommitment: "confirmed",
    },
  );
  await conn.confirmTransaction(tx);
  console.log("transaction:", tx);

  // checking if the temporary accounts, including
  // escrow are closed.
  if ((await conn.getAccountInfo(escrowStateAccountPubkey)) !== null) {
    console.error("Escrow account has not been closed");
    process.exit(1);
  }

  if ((await conn.getAccountInfo(escrowState.XTokenTempAccountPubkey)) !== null) {
    console.error("Temporary X token account has not been closed");
    process.exit(1);
  }

  console.table([
    {
      name: "Alice Token Account X",
      balance: await getTokenBalance(getPublicKey("alice_x"), conn),
      address: getPublicKey("alice_x").toBase58(),
    },
    {
      name: "Alice Token Account Y",
      balance: await getTokenBalance(getPublicKey("alice_y"), conn),
      address: getPublicKey("alice_y").toBase58(),
    },
    {
      name: "Bob Token Account X",
      balance: await getTokenBalance(bobXTokenAccountPubkey, conn),
      address: bobXTokenAccountPubkey.toBase58(),
    },
    {
      name: "Bob Token Account Y",
      balance: await getTokenBalance(bobYTokenAccountPubkey, conn),
      address: bobYTokenAccountPubkey.toBase58(),
    },
  ]);
  console.log("");
}

bob();
