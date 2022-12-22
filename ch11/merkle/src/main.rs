//! Merkle Tree
use merkle::MerkleTree;
use sha3::Sha3_256;
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

    let tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(depth, &NR_LEAF_HASH).unwrap();
    for (i, leave) in tree.leaves().take(4).enumerate() {
        println!("leaf[{i}]={:02x?}", leave);
    }
    if tree.leaves().count() > 4 {
        println!("truncated {} leaves...", tree.leaves().count() - 4);
    }
    let root = tree.root();
    println!("tree.root.len={}", root.len());
    println!("tree.root={:02x?}", root);

    if depth < 6 {
        set_leaves(depth)
    }
}

#[instrument]
fn set_leaves(depth: usize) {
    let mut tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(depth, &NR_ZERO_HASH).unwrap();

    // set the leaves.
    for i in 0..tree.leaves().count() {
        let hash = [0x11u8 * i as u8; 32];
        tree.set(i, &hash).unwrap();
    }

    // print out the leaves.
    for (i, leaf) in tree.leaves().enumerate() {
        info!("leaf[{i}]={:02x?}", leaf)
    }

    // print out the root.
    info!("tree.node={:02x?}", tree.root());

    // print out the merkle proof
    for proof in tree.proof(0).unwrap() {
        info!("proof={:?}", proof);
    }
}
