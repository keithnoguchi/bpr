//! MarkelTree
use digest::{Digest, Output, OutputSizeUser};
use generic_array::{ArrayLength, GenericArray};
use std::fmt::{self, Debug};
use std::io::{self, Result};
use std::mem;
use std::ops::{Deref, Range};
use tracing::instrument;

type Data<B> = <<B as OutputSizeUser>::OutputSize as ArrayLength<u8>>::ArrayType;

/// MerkleTree.
#[derive(Clone, Debug)]
pub struct MerkleTree<B>
where
    B: Digest,
    Data<B>: Copy,
{
    data: Vec<NodeData<B>>,
    tree_depth: usize,
    leaf_start: usize,
}

impl<B> MerkleTree<B>
where
    B: Digest,
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
    pub fn set(&mut self, index: usize, hash: &[u8]) -> Result<()> {
        let node = self.try_leaf_mut(index)?;
        if node.as_ref() == hash {
            // no change.
            return Ok(());
        }
        *node = NodeData::try_from(hash)?;

        // calculate the merkle root.
        for _ in self.parent_hash_iter_mut(self.leaf_start + index) {}

        Ok(())
    }

    #[instrument(name = "MerkleTree::proof", skip(self), err)]
    pub fn proof(&self, index: usize) -> Result<MerkleProof<B>> {
        let _node = self.try_leaf(index)?;
        Ok(self.proof_iter(self.leaf_start + index).into())
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
        })
    }

    fn try_leaf_mut(&mut self, index: usize) -> Result<&mut NodeData<B>> {
        self.leaves_iter_mut().nth(index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid leaf index: {index}"),
            )
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

    fn proof_iter(&self, index: usize) -> ProofIter<B> {
        ProofIter {
            index,
            data: &self.data,
        }
    }

    fn try_nodes_in_depth_mut(
        &mut self,
        depth: usize,
    ) -> Result<impl Iterator<Item = &mut NodeData<B>>> {
        let range = self.depth_range(depth).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid depth: {depth}"),
            )
        })?;
        Ok(self.data[range.start..range.end].iter_mut())
    }

    fn depth_range(&self, depth: usize) -> Option<Range<usize>> {
        match depth {
            depth if depth > self.tree_depth => None,
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

/// MerkleProof type to be returned by the MerkleTree::proof function.
#[derive(Debug)]
pub struct MerkleProof<B>(Vec<MerkleProofData<B>>)
where
    B: OutputSizeUser;

impl<B> MerkleProof<B>
where
    B: Digest,
{
    pub fn iter(&self) -> impl Iterator<Item = &MerkleProofData<B>> {
        self.0.iter()
    }

    pub fn verify<T>(&self, leaf: T) -> Output<B>
    where
        T: AsRef<[u8]>,
    {
        let mut data: Output<B> = GenericArray::default();
        let mut hash = leaf.as_ref();

        for proof in &self.0 {
            match proof.kind() {
                MerkleProofDataKind::Left => {
                    B::new()
                        .chain_update(hash)
                        .chain_update(proof.sibling())
                        .finalize_into(&mut data);
                }
                MerkleProofDataKind::Right => {
                    B::new()
                        .chain_update(proof.sibling())
                        .chain_update(hash)
                        .finalize_into(&mut data);
                }
            }
            hash = data.as_ref()
        }
        data
    }
}

impl<B> Deref for MerkleProof<B>
where
    B: OutputSizeUser,
{
    type Target = [MerkleProofData<B>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, B> IntoIterator for &'a MerkleProof<B>
where
    B: OutputSizeUser,
{
    type Item = &'a MerkleProofData<B>;
    type IntoIter = std::slice::Iter<'a, MerkleProofData<B>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0[..].iter()
    }
}

impl<B> IntoIterator for MerkleProof<B>
where
    B: OutputSizeUser,
{
    type Item = MerkleProofData<B>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, B> From<ProofIter<'a, B>> for MerkleProof<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    fn from(iter: ProofIter<'a, B>) -> Self {
        Self(iter.collect())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MerkleProofDataKind {
    Left,
    Right,
}

/// MerkleProofData for the merkle proof.
pub struct MerkleProofData<B>(MerkleProofDataKind, Output<B>)
where
    B: OutputSizeUser;

impl<B> MerkleProofData<B>
where
    B: OutputSizeUser,
{
    pub fn kind(&self) -> MerkleProofDataKind {
        self.0
    }

    pub fn sibling(&self) -> &[u8] {
        self.1.as_ref()
    }
}

impl<B> Debug for MerkleProofData<B>
where
    B: OutputSizeUser,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MerkleProofData")
            .field("kind", &self.0)
            .field("sibling", &format_args!("{:02x?}", self.1.as_ref()))
            .finish()
    }
}

/// ProofIter for the merkle proof creation.
#[derive(Debug)]
struct ProofIter<'a, B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    index: usize,
    data: &'a [NodeData<B>],
}

impl<'a, B> Iterator for ProofIter<'a, B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    type Item = MerkleProofData<B>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            return None;
        }
        let (kind, sibling) = if self.index & 1 == 1 {
            (MerkleProofDataKind::Left, &self.data[self.index + 1])
        } else {
            (MerkleProofDataKind::Right, &self.data[self.index - 1])
        };
        self.index = (self.index - 1) / 2;
        Some(MerkleProofData(kind, sibling.into()))
    }
}

