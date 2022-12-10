//! MarkelTree
use generic_array::typenum::U32;
use generic_array::GenericArray;
use sha3::{Digest, Sha3_256};
use std::error::Error;
use std::num::NonZeroUsize;
use std::ops::{Deref, Range};
use std::result;
use tracing::{instrument, trace};

type Result<T> = result::Result<T, Box<dyn Error + Send + Sync + 'static>>;
pub type Hash256 = GenericArray<u8, U32>;

pub struct Tree {
    depth_index: usize,
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

    #[instrument(name = "Tree::set", skip(self), err)]
    pub fn set(&mut self, leaf_offset: usize, hash: Hash256) -> Result<()> {
        let leaf_index = index(self.depth_index, 0) + leaf_offset;
        if !self.leaves.contains(&leaf_index) {
            Err("invalid leaf offset")?;
        }
        // sanity check.
        if self.hashes[leaf_index] == Some(hash) {
            return Ok(());
        }
        self.hashes[leaf_index] = Some(hash);

        // calculate the merkle root.
        let mut index = leaf_index;
        while let Some((left, right)) = sibling(index) {
            self.hasher.update(self.hashes[left].unwrap());
            self.hasher.update(self.hashes[right].unwrap());
            let hash = self.hasher.finalize_reset();
            let parent = parent(left).unwrap();
            trace!(
                left_child = %left,
                right_child = %right,
                index = %parent,
                ?hash,
                "hash calculated",
            );
            self.hashes[parent] = Some(hash);
            index = parent;
        }
        Ok(())
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

    pub fn build(mut self, depth: NonZeroUsize) -> Tree {
        let depth_index = usize::from(depth) - 1; // 0 base index.
        let mut tree = Tree {
            depth_index,
            hasher: Sha3_256::new(),
            leaves: index(depth_index, 0)..Self::tree_size(depth),
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
            let parent_hash = tree.hasher.finalize_reset();
            tree.hashes[parent..index]
                .iter_mut()
                .for_each(|hash| *hash = Some(parent_hash));
            index = parent;
        }
        tree
    }

    fn tree_size(depth: NonZeroUsize) -> usize {
        (0x1 << usize::from(depth)) - 1
    }
}

fn index(depth: usize, offset: usize) -> usize {
    let width = 0x1 << depth;
    assert!(offset < width, "invalid offset");
    width - 1 + offset
}

// log2(index)
fn pair(index: usize) -> (usize, usize) {
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

fn parent(index: usize) -> Option<usize> {
    if index == 0 {
        None
    } else {
        Some((index - 1) >> 1)
    }
}

fn sibling(index: usize) -> Option<(usize, usize)> {
    if index == 0 {
        None
    } else if index & 0x1 == 0x1 {
        Some((index, index + 1))
    } else {
        Some((index - 1, index))
    }
}

pub fn base(index: usize) -> usize {
    let (depth, _) = pair(index);
    (0x1 << depth) - 1
}

#[cfg(test)]
mod tests {
    use super::{base, index, pair, parent, sibling, TreeBuilder};
    use hex_literal::hex;
    use std::num::NonZeroUsize;

    #[test]
    fn tree_set() {
        const SAMPLE_LEAF_ZERO: [u8; 32] = [0x00u8; 32];
        const SAMPLE_LEAF_ONE: [u8; 32] = [0x11u8; 32];
        const SAMPLE_ROOT: [u8; 32] =
            hex!("57054e43fa56333fd51343b09460d48b9204999c376624f52480c5593b91eff4");

        let mut tree = TreeBuilder::new()
            .initial_leaf(SAMPLE_LEAF_ZERO.into())
            .build(NonZeroUsize::new(5).unwrap());
        let mut leaves = vec![];
        for i in 0..tree.leaves().len() {
            let leaf = SAMPLE_LEAF_ONE
                .iter()
                .map(|x| *x * i as u8)
                .collect::<super::Hash256>();
            leaves.push(leaf);
        }
        for (i, leaf) in leaves.iter().enumerate() {
            tree.set(i, *leaf).unwrap();
        }
        for (i, leaf) in tree.leaves().iter().enumerate() {
            assert_eq!(leaf.unwrap(), leaves[i]);
        }
        assert_eq!(tree.root().unwrap(), SAMPLE_ROOT.into());
    }

    #[test]
    fn tree_root() {
        const SAMPLE_LEAF: [u8; 32] =
            hex!("abababababababababababababababababababababababababababababababab");
        const SAMPLE_ROOT: [u8; 32] =
            hex!("d4490f4d374ca8a44685fe9471c5b8dbe58cdffd13d30d9aba15dd29efb92930");

        let tree = TreeBuilder::new()
            .initial_leaf(SAMPLE_LEAF.into())
            .build(NonZeroUsize::new(1).unwrap());
        assert_eq!(tree.root(), Some(SAMPLE_LEAF.into()));
        let tree = TreeBuilder::new()
            .initial_leaf(SAMPLE_LEAF.into())
            .build(NonZeroUsize::new(20).unwrap());
        assert_eq!(tree.root(), Some(SAMPLE_ROOT.into()));
    }

    #[test]
    fn tree_len() {
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(1).unwrap())
                .len(),
            1
        );
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(2).unwrap())
                .len(),
            3
        );
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(3).unwrap())
                .len(),
            7
        );
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(4).unwrap())
                .len(),
            15
        );
    }

    #[test]
    fn tree_leaves_len() {
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(1).unwrap())
                .leaves()
                .len(),
            1
        );
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(2).unwrap())
                .leaves()
                .len(),
            2
        );
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(3).unwrap())
                .leaves()
                .len(),
            4
        );
        assert_eq!(
            TreeBuilder::new()
                .build(NonZeroUsize::new(4).unwrap())
                .leaves()
                .len(),
            8
        );
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
    fn test_siblig() {
        assert_eq!(sibling(0), None);
        assert_eq!(sibling(1), Some((1, 2)));
        assert_eq!(sibling(2), Some((1, 2)));
        assert_eq!(sibling(3), Some((3, 4)));
        assert_eq!(sibling(4), Some((3, 4)));
        assert_eq!(sibling(5), Some((5, 6)));
        assert_eq!(sibling(6), Some((5, 6)));
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
