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

  await establishPayer(conn);
}

async function establishConnection(url: string): Promise<Connection> {
  return new Connection(url, "confirmed");
}

async function establishPayer(conn: Connection) {
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
  let fees = await conn.getMinimumBalanceForRentExemption(GreetingAccount.SIZE);
  console.log("minimum fee for the rent exemption", fees);
}
