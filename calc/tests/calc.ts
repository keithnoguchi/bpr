import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Calc } from "../target/types/calc";
import { expect } from "chai";

describe("calc", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Calc as Program<Calc>;
  const wallet = (program.provider as anchor.AnchorProvider).wallet;

  it("creation", async () => {
    const calcKeypair = anchor.web3.Keypair.generate();

    await program.methods
      .create("Welcome to Solana!")
      .accounts({
        calculator: calcKeypair.publicKey,
        user: wallet.publicKey,
        system_program: program.programId,
      })
      .signers([calcKeypair])
      .rpc();

    const got = await program
      .account
      .calculator
      .fetch(calcKeypair.publicKey);
    expect(got.greeting).to.equal("Welcome to Solana!");
  });

  it("addition", async () => {
    const calcKeypair = anchor.web3.Keypair.generate();
    await program.methods
      .create("addition test")
      .accounts({
        calculator: calcKeypair.publicKey,
        user: wallet.publicKey,
        system_program: program.programId,
      })
      .signers([calcKeypair])
      .rpc();

    await program.methods
      .add(new anchor.BN(1), new anchor.BN(2))
      .accounts({
        calculator: calcKeypair.publicKey,
      })
      .rpc();

    const got = await program
      .account
      .calculator
      .fetch(calcKeypair.publicKey);
    expect(got.result).to.eql(new anchor.BN(3));
  })

  it("subtraction", async () => {
    const calcKeypair = anchor.web3.Keypair.generate();
    await program.methods
      .create("subtraction test")
      .accounts({
        calculator: calcKeypair.publicKey,
        user: wallet.publicKey,
        system_program: program.programId,
      })
      .signers([calcKeypair])
      .rpc();

    await program.methods
      .sub(new anchor.BN(1), new anchor.BN(9))
      .accounts({
        calculator: calcKeypair.publicKey,
      })
      .rpc();

    const got = await program.account.calculator.fetch(calcKeypair.publicKey);
    expect(got.result).to.eql(new anchor.BN(-8));
  });

  it("multiplication", async () => {
    const calcKeypair = anchor.web3.Keypair.generate();
    await program.methods
      .create("multiplication test")
      .accounts({
        calculator: calcKeypair.publicKey,
        user: wallet.publicKey,
        system_program: program.programId,
      })
      .signers([calcKeypair])
      .rpc();

    await program.methods
      .mul(new anchor.BN(-19), new anchor.BN(-8))
      .accounts({
        calculator: calcKeypair.publicKey,
      })
      .rpc();

    const got = await program
      .account
      .calculator
      .fetch(calcKeypair.publicKey);
    expect(got.result).to.eql(new anchor.BN(152));
  });

  it("division", async () => {
    const calcKeypair = anchor.web3.Keypair.generate();
    await program.methods
      .create("division test")
      .accounts({
        calculator: calcKeypair.publicKey,
        user: wallet.publicKey,
        system_program: program.programId,
      })
      .signers([calcKeypair])
      .rpc();

    await program.methods
      .div(new anchor.BN(-19), new anchor.BN(-8))
      .accounts({
        calculator: calcKeypair.publicKey,
      })
      .rpc();

    const got = await program
      .account
      .calculator
      .fetch(calcKeypair.publicKey);
    expect(got.result).to.eql(new anchor.BN(2));
    expect(got.remainder).to.eql(new anchor.BN(3));
  });
});
