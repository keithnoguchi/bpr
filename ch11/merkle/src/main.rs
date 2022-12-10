//! Merkle Tree
use hex_literal::hex;
use merkle::{Tree, TreeBuilder};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::{info, instrument};

const NR_DEPTH: Option<NonZeroUsize> = NonZeroUsize::new(20);
const NR_LEAF: [u8; 32] = hex!("abababababababababababababababababababababababababababababababab");

fn main() {
    tracing_subscriber::fmt::init();
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
        println!("leave[{i}]={:x?}", leave);
    }
    if tree.leaves().len() > 4 {
        println!("truncated {} leaves...", tree.leaves().len() - 4);
    }
    println!("tree.root={:x?}", tree.root());

    const LEAF_ZERO: [u8; 32] = [0u8; 32];
    let tree = TreeBuilder::new()
        .initial_leaf(LEAF_ZERO.into())
        .build(depth);
    if depth.get() < 6 {
        set_leaves(tree)
    }
}

#[instrument(skip(tree))]
fn set_leaves(mut tree: Tree) {
    println!("\nset_leaves\n");
    const SAMPLE_LEAF_ONE: [u8; 32] = [0x11u8; 32];
    let mut leaves = vec![];
    for i in 0..tree.leaves().len() {
        let leaf = SAMPLE_LEAF_ONE
            .iter()
            .map(|x| *x * i as u8)
            .collect::<merkle::Hash256>();
        leaves.push(leaf);
    }
    for (i, leaf) in leaves.iter().enumerate() {
        tree.set(i, *leaf).unwrap();
    }
    for (i, leaf) in tree.leaves().iter().enumerate() {
        info!("leaves[{i}]={:02x?}", leaf);
    }
    info!("tree.root={:02x?}", tree.root());
}
