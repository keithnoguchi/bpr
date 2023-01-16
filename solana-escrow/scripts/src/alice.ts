import {
  Connection,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import * as token from "@solana/spl-token";
import BN = require("bn.js");
import {
  ESCROW_ACCOUNT_DATA_LAYOUT,
} from "./escrow";
import {
  getKeypair,
  getPublicKey,
  getEscrowTerms,
  getTokenBalance,
  writePublicKey,
} from "./utils";

const alice = async() => {
  const conn = new Connection("http://127.0.0.1:8899", "confirmed");
  const terms = getEscrowTerms();

  const escrowProgramId = getPublicKey("program");
  const aliceKeypair = getKeypair("alice");
  const tempXTokenAccountKeypair = new Keypair();
  const aliceXTokenAccountPubkey = getPublicKey("alice_x");
  const aliceYTokenAccountPubkey = getPublicKey("alice_y");
  const xTokenMintPubkey = getPublicKey("mint_x");

  // Ix#1: Create a temporary token X account as the escrow account.
  const createTempTokenAccountIx = SystemProgram.createAccount({
    programId: token.TOKEN_PROGRAM_ID,
    space: token.AccountLayout.span,
    lamports: await conn.getMinimumBalanceForRentExemption(
      token.AccountLayout.span,
    ),
    fromPubkey: aliceKeypair.publicKey,
    newAccountPubkey: tempXTokenAccountKeypair.publicKey,
  });

  // Ix#2: Initialize the newly created temp Token X account.
  //
  // https://solana-labs.github.io/solana-program-library/token/js/modules.html#createInitializeAccountInstruction
  const initTempTokenAccountIx = token.createInitializeAccountInstruction(
    tempXTokenAccountKeypair.publicKey, // account pubkey
    xTokenMintPubkey,                   // mint pubkey
    aliceKeypair.publicKey,             // authority pubkey
  );

  // Ix#3: Transfer token X to the temp token account.
  //
  // https://solana-labs.github.io/solana-program-library/token/js/modules.html#createTransferInstruction
  const transferXTokensToTempAccountIx = token.createTransferInstruction(
    aliceXTokenAccountPubkey,           // from
    tempXTokenAccountKeypair.publicKey, // to
    aliceKeypair.publicKey,             // authority
    terms.bobExpectedAmount,            // amount
  );

  // Ix#4: Creates an escrow account to hold Alice's temp Token X.
  console.log("escrow data size", ESCROW_ACCOUNT_DATA_LAYOUT.span);
  const escrowKeypair = new Keypair();
  const createEscrowAccountIx = SystemProgram.createAccount({
    space: ESCROW_ACCOUNT_DATA_LAYOUT.span,
    lamports: await conn.getMinimumBalanceForRentExemption(
      ESCROW_ACCOUNT_DATA_LAYOUT.span,
    ),
    fromPubkey: aliceKeypair.publicKey,
    newAccountPubkey: escrowKeypair.publicKey,
    programId: escrowProgramId,
  });

  // Ix#5: Initialize the escrow account for the above tmp token X account.
  const initEscrowIx = new TransactionInstruction({
    programId: escrowProgramId,
    keys: [
      { // Initializer.
        pubkey: aliceKeypair.publicKey,
        isSigner: true,
        isWritable: false,
      },
      { // temp token X account to transfer to bob's Y token.
        pubkey: tempXTokenAccountKeypair.publicKey,
        isSigner: false,
        isWritable: true,
      },
      { // alice's token Y account to receive bob's Y token.
        pubkey: aliceYTokenAccountPubkey,
        isSigner: false,
        isWritable: false,
      },
      { // escrow account.
        pubkey: escrowKeypair.publicKey,
        isSigner: false,
        isWritable: true,
      },
      { // For rent exemption calculation.
        pubkey: SYSVAR_RENT_PUBKEY,
        isSigner: false,
        isWritable: false,
      },
      { // For PDA creation and transfer by escrow program.
        pubkey: token.TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false,
      },
    ],
    data: Buffer.from(
      Uint8Array.of(0, ...new BN(terms.aliceExpectedAmount).toArray("le", 8)),
    ),
  });

  console.log("Sending Alice's transaction...");
  const tx = await conn.sendTransaction(
    new Transaction().add(
      createTempTokenAccountIx,
      initTempTokenAccountIx,
      transferXTokensToTempAccountIx,
      createEscrowAccountIx,
      initEscrowIx,
    ), // instructions.
    [aliceKeypair, tempXTokenAccountKeypair, escrowKeypair], // signers.
    { skipPreflight: false, preflightCommitment: "confirmed" },
  );
  await conn.confirmTransaction(tx);

  console.log("transaction:", tx);
  console.log(
    `Escrow successfully initialized. Alice is offering ${terms.bobExpectedAmount}X for ${terms.aliceExpectedAmount}Y\n`
  );
  writePublicKey(escrowKeypair.publicKey, "escrow");

  console.table([
    {
      name: "Alice Token Account X",
      balance: await getTokenBalance(aliceXTokenAccountPubkey, conn),
      address: aliceXTokenAccountPubkey.toBase58(),
    },
    {
      name: "Alice Token Account Y",
      balance: await getTokenBalance(aliceYTokenAccountPubkey, conn),
      address: aliceYTokenAccountPubkey.toBase58(),
    },
    {
      name: "Bob Token Account X",
      balance: await getTokenBalance(getPublicKey("bob_x"), conn),
      address: getPublicKey("bob_x").toBase58(),
    },
    {
      name: "Bob Token Account Y",
      balance: await getTokenBalance(getPublicKey("bob_y"), conn),
      address: getPublicKey("bob_y").toBase58(),
    },
    {
      name: "Alice's Token X Escrow Account",
      balance: await getTokenBalance(tempXTokenAccountKeypair.publicKey, conn),
      address: tempXTokenAccountKeypair.publicKey.toBase58(),
    },
    {
      name: "Escrow Data Account",
      address: escrowKeypair.publicKey.toBase58(),
    },
    {
      name: "Escrow Program Account",
      address: escrowProgramId.toBase58(),
    },
  ]);
  console.log("");
}

alice();
