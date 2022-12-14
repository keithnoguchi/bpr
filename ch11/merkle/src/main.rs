//! Merkle Tree
use std::path::PathBuf;
use std::str::FromStr;
use tracing::{info, instrument, warn};

const NR_DEPTH: usize = 20;
const NR_LEAF: [u8; 32] = [0xab; 32];

fn main() {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args();
    let progname = args.next().map(PathBuf::from).unwrap();
    let depth = args
        .next()
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_DEPTH);

    println!("{:?}: depth={depth}", progname.file_name().unwrap());

    let tree = merkle::TreeBuilder::new()
        .initial_leaf(NR_LEAF.into())
        .build(depth);
    println!("{tree:?}");

    for (i, leave) in tree.leaves().take(4).enumerate() {
        println!("leaf[{i}]={:x?}", leave);
    }
    if tree.leaves().count() > 4 {
        println!("truncated {} leaves...", tree.leaves().count() - 4);
    }
    println!("tree.root={:x?}", tree.root());

    let tree = merkle::TreeBuilder::new()
        .initial_leaf([0u8; 32].into())
        .build(depth);
    if depth < 6 {
        set_leaves(tree)
    }
}

#[instrument(skip(tree))]
fn set_leaves(mut tree: merkle::Tree) {
    println!("\nset_leaves\n");

    // set the leaves.
    (0..tree.leaves().count()).for_each(|i| {
        let hash = [0x11u8 * i as u8; 32];
        tree.set(i, hash.into()).unwrap();
    });

    // print out the leaves.
    for (i, leaf) in tree.leaves().enumerate() {
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
