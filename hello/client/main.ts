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

  const conn = await establishConnection("http://127.0.0.1:8899");
  console.log("connection to cluster established", conn);
}

async function establishConnection(url: string): Promise<Connection> {
  return new Connection(url, "confirmed");
}
