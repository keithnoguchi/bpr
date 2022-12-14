//! Merkle Tree
use merkle::MerkleTree;
use sha3::Sha3_256;
use std::str::FromStr;
use std::sync::Arc;

const NR_LEAVES: usize = 524_288; // 2 ^ 19, e.g. 20 depth tree.
const NR_VERIFIERS: usize = 8;

fn main() {
    let mut args = std::env::args().skip(1);
    let nr_leaves = args
        .next()
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_LEAVES);
    let nr_verifiers = args
        .next()
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_VERIFIERS);

    let tree: MerkleTree<Sha3_256> = std::iter::repeat([0xabu8; 32]).take(nr_leaves).collect();
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
    println!("create merkle tree for {} leaves", nr_leaves);
    let mut leaves = vec![];
    for i in 0..nr_leaves {
        leaves.push([i as u8; 32])
    }
    let tree0: Arc<MerkleTree<Sha3_256>> = Arc::new(leaves.iter().collect());

    println!("verify merkle proof for {} leaves", nr_leaves);
    let chunk = if nr_leaves > nr_verifiers {
        nr_leaves / nr_verifiers
    } else {
        nr_leaves
    };
    let leaves = leaves.into_iter().enumerate().collect::<Vec<_>>();
    let chunks = leaves.chunks(chunk).collect::<Vec<_>>();
    crossbeam::scope(|spawner| {
        for leaves_chunk in chunks {
            let tree = tree0.clone();
            spawner.spawn(move |_| {
                for (i, leaf) in leaves_chunk {
                    let proof = tree.proof(*i).unwrap();
                    assert_eq!(proof.verify(leaf).as_ref(), tree.root());
                }
            });
        }
    })
    .unwrap();
}
