# Blockchain Programming in Rust

[ethereum]: https://ethereum.org/en/
[rust]: https://www.rust-lang.org/
[anchor]: https://book.anchor-lang.com/
[solana]: https://solana.com/
[blockchain]: https://en.wikipedia.org/wiki/Blockchain

Let's write programs, e.g. smart contracts in [Ethereum] parlance,
in [Rust] with [Anchor] for [Solana] [Blockchain].

## Programs

[solana-program]: https://lib.rs/crates/solana-program
[rust program quickstart guide]: https://docs.solana.com/getstarted/rust
[solana labs]: https://github.com/solana-labs/example-helloworld
[paulx]: https://paulx.dev/blog/2021/01/14/programming-on-solana-an-introduction/
[tic-tac-toe project]: https://www.anchor-lang.com/docs/tic-tac-toe

- [A Solana Counter Program](solana-counter/program/src/lib.rs)
  - A native solana counter program by [Solana Labs].
    - [Counter up transactions](solana-counter/scripts/src/main.ts)
- [A Solana Escrow Program](solana-escrow/program/src/processor.rs)
  - A native solana escrow program by [paulx].
    - [Setup transactions](solana-escrow/scripts/src/setup.ts)
    - [Alice's InitEscrow transaction](solana-escrow/scripts/src/alice.ts)
    - [Bob's Exchange transaction](solana-escrow/scripts/src/bob.ts)
- [token](token)
  - [createMint](token/001-createMint.js)
  - [createAccount](token/002-createAccount.js)
  - [closeAccount](token/003-closeAccount.js)
  - [createMultisig](token/004-createMultisig.js)
- [Calculator](calc/programs/calc/src/lib.rs)
- [Tic-Tac-Toe](t3/programs/t3/src/lib.rs)
  - Refer to [Anchor]'s [Tic-Tac-Toe Project].

## Setup

### Solana Localhost Blockchain Cluster

[solana local development]: https://docs.solana.com/getstarted/local

Let's run the local cluster by following the [solana local development]
document.

### Solana Program Libray (SPL)

[solana cli]: https://docs.solana.com/cli/install-solana-cli-tools
[spl token cli]: https://lib.rs/crates/spl-token

Let's install [SPL Token CLI]:

```
$ cargo install spl-token-cli
```

### Anchor

[cargo]: https://doc.rust-lang.org/cargo/commands/cargo-install.html

Install anchor cli through [cargo]:

```
$ cargo install anchor-cli
```
```
$ anchor --version
anchor-cli 0.25.0
```

### Airdrop to your Wallet

Let's create a command line wallet for testing:

```
$ solana airdrop 2 --url devnet $(solana-keygen pubkey)
```
double check if you got airdropped.

```
$ solana balance --url devnet
2 SOL
```

## Primitives

[jimmy song]: https://programmingbitcoin.com/
[programming bitcoin]: https://programmingbitcoin.com/programming-bitcoin-book/
[learning merkel tree]: https://github.com/melekes/merkle-tree-rs/
[learning merkel tree 2]: https://dev.to/msedzins/learning-rust-merkel-tree-9p

Here is the old content regarding the primitives through [Jimmy Song]'s
wonderful [Programming Bitcoin] in Rust.

- [Merkle Tree](ch11/merkle/src/lib.rs)

Happy Hacking!
