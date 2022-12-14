//! MarkelTree
use generic_array::typenum::U32;
use generic_array::GenericArray;
use sha3::{Digest, Sha3_256};
use std::error::Error;
use std::fmt::{self, Debug};
use std::mem;
use std::ops::{Deref, Range};
use std::result;
use tracing::{instrument, trace, warn};

type Result<T> = result::Result<T, Box<dyn Error + Send + Sync + 'static>>;
pub type Hash256 = GenericArray<u8, U32>;

pub struct MerkleTree {
    depth: usize,
    hasher: Sha3_256,
    hashes: Vec<Option<Hash256>>,
}

/// Deref gives the slice of leave hashes.
impl Deref for MerkleTree {
    type Target = [Option<Hash256>];

    fn deref(&self) -> &Self::Target {
        if self.depth == 0 {
            &[]
        } else {
            let start = (1 << (self.depth - 1)) - 1;
            &self.hashes[start..]
        }
    }
}

impl Debug for MerkleTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MerkleTree")
            .field("depth", &self.depth)
            .field("root", &self.root())
            .field("leaves.len()", &self.len())
            .field("hashes.len()", &self.hashes().count())
            .finish()
    }
}

struct ParentIterMut<'a> {
    hashes: &'a mut [Option<Hash256>],
}

impl<'a> Iterator for ParentIterMut<'a> {
    type Item = &'a mut Option<Hash256>;

    // as in [rustomicon]
    //
    // [rustomicon]: https://doc.rust-lang.org/nomicon/borrow-splitting.html
    fn next(&mut self) -> Option<Self::Item> {
        mem::take(&mut self.hashes)
            .split_last_mut()
            .map(|(parent, hashes)| {
                self.hashes = hashes;
                parent
            })
    }
}

impl MerkleTree {
    pub fn with_depth(depth: usize) -> Self {
        let table_size = (1 << depth) - 1;
        Self {
            depth,
            hasher: Sha3_256::new(),
            hashes: vec![None; table_size],
        }
    }

    pub fn size(&self) -> usize {
        self.hashes.len()
    }

    pub fn root(&self) -> Option<Hash256> {
        self.hashes[0]
    }

    #[inline]
    pub fn leaves(&self) -> impl Iterator<Item = &Option<Hash256>> {
        self.iter()
    }

    fn leaves_mut(&mut self) -> impl Iterator<Item = &mut Option<Hash256>> {
        let range = self.leaf_range();
        self.hashes[range.start..range.end].iter_mut()
    }

    fn hashes(&self) -> impl Iterator<Item = &Option<Hash256>> {
        self.hashes[..].iter()
    }

