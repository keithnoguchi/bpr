//! MarkelTree
use digest::{Digest, Output, OutputSizeUser};
use generic_array::ArrayLength;
use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::mem;
use std::ops::Range;
use std::result;
use tracing::{instrument, warn};

type Data<B> = <<B as OutputSizeUser>::OutputSize as ArrayLength<u8>>::ArrayType;
type Result<T> = result::Result<T, Box<dyn Error + Send + Sync + 'static>>;

/// MerkleTree.
#[derive(Debug)]
pub struct MerkleTree<B>
where
    B: Debug + Digest + OutputSizeUser,
    Data<B>: Copy,
{
    data: Vec<NodeData<B>>,
    tree_depth: usize,
    leaf_start: usize,
}

impl<B> MerkleTree<B>
where
    B: Debug + Digest + OutputSizeUser,
    Data<B>: Copy,
{
    /// I'm not convinced with the name of this function, because
    /// giving two parameters a bit confusing.  But also, the previous
    /// builder pattern is too much.
    ///
    /// Let me think about it and come back with the better approach
    /// in the future iteration.
    pub fn with_depth_and_leaf(depth: usize, hash: &[u8]) -> Result<Self> {
        let mut tree = Self::with_depth(depth);

        // set the leaf node first with the provided hash.
        let mut node = NodeData::try_from(hash)?;
        tree.leaves_iter_mut().for_each(|leaf| *leaf = node.clone());

        // then calculate the merkle root.
        for depth in (1..depth).rev() {
            let parent =
                NodeData::from(B::new().chain_update(&node).chain_update(&node).finalize());
            tree.try_nodes_in_depth_mut(depth)
                .unwrap()
                .for_each(|node| *node = parent.clone());
            node = parent;
        }
        Ok(tree)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    /// Panic when called on the empty tree.
    pub fn root(&self) -> &[u8] {
        self.data[0].as_ref()
    }

    /// Panic when called on the empty tree.
    pub fn leaves(&self) -> impl Iterator<Item = &[u8]> {
        self.leaves_iter().map(|node| node.as_ref())
    }

    #[instrument(name = "MerkleTree::set", skip(self), err)]
    pub fn set(&mut self, leaf_offset: usize, hash: &[u8]) -> Result<()> {
        let node = self.try_leaf_mut(leaf_offset)?;
        if node.as_ref() == hash {
            // no change.
            return Ok(());
        }
        *node = NodeData::try_from(hash)?;

        // calculate the merkle root.
        for _ in self.parent_hash_iter_mut(self.leaf_start + leaf_offset) {}

        Ok(())
    }

    pub fn proof(&self, leaf_offset: usize) -> Result<MerkleProofIter<B>> {
        let _node = self.try_leaf(leaf_offset)?;
        Ok(self.merkle_proof_iter(self.leaf_start + leaf_offset))
    }

    // Make this associated function private, as it doesn't completely
    // initialize the table.  For example, the following code will panic:
    //
    // ```
    // let mut table = MerkleTree::with_depth(20);
    //
    // table.set(0, [11u8; 32]);
    // ```
    fn with_depth(tree_depth: usize) -> Self {
        let tree_size = (1 << tree_depth) - 1;
        let leaf_start = if tree_depth == 0 {
            0
        } else {
            (1 << (tree_depth - 1)) - 1
        };
        Self {
            data: vec![NodeData::default(); tree_size],
            tree_depth,
            leaf_start,
        }
    }

    fn try_leaf(&self, index: usize) -> Result<&NodeData<B>> {
        self.leaves_iter().nth(index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid leaf index: {index}"),
            )
            .into()
        })
    }

    fn try_leaf_mut(&mut self, index: usize) -> Result<&mut NodeData<B>> {
        self.leaves_iter_mut().nth(index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid leaf index: {index}"),
            )
            .into()
        })
    }

    fn leaves_iter(&self) -> impl Iterator<Item = &NodeData<B>> {
        self.data[self.leaf_start..].iter()
    }

    fn leaves_iter_mut(&mut self) -> impl Iterator<Item = &mut NodeData<B>> {
        self.data[self.leaf_start..].iter_mut()
    }

    fn parent_hash_iter_mut(&mut self, index: usize) -> ParentHashIterMut<B> {
        let index = if index & 1 == 1 { index + 1 } else { index };
        assert!(index < self.data.len(), "invalid child index");
        ParentHashIterMut {
            data: &mut self.data[..=index],
        }
    }

    fn merkle_proof_iter(&self, index: usize) -> MerkleProofIter<B> {
        MerkleProofIter {
            index,
            data: &self.data,
        }
    }

    fn try_nodes_in_depth_mut(
        &mut self,
        depth: usize,
    ) -> Result<impl Iterator<Item = &mut NodeData<B>>> {
        let range = self.depth_range(depth).ok_or("invalid depth")?;
        Ok(self.data[range.start..range.end].iter_mut())
    }

    fn depth_range(&self, depth: usize) -> Option<Range<usize>> {
        match depth {
            depth if depth > self.tree_depth => {
                warn!(tree.depth = %self.tree_depth, "invalid depth");
                None
            }
            0 => Some(Range {
                start: 0,
                end: self.data.len(),
            }),
            _ => {
                let start = (1 << (depth - 1)) - 1;
                let end = (1 << depth) - 1;
                Some(Range { start, end })
            }
        }
    }
}

