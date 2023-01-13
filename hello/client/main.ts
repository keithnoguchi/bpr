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

main().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  },
);

async function main() {
  console.log("hello world!");

  const c = await establishConnection();
}

async function establishConnection() {
  const url = "http://127.0.0.1:8899";
  const c = new Connection(url, "confirmed");
  const version = await c.getVersion();
  console.log("connection to cluster established:", url, version);
  return c
}
