//! MarkelTree
use generic_array::typenum::U32;
use generic_array::GenericArray;
use sha3::{Digest, Sha3_256};
use std::ops::{Deref, Range};

type Hash256 = GenericArray<u8, U32>;

pub struct Tree {
    hasher: Sha3_256,
    leaves: Range<usize>,
    hashes: Vec<Option<Hash256>>,
}

impl Deref for Tree {
    type Target = [Option<Hash256>];

    fn deref(&self) -> &Self::Target {
        &self.hashes[..]
    }
}

impl Tree {
    pub fn root(&self) -> Option<Hash256> {
        self[0]
    }

    pub fn leaves(&self) -> &[Option<Hash256>] {
        &self.hashes[self.leaves.start..self.leaves.end]
    }
}

#[derive(Default)]
pub struct TreeBuilder {
    initial_leaf: Option<Hash256>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initial_leaf(mut self, hash: Hash256) -> Self {
        self.initial_leaf = Some(hash);
        self
    }

    pub fn build(mut self, depth: usize) -> Tree {
        let mut tree = Tree {
            hasher: Sha3_256::new(),
            leaves: index(depth, 0)..Self::tree_size(depth),
            hashes: vec![None; Self::tree_size(depth)],
        };

        // setup the initial hash.
        let initial_leaf = match self.initial_leaf.take() {
            None => return tree,
            Some(initial_leaf) => initial_leaf,
        };
        let mut index = tree.leaves.start;
        tree.hashes[tree.leaves.start..tree.leaves.end]
            .iter_mut()
            .for_each(|hash| *hash = Some(initial_leaf));

        // calculate parent hashes all the way to the root.
        while let Some(parent) = parent(index) {
            let child = tree.hashes[index].unwrap();
            tree.hasher.update(child);
            tree.hasher.update(child);
            tree.hashes[parent] = Some(tree.hasher.finalize_reset());
            index = parent;
        }
        tree
    }

    fn tree_size(depth: usize) -> usize {
        (0x1 << (depth + 1)) - 1
    }
}

pub fn index(depth: usize, offset: usize) -> usize {
    let width = 0x1 << depth;
    assert!(offset < width, "invalid offset");
    width - 1 + offset
}

pub fn pair(index: usize) -> (usize, usize) {
    // log2(index)
    let mut depth = 0;
    let mut x = index + 1;
    loop {
        x >>= 1;
        if x == 0 {
            break;
        }
        depth += 1;
    }
    let base = (0x1 << depth) - 1;
    let offset = index - base;
    (depth, offset)
}

pub fn parent(index: usize) -> Option<usize> {
    if index == 0 {
        None
    } else {
        Some((index - 1) >> 1)
    }
}

pub fn base(index: usize) -> usize {
    let (depth, _) = pair(index);
    (0x1 << depth) - 1
}

#[cfg(test)]
mod tests {
    use super::{base, index, pair, parent, TreeBuilder};
    use hex_literal::hex;

    const SAMPLE_LEAF: [u8; 32] =
        hex!("abababababababababababababababababababababababababababababababab");
    const SAMPLE_ROOT: [u8; 32] =
        hex!("d4490f4d374ca8a44685fe9471c5b8dbe58cdffd13d30d9aba15dd29efb92930");

    #[test]
    fn tree_root() {
        let tree = TreeBuilder::new().initial_leaf(SAMPLE_LEAF.into()).build(0);
        assert_eq!(tree.root(), Some(SAMPLE_LEAF.into()));
        let tree = TreeBuilder::new()
            .initial_leaf(SAMPLE_LEAF.into())
            .build(19);
        assert_eq!(tree.root(), Some(SAMPLE_ROOT.into()));
    }

    #[test]
    fn tree_len() {
        assert_eq!(TreeBuilder::new().build(0).len(), 1);
        assert_eq!(TreeBuilder::new().build(1).len(), 3);
        assert_eq!(TreeBuilder::new().build(2).len(), 7);
        assert_eq!(TreeBuilder::new().build(3).len(), 15);
    }

