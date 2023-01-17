import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  Signer,
} from "@solana/web3.js";
import * as token from "@solana/spl-token";
import {
  getKeypair,
  getPublicKey,
  writePublicKey,
  getTokenBalance,
} from "./utils";

const setup = async () => {
  const alicePublicKey = getPublicKey("alice");
  const bobPublicKey = getPublicKey("bob");
  const clientKeypair = getKeypair("id");
  const conn = new Connection("http://127.0.0.1:8899", "confirmed");

  // Give enough founds in the native SOL.
  console.log("Airdrop 10 SOL to Alice", alicePublicKey.toBase58());
  const tx1 = await conn.requestAirdrop(alicePublicKey, LAMPORTS_PER_SOL * 10);
  await conn.confirmTransaction(tx1);
  console.log("Airdrop 10 SOL to Bob", bobPublicKey.toBase58());
  const tx2 = await conn.requestAirdrop(bobPublicKey, LAMPORTS_PER_SOL * 10);
  await conn.confirmTransaction(tx2);
  console.log("Airdrop 10 SOL to Client", clientKeypair.publicKey.toBase58());
  const tx3 = await conn.requestAirdrop(clientKeypair.publicKey, LAMPORTS_PER_SOL * 10);
  await conn.confirmTransaction(tx3);

  // Setup Token X accounts and mints 50X to Alice.
  const [mintX, aliceTokenAccountForX, bobTokenAccountForX] = await setupMint(
    "X",
    conn,
    alicePublicKey,
    bobPublicKey,
    clientKeypair,
  );
  console.log("Minting 50X to Alice's X TokenAccount", aliceTokenAccountForX.toBase58());
  const tx4 = await token.mintTo(conn, clientKeypair, mintX, aliceTokenAccountForX,
                                 clientKeypair, 50);
  console.log("Transaction ID:", tx4);
  await conn.confirmTransaction(tx4);

  // Setup Token Y accounts.
  const [mintY, aliceTokenAccountForY, bobTokenAccountForY] = await setupMint(
    "Y",
    conn,
    alicePublicKey,
    bobPublicKey,
    clientKeypair,
  );

  console.log("Minting 60Y to Bob's Y TokenAccount", bobTokenAccountForY.toBase58());
  const tx5 = await token.mintTo(conn, clientKeypair, mintY, bobTokenAccountForY,
                                 clientKeypair, 60);
  console.log("Transaction ID:", tx5);
  await conn.confirmTransaction(tx5);

  console.log("Setup complete\n");
  console.table([
    {
      name: "Alice Token Account X",
      balance: await getTokenBalance(aliceTokenAccountForX, conn),
      address: aliceTokenAccountForX.toBase58(),
    },
    {
      name: "Alice Token Account Y",
      balance: await getTokenBalance(aliceTokenAccountForY, conn),
      address: aliceTokenAccountForY.toBase58(),
    },
    {
      name: "Bob Token Account X",
      balance: await getTokenBalance(bobTokenAccountForX, conn),
      address: bobTokenAccountForX.toBase58(),
    },
    {
      name: "Bob Token Account Y",
      balance: await getTokenBalance(bobTokenAccountForY, conn),
      address: bobTokenAccountForY.toBase58(),
    },
  ]);
  console.log("");
}

const setupMint = async (
  name: string,
  conn: Connection,
  alicePublicKey: PublicKey,
  bobPublicKey: PublicKey,
  clientKeypair: Signer,
): Promise<[PublicKey, PublicKey, PublicKey]> => {
  console.log(`Creating Mint ${name}...`);
  const mint = await createMint(conn, clientKeypair);
  writePublicKey(mint, `mint_${name.toLowerCase()}`);
  console.log(`Creating Alice TokenAccount for ${name}...`);
  const aliceTokenAccount = await token.createAccount(
    conn,
    clientKeypair,  // payer
    mint,           // mint publickey
    alicePublicKey, // authority publickey
  );
  writePublicKey(aliceTokenAccount, `alice_${name.toLowerCase()}`);
  console.log(`Creating Bob TokenAccount for ${name}...`);
  const bobTokenAccount = await token.createAccount(
    conn,
    clientKeypair, // payer
    mint,          // mint publickey
    bobPublicKey,  // authority publickey
  );
  writePublicKey(bobTokenAccount, `bob_${name.toLowerCase()}`);
  return [mint, aliceTokenAccount, bobTokenAccount];
}

const createMint = (
  conn: Connection,
  { publicKey, secretKey }: Signer,
) => {
  // https://spl.solana.com/token
  return token.createMint(
    conn,
    {            // payer.
      publicKey,
      secretKey,
    },
    publicKey,   // mintAuthority.
    null,        // freezeAuthority.
    0,           // zero decimal.
  );
};

setup();
