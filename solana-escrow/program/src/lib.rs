//! A native solana escrow program by [paulx].
//!
//! [paulx]: https://paulx.dev/blog/2021/01/14/programming-on-solana-an-introduction/

#![forbid(missing_docs, missing_debug_implementations)]

mod error;
mod instruction;
mod processor;
mod state;

/// An entry point of this program.
#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
