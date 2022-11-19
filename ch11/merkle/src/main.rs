//! Chapter 11: Simplified Payment Verification
use std::path::PathBuf;
use std::str::FromStr;

const NR_LEAVES: usize = 13;

fn main() {
    let mut args = std::env::args();
    let progname = args.next().map(PathBuf::from).unwrap();
    let nr_leaves = args
        .next()
        .as_ref()
        .and_then(|v| usize::from_str(v).ok())
        .unwrap_or(NR_LEAVES);

    println!(
        "{:?}: with {} leaves",
        progname.file_name().unwrap(),
        nr_leaves,
    );

    let tree = merkle::Tree::new(nr_leaves);
    for (i, hash) in tree.nodes().enumerate() {
        println!("node[{i}]={:?}", hash);
    }
    for (i, hash) in tree.leaves().enumerate() {
        println!("leaf[{i}]={:?}", hash);
    }
}
