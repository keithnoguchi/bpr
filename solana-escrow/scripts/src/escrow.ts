import * as BufferLayout from "@solana/buffer-layout";
import * as fs from "fs";

// Layout for a public key.
const publicKey = (property = "publicKey") => {
  return BufferLayout.blob(32, property);
};

// Layout for a 64bit unsigned value.
const uint64 = (property = "uint64") => {
  return BufferLayout.blob(8, property);
}

// Layout for a escrow account.
export const ESCROW_ACCOUNT_DATA_LAYOUT = BufferLayout.struct([
  //@ts-expect-error missing types
  BufferLayout.u8("isInitialized"),
  //@ts-expect-error missing types
  publicKey("initializerPubkey"),
  //@ts-expect-error missing types
  publicKey("initializerTempTokenAccountPubkey"),
  //@ts-expect-error missing types
  publicKey("initializerReceivingTokenAccountPubkey"),
  //@ts-expect-error missing types
  uint64("expectedAmount"),
]);

export interface EscrowLayout {
  isInitialized: number,
  initializerPubkey: Uint8Array,
  initializerReceivingTokenAccountPubkey: Uint8Array,
  initializerTempTokenAccountPubkey: Uint8Array,
  expectedAmount: Uint8Array,
}

export const getEscrowTerms = (): {
  aliceExpectedAmount: number,
  bobExpectedAmount: number,
} => {
  return JSON.parse(fs.readFileSync(`./escrow-terms.json`) as unknown as string);
};
