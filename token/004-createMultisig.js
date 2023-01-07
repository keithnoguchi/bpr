import { Keypair, Connection } from "@solana/web3.js";
import { createMultisig } from "@solana/spl-token";

const c = new Connection("http://127.0.0.1:8899");
console.log(`cluster rpcEndpoint: ${c.rpcEndpoint}`);

const payer = Keypair.fromSecretKey(
  new Uint8Array([
    24,105,165,60,87,114,114,175,79,208,26,102,237,73,91,42,146,138,17,198,1,81,148,160,78,167,102,195,205,26,25,11,81,255,157,177,9,145,91,131,151,74,120,62,119,170,245,53,79,156,36,166,54,36,130,218,117,14,240,226,22,114,14,25
  ])
);
console.log(`payer address: ${payer.publicKey.toBase58()}`);

// the number of signers, 5 in this example, is just the arbitrary.
const signers = [
  Keypair.generate(),
  Keypair.generate(),
  Keypair.generate(),
  Keypair.generate(),
  Keypair.generate(),
];
for (const [i, signer] of signers.entries()) {
  console.log(`signer#${i} address: ${signer.publicKey.toBase58()}`);
}

// This number should be less than signers.length.
const m = 3;
console.assert(m < signers.length);

(async () => {
  const multisigAccountAddress = await createMultisig(
    c,
    payer,
    [
      signers[0].publicKey,
      signers[1].publicKey,
      signers[2].publicKey,
      signers[3].publicKey,
      signers[4].publicKey,
    ],
    m,
  );
  console.log(`multisig account address: ${multisigAccountAddress.toBase58()}`);
})();
