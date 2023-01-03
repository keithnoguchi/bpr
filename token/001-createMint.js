// 001-createMint.js to create a new token.
//
// Please refer to the [createMint] command reference for more detail.
//
// # Prerequisite
//
// Fund some solana to the payer account before running this script.
//
// You can do that by running the following `solana` command from
// your terminal once you setup the local test ledger.
//
// Run the test leadger first:
// ```
// $ solana-test-leadger --bind-address 127.0.0.1 &
//
// and then airdrop SOLs to the payer as well as mint authority:
//
// ```
// $ solana airdrop 2 -k keys/001/payer.json
// $ solana airdrop 2 -k keys/001/mintAuthority.json
// ```
//
// After that, you should be able to create/mint a token by
// running this file as:
//
// ```
// $ node 001-createMint.js
// ```
//
// Even if there is no fee debited from the mint authority,
// you need to have some balance to allow the authority for
// the newly created token.
//
// Please note that in case you get an error, e.g. address is already
// used, then just delete the leager and go through the steps
// above:
//
// ```
// $ killall solana-test-ledger
// $ rm -rf test-ledger # :)
// ```
//
// Happy Hacking!
//
// [createMint]: https://solana-labs.github.io/solana-program-library/token/js/modules.html#createMint
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { createMint } from "@solana/spl-token";

(async () => {
  // Use the solana-test-validator.
  //
  // You just run the following command from your console:
  //
  // ```
  // $ solana-test-validator --bind-address 127.0.0.1
  // ```
  const c = new Connection("http://127.0.0.1:8899");

  // payer of this newly minted token.
  //
  // This is a copy of the `keys/001/payer.json` file.
  const payer = Keypair.fromSecretKey(
    new Uint8Array([
      24,105,165,60,87,114,114,175,79,208,26,102,237,73,91,42,146,138,17,198,1,81,148,160,78,167,102,195,205,26,25,11,81,255,157,177,9,145,91,131,151,74,120,62,119,170,245,53,79,156,36,166,54,36,130,218,117,14,240,226,22,114,14,25]
    )
  );

  // It's a public key of the mintAuthority.json.  You can get it
  // by running the following command on your console:
  //
  // ```
  // $ solana address -k keys/001/mintAuthority.json
  // ```
  const mintAuthority = new PublicKey('8p5p3PryPsdkr64xHCQTohPGgBoMSCK1cSjrkv4gWjX6');

  // Make 2 decimals, just like dollar.
  const decimals = 2;

  // Here is the mint keypair, you can let the solana cluster to randomly
  // generate it for you, but we've created it with `solana-keygen grind`
  // command as below:
  //
  // ```
  // $ solana-keygen grind --starts-with aa:2
  // ```
  //
  // The secrent key is just the copy of the file, keys/001/aa*.json.
  const mintKeypair = Keypair.fromSecretKey(
    new Uint8Array([
      192,253,200,87,70,163,149,18,32,162,170,83,218,179,179,110,72,177,196,173,3,78,248,3,219,180,53,245,26,173,225,156,8,154,100,2,55,22,234,103,169,143,140,108,148,245,92,61,112,16,227,131,58,101,153,63,33,34,81,231,14,134,47,185
    ])
  );

  const mintPublicKey = await createMint(
    c,
    payer,
    mintAuthority,
    mintAuthority,
    decimals,
    mintKeypair,
  );

  // This mintPublicKey should be match to the our mintKeypair public key.
  //
  // You can check it by running the following solana command from your
  // terminal:
  //
  // ```
  // $ solana address -k keys/001/aa*
  // ```
  console.log(mintPublicKey.toBase58());

  // After this you can check the balance both the payer as well as the
  // mint authority through the following command:
  //
  // ```
  // $ s balance -k keys/001/payer.json
  // 1.9985284 SOL
  // $ s balance -k keys/001/mintAuthority.json
  // 2 SOL
  // ```
  // As you can see, only the payer paied the fee.
})();
