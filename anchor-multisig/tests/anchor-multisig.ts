import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { AnchorMultisig } from "../target/types/anchor_multisig";

describe("anchor-multisig", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.AnchorMultisig as Program<AnchorMultisig>;

  it("creates multisig account", async () => {
    const multisig = anchor.web3.Keypair.generate();

    // Get the nonce for the PDA based on the multisig
    // address.  The nonce is stored in the multisig
    // data account and the multisigSigner will be used
    // for the transaction creation below.
    const [multisigSigner, nonce] =
      await anchor.web3.PublicKey.findProgramAddress(
        [multisig.publicKey.toBuffer()],
        program.programId
      );

    // We don't need this big size for the multisig
    // account.  I'll come back to this size later.
    const multisigSize = 200;

    // 3/5 multisig example.
    const ownerA = anchor.web3.Keypair.generate();
    const ownerB = anchor.web3.Keypair.generate();
    const ownerC = anchor.web3.Keypair.generate();
    const ownerD = anchor.web3.Keypair.generate();
    const ownerE = anchor.web3.Keypair.generate();
    const owners = [
      ownerA.publicKey,
      ownerB.publicKey,
      ownerC.publicKey,
      ownerD.publicKey,
      ownerE.publicKey,
    ];
    const threshold = new anchor.BN(3);

    const tx = await program.rpc.createMultisig(owners, threshold, nonce, {
      accounts: {
        multisig: multisig.publicKey,
      },
      instructions: [
        await program.account.multisig.createInstruction(
          multisig,
          multisigSize
        ),
      ],
      signers: [multisig],
    });

    console.log("Multisig account had been created", tx);
  });
});
