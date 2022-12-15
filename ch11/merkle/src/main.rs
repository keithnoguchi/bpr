//! Merkle Tree
use merkle::MerkleTree;
use std::str::FromStr;
use tracing::{info, instrument, warn};

const NR_DEPTH: usize = 20;
const NR_ZERO_HASH: [u8; 32] = [0; 32];
const NR_LEAF_HASH: [u8; 32] = [0xab; 32];

fn main() {
    tracing_subscriber::fmt::init();
    let depth = std::env::args()
        .nth(1)
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_DEPTH);

    let tree = MerkleTree::with_depth_and_leaf(depth, NR_LEAF_HASH.into());
    println!("{tree:?}");

    for (i, leave) in tree.leaves().take(4).enumerate() {
        println!("leaf[{i}]={:x?}", leave);
    }
    if tree.leaves().count() > 4 {
        println!("truncated {} leaves...", tree.leaves().count() - 4);
    }
    println!("tree.root={:x?}", tree.root());

    if depth < 6 {
        set_leaves(depth)
    }
}

#[instrument]
fn set_leaves(depth: usize) {
    let mut tree = MerkleTree::with_depth_and_leaf(depth, NR_ZERO_HASH.into());

    // set the leaves.
    for i in 0..tree.len() {
        let hash = [0x11u8 * i as u8; 32];
        tree.set(i, hash.into()).unwrap();
    }

    // print out the leaves.
    for (i, leaf) in tree.iter().enumerate() {
        match leaf {
            None => warn!("missing leaf[offset={i}]"),
            Some(leaf) => info!("leaf[{i}]={:02x?}", leaf),
        }
    }

    // print out the root.
    match tree.root() {
        None => warn!("missing root"),
        Some(root) => info!("tree.root={:02x?}", root),
    }
}
