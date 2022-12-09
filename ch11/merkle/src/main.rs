//! Merkle Tree
use hex_literal::hex;
use merkle::TreeBuilder;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::str::FromStr;

const NR_DEPTH: Option<NonZeroUsize> = NonZeroUsize::new(20);
const NR_LEAF: [u8; 32] = hex!("abababababababababababababababababababababababababababababababab");

fn main() {
    let mut args = std::env::args();
    let progname = args.next().map(PathBuf::from).unwrap();
    let depth = args
        .next()
        .as_ref()
        .and_then(|v| NonZeroUsize::from_str(v).ok())
        .unwrap_or(NR_DEPTH.unwrap());

    println!("{:?}: depth={depth}", progname.file_name().unwrap());

    let tree = TreeBuilder::new().initial_leaf(NR_LEAF.into()).build(depth);

    for (i, leave) in tree.leaves().iter().take(4).enumerate() {
        println!("leave[{i}]={:?}", leave);
    }
    if tree.leaves().len() > 4 {
        println!("truncated {} leaves...", tree.leaves().len() - 4);
    }
    println!("tree.root={:?}", tree.root());
}
