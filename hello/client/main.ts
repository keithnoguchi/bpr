/**
 * Simplified [Hello world]
 *
 * [hello world]: https://github.com/solana-labs/example-helloworld/blob/master/src/client/hello_world.ts
 */
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

  const conn = await establishConnection("http://127.0.0.1:8899");
  console.log("connection to cluster established", conn);

  const payer = await establishPayer(conn);
  console.log("payer of this transaction, and rent", payer);
}

async function establishConnection(url: string): Promise<Connection> {
  return new Connection(url, "confirmed");
}

async function establishPayer(conn: Connection): Promise<Keypair> {
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

  return await getPayer();
}

async function getPayer(): Promise<Keypair> {
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