/// MerkleProofIter for the merkle proof.
#[derive(Debug)]
pub struct MerkleProofIter<'a, B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    index: usize,
    data: &'a [NodeData<B>],
}

impl<'a, B> Iterator for MerkleProofIter<'a, B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    type Item = MerkleNode<'a, B>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            return None;
        }
        let (kind, start, end) = if self.index & 1 == 1 {
            (MerkleNodeKind::Left, self.index, self.index + 1)
        } else {
            (MerkleNodeKind::Right, self.index - 1, self.index)
        };
        self.index = (self.index - 1) / 2;
        Some(MerkleNode {
            kind,
            data: &self.data[start..=end],
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MerkleNodeKind {
    Root,
    Left,
    Right,
}

#[derive(Debug)]
pub struct MerkleNode<'a, B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    kind: MerkleNodeKind,
    data: &'a [NodeData<B>],
}

impl<'a, B> MerkleNode<'a, B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    pub fn kind(&self) -> MerkleNodeKind {
        self.kind
    }

    pub fn data(&self) -> &[u8] {
        match self.kind {
            MerkleNodeKind::Root | MerkleNodeKind::Left => self.data[0].as_ref(),
            MerkleNodeKind::Right => self.data[1].as_ref(),
        }
    }

    pub fn sibling_data(&self) -> Option<&[u8]> {
        match self.kind {
            MerkleNodeKind::Root => None,
            MerkleNodeKind::Left => Some(self.data[1].as_ref()),
            MerkleNodeKind::Right => Some(self.data[0].as_ref()),
        }
    }
}

/// Merkle tree node data.
///
/// It provides the convenient way to access the actual hash value
/// through the deref method.  The node is always initialized,
/// e.g., Some(Output<B>) by MerkleTree type.
///
/// It's a private type to provide AsRef<[u8]> to the actual
/// data.
#[derive(Copy, Debug)]
struct NodeData<B>(Option<Output<B>>)
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy;

impl<B> Clone for NodeData<B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<B> Default for NodeData<B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    fn default() -> Self {
        Self(None)
    }
}

impl<B> AsRef<[u8]> for NodeData<B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    fn as_ref(&self) -> &[u8] {
        assert!(self.0.is_some(), "accessing uninitialized node");
        self.0.as_ref().unwrap()
    }
}

impl<B> TryFrom<&[u8]> for NodeData<B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    type Error = &'static str;

    fn try_from(slice: &[u8]) -> result::Result<Self, Self::Error> {
        if slice.len() != B::output_size() {
            return Err("invalid slice length");
        }
        Ok(Self(Some(Output::<B>::clone_from_slice(slice))))
    }
}

impl<B> From<&NodeData<B>> for Output<B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    fn from(node: &NodeData<B>) -> Output<B> {
        assert!(node.0.is_some(), "accessing uninitialized node");
        node.0.unwrap()
    }
}

impl<B> From<Output<B>> for NodeData<B>
where
    B: Debug + OutputSizeUser,
    Data<B>: Copy,
{
    fn from(inner: Output<B>) -> Self {
        Self(Some(inner))
    }
}

struct ParentHashIterMut<'a, B>
where
    B: Debug + Digest + OutputSizeUser,
    Data<B>: Copy,
{
    data: &'a mut [NodeData<B>],
}

impl<'a, B> Iterator for ParentHashIterMut<'a, B>
where
    B: Debug + Digest + OutputSizeUser,
    Data<B>: Copy,
{
    type Item = Output<B>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }

        // get the left and right child.
        assert!(self.data.len() >= 3, "invalid index calculation");
        let (right, data) = mem::take(&mut self.data).split_last_mut().unwrap();
        let (left, data) = data.split_last_mut().unwrap();

        // calculate the parent hash.
        let parent_index = (data.len() - 1) / 2;
        let hash = B::new().chain_update(&left).chain_update(&right).finalize();
        data[parent_index] = NodeData::from(hash);

        // update the data in the iterator.
        self.data = if parent_index == 0 {
            &mut []
        } else if parent_index & 1 == 1 {
            &mut data[..=parent_index + 1]
        } else {
            &mut data[..=parent_index]
        };

        Some(hash)
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

