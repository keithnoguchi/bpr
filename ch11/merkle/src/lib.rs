//! MarkelTree
use digest::{Digest, Output, OutputSizeUser};
use generic_array::ArrayLength;
use std::error::Error;
use std::fmt::{self, Debug};
use std::mem;
use std::ops::{Deref, Range};
use std::result;
use tracing::{instrument, trace, warn};

type Result<T> = result::Result<T, Box<dyn Error + Send + Sync + 'static>>;

pub struct MerkleTree<H>
where
    H: Digest + OutputSizeUser,
    <<H as OutputSizeUser>::OutputSize as ArrayLength<u8>>::ArrayType: Copy,
{
    depth: usize,
    hashes: Vec<Option<Output<H>>>,
}

struct ParentIterMut<'a, H>
where
    H: OutputSizeUser,
{
    hashes: &'a mut [Option<Output<H>>],
}

impl<'a, H> Iterator for ParentIterMut<'a, H>
where
    H: OutputSizeUser,
{
    type Item = &'a mut Option<Output<H>>;

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

impl<H> MerkleTree<H>
where
    H: Digest + OutputSizeUser,
    <<H as OutputSizeUser>::OutputSize as ArrayLength<u8>>::ArrayType: Copy,
{
    /// I'm not convinced with the name of this function, because
    /// giving two parameters a bit confusing.  But also, the previous
    /// builder pattern is too much.
    ///
    /// Let me think about it and come back with the better approach
    /// in the future iteration.
    pub fn with_depth_and_leaf(depth: usize, leaf: Output<H>) -> Self {
        let mut table = Self::with_depth(depth);
        table.leaves_mut().for_each(|node| *node = Some(leaf));

        // calculate the merkle root.
        let mut child_hash = leaf;
        for depth in (1..depth).rev() {
            let parent_hash = H::new()
                .chain_update(&child_hash)
                .chain_update(&child_hash)
                .finalize();
            table
                .try_hashes_in_depth_mut(depth)
                .unwrap()
                .for_each(|node| *node = Some(parent_hash));
            child_hash = parent_hash;
        }
        table
    }

    // Make this associated function private, as it doesn't completely
    // initialize the table.  For example, the following code will panic:
    //
    // ```
    // let mut table = MerkleTree::with_depth(20);
    //
    // table.set(0, [11u8; 32].into());
    // ```
    fn with_depth(depth: usize) -> Self {
        let table_size = (1 << depth) - 1;
        Self {
            depth,
            hashes: vec![None; table_size],
        }
    }

    pub fn root(&self) -> Option<Output<H>> {
        self.hashes[0]
    }

    pub fn leaves(&self) -> impl Iterator<Item = &Option<Output<H>>> {
        if self.depth == 0 {
            [].iter()
        } else {
            let start = (1 << (self.depth - 1)) - 1;
            self.hashes[start..].iter()
        }
    }

    fn leaves_mut(&mut self) -> impl Iterator<Item = &mut Option<Output<H>>> {
        if self.depth == 0 {
            [].iter_mut()
        } else {
            let start = (1 << (self.depth - 1)) - 1;
            self.hashes[start..].iter_mut()
        }
    }

    #[instrument(name = "MerkleTree::set", skip(self), err)]
    pub fn set(&mut self, leaf_offset: usize, hash: Output<H>) -> Result<()> {
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
            let hash = H::new()
                .chain_update(self.hash(left)?)
                .chain_update(self.hash(right)?)
                .finalize();
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

    pub fn proof(&self, leaf_offset: usize) -> Result<Vec<(Position, Output<H>)>> {
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

    fn proof_pair(&self, index: usize) -> Option<(Position, Output<H>)> {
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

    fn try_hashes_in_depth_mut(
        &mut self,
        depth: usize,
    ) -> Result<impl Iterator<Item = &mut Option<Output<H>>>> {
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

    fn hash(&self, index: usize) -> Result<&Output<H>> {
        self.hashes
            .get(index)
            .and_then(|hash| hash.as_ref())
            .ok_or_else(|| "missing hash".into())
    }
}

impl<H> Debug for MerkleTree<H>
where
    H: Digest + OutputSizeUser,
    <<H as OutputSizeUser>::OutputSize as ArrayLength<u8>>::ArrayType: Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MerkleTree")
            .field("depth", &self.depth)
            .field("root", &self.root())
            .field("len", &self.hashes.len())
            .field("leaves_len", &self.leaves().count())
            .finish()
    }
}

impl<H> Deref for MerkleTree<H>
where
    H: Digest + OutputSizeUser,
    <<H as OutputSizeUser>::OutputSize as ArrayLength<u8>>::ArrayType: Copy,
{
    type Target = [Option<Output<H>>];

    fn deref(&self) -> &Self::Target {
        &self.hashes[..]
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
    use super::{MerkleTree, Position};
    use hex_literal::hex;
    use sha3::Sha3_256;
    use std::ops::Range;

    #[test]
    fn tree_proof() {
        let tree = TreeBuilder::build(5);
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
        let tree = TreeBuilder::build(5);
        assert_eq!(tree.root().unwrap(), TreeBuilder::SAMPLE_ROOT.into());
    }

    #[test]
    fn tree_root() {
        const SAMPLE_LEAF: [u8; 32] =
            hex!("abababababababababababababababababababababababababababababababab");
        const SAMPLE_ROOT: [u8; 32] =
            hex!("d4490f4d374ca8a44685fe9471c5b8dbe58cdffd13d30d9aba15dd29efb92930");

        let tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(1, SAMPLE_LEAF.into());
        assert_eq!(tree.root(), Some(SAMPLE_LEAF.into()));
        let tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(20, SAMPLE_LEAF.into());
        assert_eq!(tree.root(), Some(SAMPLE_ROOT.into()));
    }

    #[test]
    fn tree_try_hashes_in_depth_mut() {
        assert_eq!(
            TreeBuilder::build(1)
                .try_hashes_in_depth_mut(0)
                .unwrap()
                .count(),
            1,
        );
        assert_eq!(
            TreeBuilder::build(1)
                .try_hashes_in_depth_mut(1)
                .unwrap()
                .count(),
            1,
        );
        assert!(TreeBuilder::build(1).try_hashes_in_depth_mut(2).is_err());
        assert_eq!(
            TreeBuilder::build(2)
                .try_hashes_in_depth_mut(0)
                .unwrap()
                .count(),
            3,
        );
        assert_eq!(
            TreeBuilder::build(2)
                .try_hashes_in_depth_mut(1)
                .unwrap()
                .count(),
            1,
        );
        assert_eq!(
            TreeBuilder::build(2)
                .try_hashes_in_depth_mut(2)
                .unwrap()
                .count(),
            2,
        );
        assert!(TreeBuilder::build(2).try_hashes_in_depth_mut(3).is_err());
    }

    #[test]
    fn tree_leaf_range() {
        assert_eq!(
            TreeBuilder::build(1).leaf_range(),
            Range { start: 0, end: 1 },
        );
        assert_eq!(
            TreeBuilder::build(2).leaf_range(),
            Range { start: 1, end: 3 },
        );
        assert_eq!(
            TreeBuilder::build(5).leaf_range(),
            Range { start: 15, end: 31 },
        );
    }

    #[test]
    fn tree_len() {
        let leaf = [0u8; 32];
        assert_eq!(
            MerkleTree::<Sha3_256>::with_depth_and_leaf(0, leaf.into()).len(),
            0
        );
        for depth in 1..=10 {
            let want = (1 << depth) - 1;
            assert_eq!(
                MerkleTree::<Sha3_256>::with_depth_and_leaf(depth, leaf.into()).len(),
                want
            );
        }
    }

    #[test]
    fn tree_leaves_count() {
        let leaf = [0u8; 32];
        assert_eq!(
            MerkleTree::<Sha3_256>::with_depth_and_leaf(0, leaf.into())
                .leaves()
                .count(),
            0
        );
        for depth in 1..=10 {
            let want = 1 << depth - 1;
            assert_eq!(
                MerkleTree::<Sha3_256>::with_depth_and_leaf(depth, leaf.into())
                    .leaves()
                    .count(),
                want,
            );
        }
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

    struct TreeBuilder;

    impl TreeBuilder {
        const SAMPLE_ROOT: [u8; 32] =
            hex!("57054e43fa56333fd51343b09460d48b9204999c376624f52480c5593b91eff4");

        fn build(depth: usize) -> MerkleTree<Sha3_256> {
            let leaf = [0u8; 32];
            let mut tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(depth, leaf.into());
            for i in 0..tree.leaves().count() {
                let leaf = [0x11 * i as u8; 32];
                tree.set(i, leaf.into()).unwrap();
            }
            tree
        }
    }
}
