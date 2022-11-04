import * as anchor from "@project-serum/anchor";
import { AnchorError, Program } from "@project-serum/anchor";
import { T3 } from "../target/types/t3";
import { assert, expect } from 'chai';

describe("t3", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.T3 as Program<T3>;

  it('setup game!', async () => {
    const gameKeypair = anchor.web3.Keypair.generate();
    const playerOne = (program.provider as anchor.AnchorProvider).wallet;
    const playerTwo = anchor.web3.Keypair.generate();
    await program.methods
      .setupGame(playerTwo.publicKey)
      .accounts({
        game: gameKeypair.publicKey,
        playerOne: playerOne.publicKey,
      })
      .signers([gameKeypair])
      .rpc();

    let gameState = await program.account.game.fetch(gameKeypair.publicKey);

    expect(gameState.turn).to.equal(1);
    expect(gameState.players).to.eql([playerOne.publicKey, playerTwo.publicKey]);
    expect(gameState.state).to.eql({ active: {} });
    expect(gameState.board).to.eql([
      [null, null, null],
      [null, null, null],
      [null, null, null],
    ]);
  });

  it('player one wins', async () => {
    const gameKeypair = anchor.web3.Keypair.generate();
    const playerOne = program.provider.wallet;
    const playerTwo = anchor.web3.Keypair.generate();
    await program.methods
      .setupGame(playerTwo.publicKey)
      .accounts({
        game: gameKeypair.publicKey,
        playerOne: playerOne.publicKey,
      })
      .signers([gameKeypair])
      .rpc();

    const got = await program.account.game.fetch(gameKeypair.publicKey);
    expect(got.turn).to.equal(1);
    expect(got.players).to.eql([playerOne.publicKey, playerTwo.publicKey]);
    expect(got.state).to.eql({ active: {} })
    expect(got.board).to.eql([
      [null, null, null],
      [null, null, null],
      [null, null, null],
    ]);

    await testPlay(
      program,
      gameKeypair.publicKey,
      playerOne,
      { row: 0, column: 0 },
      2,
      { active: {} },
      [
        [{ x: {} }, null, null],
        [null, null, null],
        [null, null, null],
      ],
    );

    await testPlay(
      program,
      gameKeypair.publicKey,
      playerTwo,
      { row: 1, column: 1 },
      3,
      { active: {} },
      [
        [{ x: {} }, null, null],
        [null, { o: {} }, null],
        [null, null, null],
      ],
    );

    await testPlay(
      program,
      gameKeypair.publicKey,
      playerOne,
      { row: 1, column: 0 },
      4,
      { active: {} },
      [
        [{ x: {} }, null, null],
        [{ x: {} }, { o: {} }, null],
        [null, null, null],
      ],
    );

    await testPlay(
      program,
      gameKeypair.publicKey,
      playerTwo,
      { row: 2, column: 1 },
      5,
      { active: {} },
      [
        [{ x: {} }, null, null],
        [{ x: {} }, { o: {} }, null],
        [null, { o: {} }, null],
      ],
    );

    await testPlay(
      program,
      gameKeypair.publicKey,
      playerOne,
      { row: 2, column: 0 },
      5,
      { won: { winner: playerOne.publicKey }},
      [
        [{ x: {} }, null, null],
        [{ x: {} }, { o: {} }, null],
        [{ x: {} }, { o: {} }, null],
      ],
    );
  });

  it('out of bounds row', async () => {
    const gameKeypair = anchor.web3.Keypair.generate();
    const playerOne = program.provider.wallet;
    const playerTwo = anchor.web3.Keypair.generate();
    await program.methods
      .setupGame(playerTwo.publicKey)
      .accounts({
        game: gameKeypair.publicKey,
        playerOne: playerOne.publicKey,
      })
      .signers([gameKeypair])
      .rpc();

    try {
      await testPlay(
        program,
        gameKeypair.publicKey,
        playerOne,
        { row: 5, column: 1 },
        4,
        { active: {} },
        [
          [null, null, null],
          [null, null, null],
          [null, null, null],
        ],
      )
      assert(false, "should've failed but didn't");
    } catch(_e) {
      expect(_e).to.be.instanceOf(AnchorError);
      const e: AnchorError = _e;
      expect(e.error.errorCode.number).to.equal(6003);
    }
  });

  it('out of order', async () => {
    const gameKeypair = anchor.web3.Keypair.generate();
    const playerOne = program.provider.wallet;
    const playerTwo = anchor.web3.Keypair.generate();
    await program.methods
      .setupGame(playerTwo.publicKey)
      .accounts({
        game: gameKeypair.publicKey,
        playerOne: playerOne.publicKey,
      })
      .signers([gameKeypair])
      .rpc();

    await testPlay(
      program,
      gameKeypair.publicKey,
      playerOne,
      { row: 1, column: 1 },
      2,
      { active: {} },
      [
        [null, null, null],
        [null, { x: {} }, null],
        [null, null, null],
      ],
    );

    try {
      await testPlay(
        program,
        gameKeypair.publicKey,
        playerOne,
        { row: 0, column: 0 },
        2,
        { active: {} },
        [
          [null, null, null],
          [null, { x: {} }, null],
          [null, null, null],
        ],
      );
      assert(false, "should've failed but didn't");
    } catch (_e) {
      expect(_e).to.be.instanceOf(AnchorError);
      const e: AnchorError = _e;
      expect(e.error.errorCode.code).to.equal('NotPlayersTurn');
      expect(e.error.errorCode.number).to.equal(6004);
      expect(e.program.equals(program.programId)).is.true
      expect(e.error.comparedValues).to.deep.equal([
        playerTwo.publicKey,
        playerOne.publicKey,
      ]);
    }
  });
});

async function testPlay(
  program: Program<T3>,
  game,
  player,
  tile,
  wantTurn,
  wantGameState,
  wantBoard,
) {
  await program.methods
    .play(tile)
    .accounts({
      player: player.publicKey,
      game,
    })
    .signers(player instanceof (anchor.Wallet as any) ? [] : [player])
    .rpc();

  const got = await program.account.game.fetch(game);
  expect(got.turn).to.equal(wantTurn);
  expect(got.state).to.eql(wantGameState);
  expect(got.board).to.eql(wantBoard);
}
