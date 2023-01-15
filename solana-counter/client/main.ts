// Counter program: Simplified [Hello world]
//
// [hello world]: https://github.com/solana-labs/example-helloworld/blob/master/src/client/hello_world.ts
import {
  Keypair,
  Connection,
  PublicKey,
  LAMPORTS_PER_SOL,
  SystemProgram,
  TransactionInstruction,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as borsh from "borsh";
import os from "os";
import fs from "mz/fs";
import path from "path";
import yaml from "yaml";

class Counter {
  count = 0;
  constructor(fields: {count: number} | undefined = undefined) {
    if (fields) {
      this.count = fields.count;
    }
  }
  static SCHEMA = new Map([
    [Counter, {kind: 'struct', fields: [['count', 'u8']]}],
  ]);
  static SPACE = borsh.serialize(
    Counter.SCHEMA,
    new Counter(),
  ).length;
  static SEED = "counter";
  static NUMBER_OF_SIGNATURES = 1;
}

main().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  },
);

async function main() {
  // Creates connection.
  const conn = await establishConnection("http://127.0.0.1:8899");
  console.debug("connection to cluster established on", conn.rpcEndpoint);

  // Gets the payer/player for the transaction.
  const payer = await getPayer(conn, Counter.SPACE);
  const balance = await conn.getBalance(payer.publicKey);
  console.log(`payer(${balance/LAMPORTS_PER_SOL} SOL):`,
              payer.publicKey.toBase58());

  // Get the fees for the data account creation + transaction.
  const fees = await getFees(conn, Counter.SPACE,
                             Counter.NUMBER_OF_SIGNATURES);
  console.log(`required fee (rent + tx fee): ${fees/LAMPORTS_PER_SOL}`);

  // Gets the program ID.
  const programId = await getProgramId(
    path.resolve(__dirname, "../../target/deploy/solana_counter-keypair.json"),
  );
  console.log("programId:", programId.toBase58());

  // Checks the program validity.
  await checkProgramAccount(conn, programId);
  console.log("program is loaded on-chain and is a valid executable");

  // Gets the counter Id.
  const counterId = await getCounterId(payer, Counter.SEED, programId)
  console.log("counterId:", counterId.toBase58());

  // airdrop the payer in case there is not enough balance.
  if (balance < fees) {
    await airdropPayer(conn, payer, fees - balance);
  }

  // Creates the data account if it's not there already.
  if (await checkCounter(conn, counterId)) {
    console.log("counter is on-chain");
  } else {
    console.log(`counter(Id=${counterId}) need to be created`);
    await createCounter(conn, payer, Counter.SPACE,
                        Counter.SEED, counterId, programId);
  }

  // call the counter program until it's wrap around.
  let counter = await getCounter(conn, counterId);
  console.log(`start of the counter=${counter}`);
  while (true) {
    await incrementCounter(conn, payer, counterId, programId);
    counter = await getCounter(conn, counterId);
    if (counter == 0) {
      console.log("counter wrapped around. Let's finish the call.");
      break;
    }
    process.stdout.write(".");
  }
}

async function establishConnection(url: string): Promise<Connection> {
  return new Connection(url, "confirmed");
}

async function getPayer(conn: Connection, size: number): Promise<Keypair> {
  try {
    const config = await getConfig();
    if (!config.keypair_path) throw new Error("Missing keypair path");
    return await createKeypairFromFile(config.keypair_path);
  } catch (e) {
    console.warn(
      "Failed to create keypair from CLI config file, falling back to new random keypair",
    );
    return Keypair.generate();
  }
}

async function getFees(
  conn: Connection,
  space: number,
  number_of_signature: number,
): Promise<number> {
  let fees = await conn.getMinimumBalanceForRentExemption(space);
  console.log("minimum fee for the rent exemption", fees, "for", space, "Byte(s)");
  const {feeCalculator} = await conn.getRecentBlockhash();
  const fee_for_one_signature = feeCalculator.lamportsPerSignature;
  console.log("transaction fee for a single signature", fee_for_one_signature);
  fees += fee_for_one_signature * number_of_signature;
  console.log("total transaction fee", fees);
  return fees;
}

async function getProgramId(filePath: string): Promise<PublicKey> {
  try {
    const programKeypair = await createKeypairFromFile(filePath);
    return programKeypair.publicKey;
  } catch (e) {
    const errMsg = (e as Error).message;
    throw new Error(
      `Failed to read program keypair at '${filePath}' due to error: ${errMsg}.`,
    );
  }
}

async function checkProgramAccount(conn: Connection, programId: PublicKey) {
  const programInfo = await conn.getAccountInfo(programId);
  if (programInfo === null) {
    throw new Error("Program needs to be build and deployed");
  } else if (!programInfo.executable) {
    throw new Error("Program is not executable");
  }
}

async function getCounterId(
  payer: Keypair,
  seed: string,
  programId: PublicKey,
): Promise<PublicKey> {
  return await PublicKey.createWithSeed(
    payer.publicKey,
    seed,
    programId,
  );
}

async function checkCounter(
  conn: Connection,
  counterId: PublicKey,
): Promise<Boolean> {
  const counter = await conn.getAccountInfo(counterId);
  if (counter === null) {
    return false;
  } else {
    return true;
  }
}

async function airdropPayer(conn: Connection, payer: Keypair, amount: number) {
  const sig = await conn.requestAirdrop(
    payer.publicKey,
    amount,
  );
  await conn.confirmTransaction(sig);
}

async function createCounter(
  conn: Connection, payer: Keypair, space: number,
  seed: string, counterId: PublicKey, programId: PublicKey,
) {
  const lamports = await conn.getMinimumBalanceForRentExemption(space);

  const tx = new Transaction().add(
    SystemProgram.createAccountWithSeed({
      fromPubkey: payer.publicKey,
      basePubkey: payer.publicKey,
      seed,
      newAccountPubkey: counterId,
      lamports,
      space,
      programId,
    }),
  );
  const signers = [payer];
  await sendAndConfirmTransaction(conn, tx, signers);
}

async function getCounter(conn: Connection, counterId: PublicKey): Promise<number> {
  const counterInfo = await conn.getAccountInfo(counterId);
  if (counterInfo === null) {
    throw 'Error: cannot find the counter on chain';
  }
  const counter = borsh.deserialize(
    Counter.SCHEMA,
    Counter,
    counterInfo.data,
  );
  return counter.count;
}

async function incrementCounter(
  conn: Connection,
  payer: Keypair,
  counterId: PublicKey,
  programId: PublicKey,
) {
  const instruction = new TransactionInstruction({
    keys: [{pubkey: counterId, isSigner: false, isWritable: true}],
    programId,
    data: Buffer.alloc(0),
  });
  const signers = [payer];
  await sendAndConfirmTransaction(
    conn,
    new Transaction().add(instruction),
    signers,
  );
}

async function getConfig(): Promise<any> {
  const CONFIG_FILE_PATH = path.resolve(
    os.homedir(),
    ".config",
    "solana",
    "cli",
    "config.yml",
  );
  const configYaml = await fs.readFile(CONFIG_FILE_PATH, {encoding: "utf-8"});
  return yaml.parse(configYaml);
}

async function createKeypairFromFile(filePath: string): Promise<Keypair> {
  const secretKeyString = await fs.readFile(filePath, {encoding: "utf-8"});
  const secretKey = Uint8Array.from(JSON.parse(secretKeyString));
  return Keypair.fromSecretKey(secretKey);
}
