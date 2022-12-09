//! Merkle Tree
use hex_literal::hex;
use merkle::TreeBuilder;
use std::path::PathBuf;
use std::str::FromStr;

const NR_DEPTH: usize = 20;
const NR_LEAF: [u8; 32] = hex!("abababababababababababababababababababababababababababababababab");

fn main() {
    let mut args = std::env::args();
    let progname = args.next().map(PathBuf::from).unwrap();
    let depth = args
        .next()
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_DEPTH);

    println!("{:?}: depth={depth}", progname.file_name().unwrap());

    let tree = TreeBuilder::new().initial_leaf(NR_LEAF.into()).build(depth);

    println!("{:?}", tree.root());
}