    #[test]
    fn tree_leaves_len() {
        assert_eq!(TreeBuilder::new().build(0).leaves().len(), 1);
        assert_eq!(TreeBuilder::new().build(1).leaves().len(), 2);
        assert_eq!(TreeBuilder::new().build(2).leaves().len(), 4);
        assert_eq!(TreeBuilder::new().build(3).leaves().len(), 8);
    }

    #[test]
    fn tree_builder_tree_size() {
        assert_eq!(TreeBuilder::tree_size(0), 1);
        assert_eq!(TreeBuilder::tree_size(1), 3);
        assert_eq!(TreeBuilder::tree_size(2), 7);
        assert_eq!(TreeBuilder::tree_size(3), 15);
        assert_eq!(TreeBuilder::tree_size(4), 31);
    }

    #[test]
    fn test_index() {
        assert_eq!(index(0, 0), 0);
        assert_eq!(index(1, 0), 1);
        assert_eq!(index(1, 1), 2);
        assert_eq!(index(2, 0), 3);
        assert_eq!(index(2, 1), 4);
        assert_eq!(index(2, 2), 5);
        assert_eq!(index(2, 3), 6);
        assert_eq!(index(3, 0), 7);
        assert_eq!(index(3, 1), 8);
        assert_eq!(index(3, 2), 9);
        assert_eq!(index(3, 3), 10);
        assert_eq!(index(3, 4), 11);
        assert_eq!(index(3, 5), 12);
        assert_eq!(index(3, 6), 13);
        assert_eq!(index(3, 7), 14);
    }

    #[test]
    fn test_index_panic() {
        assert!(std::panic::catch_unwind(|| index(0, 1)).is_err());
        assert!(std::panic::catch_unwind(|| index(1, 2)).is_err());
        assert!(std::panic::catch_unwind(|| index(2, 4)).is_err());
        assert!(std::panic::catch_unwind(|| index(3, 8)).is_err());
    }

    #[test]
    fn test_pair() {
        assert_eq!(pair(0), (0, 0));
        assert_eq!(pair(1), (1, 0));
        assert_eq!(pair(2), (1, 1));
        assert_eq!(pair(3), (2, 0));
        assert_eq!(pair(4), (2, 1));
        assert_eq!(pair(5), (2, 2));
        assert_eq!(pair(6), (2, 3));
        assert_eq!(pair(7), (3, 0));
        assert_eq!(pair(8), (3, 1));
        assert_eq!(pair(9), (3, 2));
        assert_eq!(pair(10), (3, 3));
        assert_eq!(pair(11), (3, 4));
        assert_eq!(pair(12), (3, 5));
        assert_eq!(pair(13), (3, 6));
        assert_eq!(pair(14), (3, 7));
    }

    #[test]
    fn test_parent() {
        assert_eq!(parent(0), None);
        assert_eq!(parent(1), Some(0));
        assert_eq!(parent(2), Some(0));
        assert_eq!(parent(3), Some(1));
        assert_eq!(parent(4), Some(1));
        assert_eq!(parent(5), Some(2));
        assert_eq!(parent(6), Some(2));
        assert_eq!(parent(7), Some(3));
        assert_eq!(parent(8), Some(3));
        assert_eq!(parent(9), Some(4));
        assert_eq!(parent(10), Some(4));
        assert_eq!(parent(11), Some(5));
        assert_eq!(parent(12), Some(5));
        assert_eq!(parent(13), Some(6));
        assert_eq!(parent(14), Some(6));
    }

    #[test]
    fn test_base() {
        assert_eq!(base(0), 0);
        assert_eq!(base(1), 1);
        assert_eq!(base(2), 1);
        assert_eq!(base(3), 3);
        assert_eq!(base(4), 3);
        assert_eq!(base(5), 3);
        assert_eq!(base(6), 3);
        assert_eq!(base(7), 7);
        assert_eq!(base(8), 7);
        assert_eq!(base(9), 7);
        assert_eq!(base(10), 7);
        assert_eq!(base(11), 7);
        assert_eq!(base(12), 7);
        assert_eq!(base(13), 7);
        assert_eq!(base(14), 7);
    }
}
