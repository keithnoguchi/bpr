# Blockchain Programming in Rust

Let's write programs, e.g. smart contracts in [Ethereum] parlance,
in [Rust] with [Anchor] for [Solana] [Blockchain].

## Setup

### Solana

As in [Solana CLI] documentation:
```
$ sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
```

The above script will update the `$PATH` environment variable.
```
$ . ~/.profile
```

Double check if you have `solana` in your environment:
```
$ solana --version
solana-cli 1.13.5 (src:959b760c; feat:1365939126)
```

### Anchor

Let's build it from scratch with [cargo] for fun:

```
$ cargo install anchor-cli
```
```
$ anchor --version
anchor-cli 0.25.0
```

[rust]: https://www.rust-lang.org/
[anchor]: https://book.anchor-lang.com/
[solana]: https://solana.com/
[blockchain]: https://en.wikipedia.org/wiki/Blockchain
[ehtereum]: https://ethereum.org/en/
[solana cli]: https://docs.solana.com/cli/install-solana-cli-tools
[cargo]: https://doc.rust-lang.org/cargo/commands/cargo-install.html
