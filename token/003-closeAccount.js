// Let's close the newly created token account
import { Keypair, Connection, PublicKey } from "@solana/web3.js";
import { closeAccount } from "@solana/spl-token";

// Let's get the account address from the command line.
console.assert(process.argv.length == 3);
const accountAddress = new PublicKey(process.argv[2]);

// We only have one owner, which is the payer of the account,
// as in 002-createAccount.js.
//
// It's stored under `keys/001/payer.json`:
const payer = Keypair.fromSecretKey(
  new Uint8Array([
    24,105,165,60,87,114,114,175,79,208,26,102,237,73,91,42,146,138,17,198,1,81,148,160,78,167,102,195,205,26,25,11,81,255,157,177,9,145,91,131,151,74,120,62,119,170,245,53,79,156,36,166,54,36,130,218,117,14,240,226,22,114,14,25
  ])
);

// Destination address, which will receive the remaining balance.
const destination = payer.publicKey;

// Mint authority keypire to sign this transaction,
// which is stored in `keys/001/mintAuthority.json`.
const mintAuthority = Keypair.fromSecretKey(
  new Uint8Array([
    233,62,222,40,211,61,66,197,231,235,11,156,243,84,216,120,238,18,56,242,124,87,197,254,28,147,195,176,184,224,90,196,116,17,139,224,54,71,213,146,187,12,242,3,81,24,16,116,83,82,219,222,88,226,152,90,13,244,181,223,149,217,227,81
  ])
);

// connection to the local test cluster.
const c = new Connection("http://127.0.0.1:8899");

(async () => {
  console.log(accountAddress.toBase58());

  const txSignature = closeAccount(
    c,
    payer,
    accountAddress,
    destination,
    mintAuthority,
  );
  console.log(txSignature);
})();
