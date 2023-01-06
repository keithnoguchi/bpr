// Let's create a new account for the newly creates/minted token
// by 001-createMint.js.
//
// The address of the newly minted token is below:
//
// ```
// $ solana address -k keys/001/aa*.json
// aaoiaaL5DWRngrLUiwpGvLZtFewVdNj5v3YES5SmRxt
// ```
import { Keypair, Connection, PublicKey } from "@solana/web3.js";
import { createAccount } from "@solana/spl-token";

(async () => {
  const c = new Connection("http://127.0.0.1:8899");

  // same payer as 001-createMint.js.
  const payer = Keypair.fromSecretKey(
    new Uint8Array([
      24,105,165,60,87,114,114,175,79,208,26,102,237,73,91,42,146,138,17,198,1,81,148,160,78,167,102,195,205,26,25,11,81,255,157,177,9,145,91,131,151,74,120,62,119,170,245,53,79,156,36,166,54,36,130,218,117,14,240,226,22,114,14,25]
    )
  );

  // mint public key.  You can get it with the following `solana` command:
  //
  // ```
  // $ solana address -k keys/001/aa*.json
  // aaoiaaL5DWRngrLUiwpGvLZtFewVdNj5v3YES5SmRxt
  // ```
  // The file name matches to the above address, too.
  const mintAddress = new PublicKey("aaoiaaL5DWRngrLUiwpGvLZtFewVdNj5v3YES5SmRxt");

  // Owner of the newly created account.  We use the payer here, whoever owns
  // this account will pay for it.
  const newAccountOwner = payer.publicKey;

  // Generate the new account keypair, which will be returned from the ledger.
  //
  // This way, same owner, in this case, payer, can have multiple accounts
  // for this token.
  //
  // Try comment this line out and run this script twice and you'll
  // see the error.
  const newAccountKeypair = Keypair.generate();
  console.log(newAccountKeypair.publicKey.toBase58());

  // Create a new account
  const newAccount = await createAccount(
    c,
    payer,
    mintAddress,
    newAccountOwner,
    newAccountKeypair,
  );

  // This address should match the newAccountKeypair.publicKey.
  //
  // And also, you should be able to see those accounts under
  // the Distribution section through [the solana explorer]:
  //
  // [the solana explorer]: https://explorer.solana.com/address/aaoiaaL5DWRngrLUiwpGvLZtFewVdNj5v3YES5SmRxt/largest?cluster=custom&customUrl=http%3A%2F%2F127.0.0.1%3A8899
  console.log(newAccount.toBase58());
})();