pub fn parent(index: usize) -> Option<usize> {
    if index == 0 {
        None
    } else {
        Some((index - 1) >> 1)
    }
}

pub fn base(index: usize) -> usize {
    let (_, offset) = depth_and_offset(index);
    index - offset
}

#[cfg(test)]
mod tests {
    use super::{base, depth_and_offset, index, parent};
    use super::{MerkleNodeKind, MerkleTree};
    use hex_literal::hex;
    use sha3::Sha3_256;

    #[test]
    fn tree_proof() {
        let tree = TreeBuilder::build(5);

        let got: Vec<_> = tree.proof(3).unwrap().collect();
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].kind(), MerkleNodeKind::Right);
        assert_eq!(got[0].sibling_data().unwrap(), &[0x22; 32]);
        assert_eq!(got[1].kind(), MerkleNodeKind::Right);
        assert_eq!(
            got[1].sibling_data().unwrap(),
            hex!("35e794f1b42c224a8e390ce37e141a8d74aa53e151c1d1b9a03f88c65adb9e10"),
        );
        assert_eq!(got[2].kind(), MerkleNodeKind::Left);
        assert_eq!(
            got[2].sibling_data().unwrap(),
            hex!("26fca7737f48fa702664c8b468e34c858e62f51762386bd0bddaa7050e0dd7c0"),
        );
        assert_eq!(got[3].kind(), MerkleNodeKind::Left);
        assert_eq!(
            got[3].sibling_data().unwrap(),
            hex!("e7e11a86a0c1d8d8624b1629cb58e39bb4d0364cb8cb33c4029662ab30336858"),
        );
    }

    #[test]
    fn tree_root() {
        const SAMPLE_LEAF: [u8; 32] =
            hex!("abababababababababababababababababababababababababababababababab");
        const SAMPLE_ROOT: [u8; 32] =
            hex!("d4490f4d374ca8a44685fe9471c5b8dbe58cdffd13d30d9aba15dd29efb92930");

        let tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(1, &SAMPLE_LEAF).unwrap();
        assert_eq!(tree.root(), &SAMPLE_LEAF);
        let tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(20, &SAMPLE_LEAF).unwrap();
        assert_eq!(tree.root(), &SAMPLE_ROOT);
    }

    #[test]
    fn tree_try_nodes_in_depth_mut() {
        assert_eq!(
            TreeBuilder::build(1)
                .try_nodes_in_depth_mut(0)
                .unwrap()
                .count(),
            1,
        );
        assert_eq!(
            TreeBuilder::build(1)
                .try_nodes_in_depth_mut(1)
                .unwrap()
                .count(),
            1,
        );
        assert!(TreeBuilder::build(1).try_nodes_in_depth_mut(2).is_err());
        assert_eq!(
            TreeBuilder::build(2)
                .try_nodes_in_depth_mut(0)
                .unwrap()
                .count(),
            3,
        );
        assert_eq!(
            TreeBuilder::build(2)
                .try_nodes_in_depth_mut(1)
                .unwrap()
                .count(),
            1,
        );
        assert_eq!(
            TreeBuilder::build(2)
                .try_nodes_in_depth_mut(2)
                .unwrap()
                .count(),
            2,
        );
        assert!(TreeBuilder::build(2).try_nodes_in_depth_mut(3).is_err());
    }

    #[test]
    fn tree_len() {
        assert_eq!(MerkleTree::<Sha3_256>::with_depth(0).len(), 0);
        for depth in 1..=10 {
            let want = (1 << depth) - 1;
            assert_eq!(MerkleTree::<Sha3_256>::with_depth(depth).len(), want);
        }
    }

    #[test]
    fn tree_leaves_count() {
        let leaf = [0u8; 32];
        assert_eq!(
            MerkleTree::<Sha3_256>::with_depth_and_leaf(0, &leaf)
                .unwrap()
                .leaves()
                .count(),
            0,
        );
        for depth in 1..=10 {
            let want = 1 << depth - 1;
            assert_eq!(
                MerkleTree::<Sha3_256>::with_depth_and_leaf(depth, &leaf)
                    .unwrap()
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
        fn build(depth: usize) -> MerkleTree<Sha3_256> {
            let leaf = [0u8; 32];
            let mut tree = MerkleTree::<Sha3_256>::with_depth_and_leaf(depth, &leaf).unwrap();
            for i in 0..tree.leaves().count() {
                let leaf = [0x11 * i as u8; 32];
                tree.set(i, &leaf).unwrap();
            }
            tree
        }
    }
}