    #[instrument(name = "MerkleTree::set", skip(self), err)]
    pub fn set(&mut self, leaf_offset: usize, hash: Hash256) -> Result<()> {
        let leaf = match self.leaves_mut().nth(leaf_offset) {
            None => Err("invalid leaf offset")?,
            Some(leaf) => leaf,
        };
        if *leaf == Some(hash) {
            // no change.
            return Ok(());
        }
        *leaf = Some(hash);

        // calculate the merkle root.
        let mut index = self.leaf_range().start + leaf_offset;
        while let Some((left, right)) = siblings(index) {
            let mut hasher = self.hasher.clone();
            hasher.update(self.hash(left)?);
            hasher.update(self.hash(right)?);
            let hash = hasher.finalize_reset();
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

    pub fn proof(&self, leaf_offset: usize) -> Result<Vec<(Position, Hash256)>> {
        let leaf_index = self.leaf_index(leaf_offset)?;
        let mut proof = vec![];
        match self.proof_pair(leaf_index) {
            None => Err("missing hash for the leaf pair")?,
            Some(pair) => {
                proof.push(pair);
            }
        }
        for ancester in ancesters(leaf_index) {
            match self.proof_pair(ancester) {
                Some(pair) => proof.push(pair),
                None => break,
            }
        }
        Ok(proof)
    }

    fn proof_pair(&self, index: usize) -> Option<(Position, Hash256)> {
        sibling(index)
            .and_then(|sibling| self.hashes[sibling])
            .map(|hash| (Position::from(index), hash))
    }

    fn leaf_index(&self, leaf_offset: usize) -> Result<usize> {
        let leaf_range = self.leaf_range();
        let leaf_index = leaf_range.start + leaf_offset;
        if !leaf_range.contains(&leaf_index) {
            Err("invalid leaf offset")?;
        }
        Ok(leaf_index)
    }

    #[inline]
    fn leaf_range(&self) -> Range<usize> {
        self.depth_range(self.depth).unwrap()
    }

    fn set_initial_leaf(&mut self, initial_leaf: &Hash256) {
        self.leaves_mut()
            .for_each(|leaf| *leaf = Some(*initial_leaf));

        // update all the way to the root.
        let mut child_hash = *initial_leaf;
        for depth in (1..self.depth).rev() {
            self.hasher.update(child_hash);
            self.hasher.update(child_hash);
            let hash = self.hasher.finalize_reset();
            // it's safe to unwrap here as depth is in the valid range.
            self.try_hashes_in_depth_mut(depth)
                .unwrap()
                .for_each(|node| *node = Some(hash));
            child_hash = hash;
        }
    }

    fn try_hashes_in_depth_mut(
        &mut self,
        depth: usize,
    ) -> Result<impl Iterator<Item = &mut Option<Hash256>>> {
        let range = self.depth_range(depth).ok_or("invalid depth")?;
        Ok(self.hashes[range.start..range.end].iter_mut())
    }

    fn depth_range(&self, depth: usize) -> Option<Range<usize>> {
        match depth {
            depth if depth > self.depth => {
                warn!(tree.depth = %self.depth, "invalid depth");
                None
            }
            0 => Some(Range {
                start: 0,
                end: self.hashes.len(),
            }),
            _ => {
                let start = (1 << (depth - 1)) - 1;
                let end = (1 << depth) - 1;
                Some(Range { start, end })
            }
        }
    }

    fn hash(&self, index: usize) -> Result<&Hash256> {
        self.hashes
            .get(index)
            .and_then(|hash| hash.as_ref())
            .ok_or_else(|| "missing hash".into())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Position {
    Root,
    Left,
    Right,
}

impl From<usize> for Position {
    fn from(index: usize) -> Self {
        if index == 0 {
            Self::Root
        } else if index & 0x1 == 0x1 {
            Self::Left
        } else {
            Self::Right
        }
    }
}

#[derive(Default)]
pub struct MerkleTreeBuilder {
    initial_leaf: Option<Hash256>,
}

impl MerkleTreeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initial_leaf(mut self, hash: Hash256) -> Self {
        self.initial_leaf = Some(hash);
        self
    }

    pub fn build(mut self, depth: usize) -> MerkleTree {
        // setup the initial hash.
        let mut tree = MerkleTree::with_depth(depth);
        match self.initial_leaf.take() {
            None => tree,
            Some(initial_leaf) => {
                tree.set_initial_leaf(&initial_leaf);
                tree
            }
        }
    }
}

pub fn index(depth: usize, offset: usize) -> usize {
    let width = 0x1 << depth;
    assert!(offset < width, "invalid offset");
    width - 1 + offset
}

fn depth_and_offset(index: usize) -> (usize, usize) {
    // log2(index)
    let mut depth = 0;
    let mut x = (index + 1) >> 1;
    while x != 0 {
        depth += 1;
        x >>= 1;
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

fn sibling(index: usize) -> Option<usize> {
    match Position::from(index) {
        Position::Root => None,
        Position::Left => Some(index + 1),
        Position::Right => Some(index - 1),
    }
}

fn siblings(index: usize) -> Option<(usize, usize)> {
    sibling(index).map(|sibling| {
        if index < sibling {
            (index, sibling)
        } else {
            (sibling, index)
        }
    })
}

fn ancesters(mut index: usize) -> Vec<usize> {
    let mut ancesters = vec![];
    while let Some(parent) = parent(index) {
        ancesters.push(parent);
        index = parent;
    }
    ancesters
}

pub fn base(index: usize) -> usize {
    let (_, offset) = depth_and_offset(index);
    index - offset
}

#[cfg(test)]
mod tests {
    use super::{ancesters, parent, sibling, siblings};
    use super::{base, depth_and_offset, index};
    use super::{MerkleTree, MerkleTreeBuilder, Position};
    use hex_literal::hex;
    use std::ops::Range;

    #[test]
    fn tree_proof() {
        let tree = TestTreeBuilder::build(5);

        let got = tree.proof(3).unwrap();
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].0, Position::Right);
        assert_eq!(got[0].1, [0x22; 32].into());
        assert_eq!(got[1].0, Position::Right);
        assert_eq!(
            got[1].1,
            hex!("35e794f1b42c224a8e390ce37e141a8d74aa53e151c1d1b9a03f88c65adb9e10").into(),
        );
        assert_eq!(got[2].0, Position::Left);
        assert_eq!(
            got[2].1,
            hex!("26fca7737f48fa702664c8b468e34c858e62f51762386bd0bddaa7050e0dd7c0").into(),
        );
        assert_eq!(got[3].0, Position::Left);
        assert_eq!(
            got[3].1,
            hex!("e7e11a86a0c1d8d8624b1629cb58e39bb4d0364cb8cb33c4029662ab30336858").into(),
        );
    }

    #[test]
    fn tree_set() {
        let tree = TestTreeBuilder::build(5);
        assert_eq!(tree.root().unwrap(), TestTreeBuilder::SAMPLE_ROOT.into());
    }

    #[test]
    fn tree_root() {
        const SAMPLE_LEAF: [u8; 32] =
            hex!("abababababababababababababababababababababababababababababababab");
        const SAMPLE_ROOT: [u8; 32] =
            hex!("d4490f4d374ca8a44685fe9471c5b8dbe58cdffd13d30d9aba15dd29efb92930");

        let tree = MerkleTreeBuilder::new()
            .initial_leaf(SAMPLE_LEAF.into())
            .build(1);
        assert_eq!(tree.root(), Some(SAMPLE_LEAF.into()));
        let tree = MerkleTreeBuilder::new()
            .initial_leaf(SAMPLE_LEAF.into())
            .build(20);
        assert_eq!(tree.root(), Some(SAMPLE_ROOT.into()));
    }

    #[test]
    fn tree_leaves_count() {
        assert_eq!(TestTreeBuilder::build(1).leaves().count(), 1);
        assert_eq!(TestTreeBuilder::build(2).leaves().count(), 2);
        assert_eq!(TestTreeBuilder::build(3).leaves().count(), 4);
        assert_eq!(TestTreeBuilder::build(4).leaves().count(), 8);
    }

    #[test]
    fn tree_hashes_count() {
        assert_eq!(TestTreeBuilder::build(1).hashes().count(), 1);
        assert_eq!(TestTreeBuilder::build(2).hashes().count(), 3);
        assert_eq!(TestTreeBuilder::build(3).hashes().count(), 7);
        assert_eq!(TestTreeBuilder::build(4).hashes().count(), 15);
        assert_eq!(TestTreeBuilder::build(5).hashes().count(), 31);
    }

    #[test]
    fn tree_try_hashes_in_depth_mut() {
        assert_eq!(
            TestTreeBuilder::build(1)
                .try_hashes_in_depth_mut(0)
                .unwrap()
                .count(),
            1,
        );
        assert_eq!(
            TestTreeBuilder::build(1)
                .try_hashes_in_depth_mut(1)
                .unwrap()
                .count(),
            1,
        );
        assert!(TestTreeBuilder::build(1)
            .try_hashes_in_depth_mut(2)
            .is_err());
        assert_eq!(
            TestTreeBuilder::build(2)
                .try_hashes_in_depth_mut(0)
                .unwrap()
                .count(),
            3,
        );
        assert_eq!(
            TestTreeBuilder::build(2)
                .try_hashes_in_depth_mut(1)
                .unwrap()
                .count(),
            1,
        );
        assert_eq!(
            TestTreeBuilder::build(2)
                .try_hashes_in_depth_mut(2)
                .unwrap()
                .count(),
            2,
        );
        assert!(TestTreeBuilder::build(2)
            .try_hashes_in_depth_mut(3)
            .is_err());
    }

    #[test]
    fn tree_leaf_range() {
        assert_eq!(
            TestTreeBuilder::build(1).leaf_range(),
            Range { start: 0, end: 1 },
        );
        assert_eq!(
            TestTreeBuilder::build(2).leaf_range(),
            Range { start: 1, end: 3 },
        );
        assert_eq!(
            TestTreeBuilder::build(5).leaf_range(),
            Range { start: 15, end: 31 },
        );
    }

    #[test]
    fn tree_len() {
        assert_eq!(MerkleTree::with_depth(0).len(), 0);
        for depth in 1..=20 {
            let want = 1 << (depth - 1);
            assert_eq!(MerkleTree::with_depth(depth).len(), want);
        }
    }

    #[test]
    fn tree_size() {
        assert_eq!(MerkleTree::with_depth(0).size(), 0);
        assert_eq!(MerkleTree::with_depth(1).size(), 1);
        assert_eq!(MerkleTree::with_depth(2).size(), 3);
        assert_eq!(MerkleTree::with_depth(3).size(), 7);
        assert_eq!(MerkleTree::with_depth(4).size(), 15);
        assert_eq!(MerkleTree::with_depth(5).size(), 31);
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
    fn test_depth_and_offset() {
        assert_eq!(depth_and_offset(0), (0, 0));
        assert_eq!(depth_and_offset(1), (1, 0));
        assert_eq!(depth_and_offset(2), (1, 1));
        assert_eq!(depth_and_offset(3), (2, 0));
        assert_eq!(depth_and_offset(4), (2, 1));
        assert_eq!(depth_and_offset(5), (2, 2));
        assert_eq!(depth_and_offset(6), (2, 3));
        assert_eq!(depth_and_offset(7), (3, 0));
        assert_eq!(depth_and_offset(8), (3, 1));
        assert_eq!(depth_and_offset(9), (3, 2));
        assert_eq!(depth_and_offset(10), (3, 3));
        assert_eq!(depth_and_offset(11), (3, 4));
        assert_eq!(depth_and_offset(12), (3, 5));
        assert_eq!(depth_and_offset(13), (3, 6));
        assert_eq!(depth_and_offset(14), (3, 7));
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
        assert_eq!(sibling(1), Some(2));
        assert_eq!(sibling(2), Some(1));
        assert_eq!(sibling(3), Some(4));
        assert_eq!(sibling(4), Some(3));
        assert_eq!(sibling(5), Some(6));
        assert_eq!(sibling(6), Some(5));
    }

    #[test]
    fn test_sibligs() {
        assert_eq!(siblings(0), None);
        assert_eq!(siblings(1), Some((1, 2)));
        assert_eq!(siblings(2), Some((1, 2)));
        assert_eq!(siblings(3), Some((3, 4)));
        assert_eq!(siblings(4), Some((3, 4)));
        assert_eq!(siblings(5), Some((5, 6)));
        assert_eq!(siblings(6), Some((5, 6)));
    }

    #[test]
    fn test_ancesters() {
        assert_eq!(ancesters(0), vec![]);
        assert_eq!(ancesters(1), vec![0]);
        assert_eq!(ancesters(2), vec![0]);
        assert_eq!(ancesters(3), vec![1, 0]);
        assert_eq!(ancesters(4), vec![1, 0]);
        assert_eq!(ancesters(5), vec![2, 0]);
        assert_eq!(ancesters(6), vec![2, 0]);
        assert_eq!(ancesters(7), vec![3, 1, 0]);
        assert_eq!(ancesters(8), vec![3, 1, 0]);
        assert_eq!(ancesters(9), vec![4, 1, 0]);
        assert_eq!(ancesters(10), vec![4, 1, 0]);
        assert_eq!(ancesters(11), vec![5, 2, 0]);
        assert_eq!(ancesters(12), vec![5, 2, 0]);
        assert_eq!(ancesters(13), vec![6, 2, 0]);
        assert_eq!(ancesters(14), vec![6, 2, 0]);
        assert_eq!(ancesters(15), vec![7, 3, 1, 0]);
        assert_eq!(ancesters(16), vec![7, 3, 1, 0]);
        assert_eq!(ancesters(17), vec![8, 3, 1, 0]);
        assert_eq!(ancesters(18), vec![8, 3, 1, 0]);
        assert_eq!(ancesters(19), vec![9, 4, 1, 0]);
        assert_eq!(ancesters(20), vec![9, 4, 1, 0]);
        assert_eq!(ancesters(21), vec![10, 4, 1, 0]);
        assert_eq!(ancesters(22), vec![10, 4, 1, 0]);
        assert_eq!(ancesters(23), vec![11, 5, 2, 0]);
        assert_eq!(ancesters(24), vec![11, 5, 2, 0]);
        assert_eq!(ancesters(25), vec![12, 5, 2, 0]);
        assert_eq!(ancesters(26), vec![12, 5, 2, 0]);
        assert_eq!(ancesters(27), vec![13, 6, 2, 0]);
        assert_eq!(ancesters(28), vec![13, 6, 2, 0]);
        assert_eq!(ancesters(29), vec![14, 6, 2, 0]);
        assert_eq!(ancesters(30), vec![14, 6, 2, 0]);
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

    struct TestTreeBuilder;

    impl TestTreeBuilder {
        const SAMPLE_ROOT: [u8; 32] =
            hex!("57054e43fa56333fd51343b09460d48b9204999c376624f52480c5593b91eff4");

        fn build(depth: usize) -> MerkleTree {
            let mut tree = MerkleTreeBuilder::new()
                .initial_leaf([0u8; 32].into())
                .build(depth);
            for i in 0..tree.len() {
                let leaf = [0x11 * i as u8; 32];
                tree.set(i, leaf.into()).unwrap();
            }
            tree
        }
    }
}
