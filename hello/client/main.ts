// Simplified [Hello world]
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

main().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  },
);

async function main() {
  console.log("hello world!");

  // Creates connection.
  const conn = await establishConnection("http://127.0.0.1:8899");
  console.log("connection to cluster established on", conn.rpcEndpoint);

  // Get the payer for the transaction.
  const payer = await getPayer(conn);
  console.log("payer:", payer.publicKey.toBase58());

  // get the program ID.
  const programId = await getProgramId(
    path.resolve(__dirname, "../../target/deploy/hello-keypair.json"),
  );
  console.log("programId:", programId.toBase58());

  // Check the program validity.
  if (await checkProgramAccount(conn, programId)) {
    console.log("program is loaded on-chain and is a valid executable");
  }

  // Derive the address (public key) of a greeting data account from the
  // program, so that it's easy to find later.
  const GREETING_SEED = "hello";
  const dataId = await getDataId(payer, GREETING_SEED, programId)
  console.log("dataId:", dataId.toBase58());

  // create an data account if it's not there.
  if (!await checkDataAccount(conn, dataId)) {
    console.log(`dataId(${dataId}) needed to be created`);
  }
}

async function establishConnection(url: string): Promise<Connection> {
  return new Connection(url, "confirmed");
}

async function getPayer(conn: Connection): Promise<Keypair> {
  class GreetingAccount {
    counter = 0;
    constructor(fields: {counter: number} | undefined = undefined) {
      if (fields) {
        this.counter = fields.counter;
      }
    }
    static GreetingSchema = new Map([
      [GreetingAccount, {kind: 'struct', fields: [['counter', 'u8']]}],
    ]);
    static SIZE = borsh.serialize(
      GreetingAccount.GreetingSchema,
      new GreetingAccount(),
    ).length;
  }
  const size = GreetingAccount.SIZE;
  let fees = await conn.getMinimumBalanceForRentExemption(size);
  console.log("minimum fee for the rent exemption", fees, "for", size, "Byte(s)");
  const {feeCalculator} = await conn.getRecentBlockhash();
  const fee_for_one_signature = feeCalculator.lamportsPerSignature;
  console.log("transaction fee for a single signature", fee_for_one_signature);
  fees += fee_for_one_signature; // just one signature.
  console.log("total transaction fee", fees);

  return await parsePayer();
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

async function checkProgramAccount(conn: Connection, programId: PublicKey): Promise<Boolean> {
  const programInfo = await conn.getAccountInfo(programId);
  if (programInfo === null) {
    throw new Error("Program needs to be build and deployed");
  } else if (!programInfo.executable) {
    throw new Error("Program is not executable");
  }
  return true;
}

async function getDataId(payer: Keypair, seed: string, programId: PublicKey): Promise<PublicKey> {
  return await PublicKey.createWithSeed(
    payer.publicKey,
    seed,
    programId,
  );
}

async function checkDataAccount(conn: Connection, dataId: PublicKey): Promise<Boolean> {
  const dataAccount = await conn.getAccountInfo(dataId);
  if (dataAccount === null) {
    return false;
  } else {
    return true;
  }
}

async function parsePayer(): Promise<Keypair> {
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