struct ParentHashIterMut<'a, B>
where
    B: Digest,
    Data<B>: Copy,
{
    data: &'a mut [NodeData<B>],
}

impl<'a, B> Iterator for ParentHashIterMut<'a, B>
where
    B: Digest,
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

/// Merkle tree node data.
///
/// It provides the convenient way to access the actual hash value
/// through the deref method.  The node is always initialized,
/// e.g., Some(Output<B>) by MerkleTree type.
///
/// It's a private type to provide AsRef<[u8]> to the actual
/// data.
#[derive(Copy)]
struct NodeData<B>(Option<Output<B>>)
where
    B: OutputSizeUser,
    Data<B>: Copy;

impl<B> Clone for NodeData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<B> Default for NodeData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    fn default() -> Self {
        Self(None)
    }
}

impl<B> Debug for NodeData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut f = f.debug_struct("NodeData");
        match self.0 {
            Some(data) => f.field("data", &format_args!("{:02x?}", data.as_ref())),
            None => f.field("data", &"uninitialized"),
        }
        .finish()
    }
}

impl<B> AsRef<[u8]> for NodeData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    fn as_ref(&self) -> &[u8] {
        assert!(self.0.is_some(), "accessing uninitialized node");
        self.0.as_ref().unwrap()
    }
}

impl<B> TryFrom<&[u8]> for NodeData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    type Error = io::Error;

    fn try_from(slice: &[u8]) -> Result<Self> {
        if slice.len() != B::output_size() {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "invalid slice length: {}!={}",
                    slice.len(),
                    B::output_size()
                ),
            ))
        } else {
            Ok(Self(Some(Output::<B>::clone_from_slice(slice))))
        }
    }
}

impl<B> From<&NodeData<B>> for Output<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    fn from(node: &NodeData<B>) -> Output<B> {
        assert!(node.0.is_some(), "accessing uninitialized node");
        node.0.unwrap()
    }
}

impl<B> From<Output<B>> for NodeData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    fn from(inner: Output<B>) -> Self {
        Self(Some(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::{MerkleProofDataKind, MerkleTree};
    use hex_literal::hex;
    use sha3::Sha3_256;

    #[test]
    fn tree_proof_verify() {
        let tree = TreeBuilder::build(5);
        let want = hex!("57054e43fa56333fd51343b09460d48b9204999c376624f52480c5593b91eff4");

        let got = tree.proof(3).unwrap().verify(&[0x33; 32]);
        assert_eq!(got, want.into());
    }

    #[test]
    fn tree_proof() {
        let tree = TreeBuilder::build(5);

        let got = tree.proof(3).unwrap();
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].kind(), MerkleProofDataKind::Right);
        assert_eq!(got[0].sibling(), &[0x22; 32]);
        assert_eq!(got[1].kind(), MerkleProofDataKind::Right);
        assert_eq!(
            got[1].sibling(),
            hex!("35e794f1b42c224a8e390ce37e141a8d74aa53e151c1d1b9a03f88c65adb9e10"),
        );
        assert_eq!(got[2].kind(), MerkleProofDataKind::Left);
        assert_eq!(
            got[2].sibling(),
            hex!("26fca7737f48fa702664c8b468e34c858e62f51762386bd0bddaa7050e0dd7c0"),
        );
        assert_eq!(got[3].kind(), MerkleProofDataKind::Left);
        assert_eq!(
            got[3].sibling(),
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
