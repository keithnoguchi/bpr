//! Merkle Tree
use merkle::MerkleTree;
use sha3::Sha3_256;
use std::str::FromStr;
use std::sync::Arc;

const NR_DEPTH: usize = 20;
const NR_VERIFIERS: usize = 8;

fn main() {
    let mut args = std::env::args().skip(1);
    let nr_depth = args
        .next()
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_DEPTH);
    let nr_verifiers = args
        .next()
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_VERIFIERS);

    let leaves: Vec<_> = std::iter::repeat([0xabu8; 32])
        .take(1 << (nr_depth - 1))
        .collect();
    let tree = MerkleTree::<Sha3_256>::with_leaves(leaves).unwrap();
    for (i, leave) in tree.leaves().take(4).enumerate() {
        println!("leaf[{i}]={:02x?}", leave);
    }
    if tree.leaves().count() > 4 {
        println!("truncated {} leaves...", tree.leaves().count() - 4);
    }
    let root = tree.root();
    println!("tree.root.len={}", root.len());
    println!("tree.root={:02x?}", root);

    // merkle proof and verification.
    println!("create merkle tree for {} leaves", 1 << (nr_depth - 1));
    let mut leaves = vec![];
    for i in 0..1 << (nr_depth - 1) {
        let hash = [i as u8; 32];
        leaves.push(hash);
    }
    let tree0 = Arc::new(MerkleTree::<Sha3_256>::with_leaves(&leaves).unwrap());

    println!("verify merkle proof for {} leaves", 1 << (nr_depth - 1));
    let chunk = leaves.len() / nr_verifiers;
    let leaves = leaves.into_iter().enumerate().collect::<Vec<_>>();
    let chunks = leaves.chunks(chunk).collect::<Vec<_>>();
    crossbeam::scope(|spawner| {
        for leaves_chunk in chunks {
            let tree = tree0.clone();
            spawner.spawn(move |_| {
                for (i, leaf) in leaves_chunk {
                    let proof = tree.proof(*i).unwrap();
                    assert_eq!(AsRef::<[u8]>::as_ref(&proof.verify(leaf)), tree.root());
                }
            });
        }
    })
    .unwrap();
}
