//! MarkelTree
use digest::{Digest, Output, OutputSizeUser};
use generic_array::{ArrayLength, GenericArray};
use std::fmt::{self, Debug};
use std::io::{self, Result};
use std::iter::FromIterator;
use std::mem;
use std::ops::{Deref, Range};

type Data<B> = <<B as OutputSizeUser>::OutputSize as ArrayLength<u8>>::ArrayType;

/// MerkleTree.
#[derive(Clone, Debug)]
pub struct MerkleTree<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    data: Vec<NodeData<B>>,
    leaf_start: usize,
}

impl<B, I> FromIterator<I> for MerkleTree<B>
where
    B: Digest,
    Data<B>: Copy,
    I: AsRef<[u8]>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        // assuming size_hint() returns the correct length for now.
        let iter = iter.into_iter();
        let (leaves, _) = iter.size_hint();
        let mut tree = Self::with_depth(Self::tree_depth(leaves));

        // set leaves.
        let mut leaf_next = tree.leaf_start;
        iter.for_each(|hash| {
            assert!(
                hash.as_ref().len() == <B as Digest>::output_size(),
                "invalid hash length"
            );
            let node = NodeData::try_from(hash.as_ref()).unwrap();
            tree.data[leaf_next] = node;
            leaf_next += 1;
        });
        assert!(
            tree.leaf_start != leaf_next,
            "zero length tree is not supported",
        );

        // make sure the even leaves.
        if !Self::odd_index(leaf_next) {
            tree.data[leaf_next] = tree.data[leaf_next - 1].clone();
            leaf_next += 1;
        }

        // calculate the merkle root.
        let leaf_range = tree.leaf_start..leaf_next;
        for _ in tree.parent_hash_range_iter(leaf_range) {}
        tree
    }
}

impl<B> MerkleTree<B>
where
    B: Digest,
    Data<B>: Copy,
{
    pub fn root(&self) -> &[u8] {
        self.data[0].as_ref()
    }

    pub fn leaves(&self) -> impl Iterator<Item = &[u8]> {
        self.leaves_iter().map(|node| node.as_ref())
    }

    pub fn set(&mut self, index: usize, hash: &[u8]) -> Result<()> {
        let node = self.try_leaf_mut(index)?;
        if let Some(inner) = node.0 {
            if inner.as_ref() == hash {
                // no change.
                return Ok(());
            }
        }
        *node = NodeData::try_from(hash)?;

        // calculate the merkle root.
        let range = match self.leaf_start + index {
            start if Self::odd_index(start) => start..start + 2,
            start => start - 1..start + 1,
        };
        for _ in self.parent_hash_range_iter(range) {}

        Ok(())
    }

    pub fn proof(&self, index: usize) -> Result<MerkleProof<B>> {
        let _node = self.try_leaf(index)?;
        Ok(self.proof_iter(self.leaf_start + index).into())
    }

    fn with_depth(depth: usize) -> Self {
        assert!(depth != 0, "zero depth tree is not supported");
        let tree_size = (1 << depth) - 1;
        let leaf_start = (1 << (depth - 1)) - 1;
        Self {
            data: vec![NodeData::default(); tree_size],
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

    fn parent_hash_range_iter(&mut self, range: Range<usize>) -> ParentHashRangeIter<B> {
        ParentHashRangeIter {
            child_start: range.start,
            data: &mut self.data[..range.end],
        }
    }

    fn proof_iter(&self, index: usize) -> ProofIter<B> {
        ProofIter {
            index,
            data: &self.data,
        }
    }

    #[inline]
    const fn tree_depth(leaves: usize) -> usize {
        match leaves.count_ones() {
            0 => 0,
            1 => (leaves - 1).trailing_ones() as usize + 1,
            _ => {
                let mut depth = 2;
                let mut remain = leaves >> 1;
                while remain > 0 {
                    depth += 1;
                    remain >>= 1;
                }
                depth
            }
        }
    }

    #[inline]
    const fn odd_index(index: usize) -> bool {
        index & 1 == 1
    }
}

/// MerkleProof type to be returned by the MerkleTree::proof function.
#[derive(Clone, Debug)]
pub struct MerkleProof<B>(Vec<MerkleProofData<B>>)
where
    B: OutputSizeUser,
    Data<B>: Copy;

impl<B> MerkleProof<B>
where
    B: Digest,
    Data<B>: Copy,
{
    pub fn iter(&self) -> impl Iterator<Item = &MerkleProofData<B>> {
        self.0.iter()
    }

    pub fn verify<T>(&self, leaf: T) -> impl AsRef<[u8]>
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
    Data<B>: Copy,
{
    type Target = [MerkleProofData<B>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, B> IntoIterator for &'a MerkleProof<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
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
    Data<B>: Copy,
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
#[derive(Copy, Clone)]
pub struct MerkleProofData<B>(MerkleProofDataKind, Output<B>)
where
    B: OutputSizeUser,
    Data<B>: Copy;

impl<B> MerkleProofData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
{
    #[inline]
    pub fn kind(&self) -> MerkleProofDataKind {
        self.0
    }

    #[inline]
    pub fn sibling(&self) -> &[u8] {
        self.1.as_ref()
    }
}

impl<B> Debug for MerkleProofData<B>
where
    B: OutputSizeUser,
    Data<B>: Copy,
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

struct ParentHashRangeIter<'a, B>
where
    B: Digest,
    Data<B>: Copy,
{
    child_start: usize,
    data: &'a mut [NodeData<B>],
}

impl<'a, B> Iterator for ParentHashRangeIter<'a, B>
where
    B: Digest,
    Data<B>: Copy,
{
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() == 1 {
            return None;
        }
        // update the parent hashes.
        let parent_start = (self.child_start - 1) / 2;
        let parent_end = (self.data.len() - 1) / 2;
        let (data, children) = mem::take(&mut self.data).split_at_mut(self.child_start);
        for (i, hashes) in children.chunks(2).enumerate() {
            let mut hasher = B::new();
            for hash in hashes {
                hasher.update(hash);
            }
            data[parent_start + i] = NodeData::from(hasher.finalize());
        }
        // adjust the start and the end index for the next calculation.
        self.child_start = if parent_start != 0 && parent_start & 1 == 0 {
            parent_start - 1
        } else {
            parent_start
        };
        let child_end = if parent_end & 1 == 0 {
            parent_end + 1
        } else {
            parent_end
        };
        // Make sure there is no hole.
        if data[child_end - 1].0.is_none() {
            data[child_end - 1] = data[child_end - 2].clone();
        }
        self.data = &mut data[..child_end];
        Some(parent_start..parent_end)
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
        assert_eq!(got.as_ref(), want);
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
    fn tree_root_from_iter_depth_1() {
        const SAMPLE_LEAF: [u8; 32] = [0xabu8; 32];
        let tree = MerkleTree::<Sha3_256>::from_iter([SAMPLE_LEAF]);
        assert_eq!(tree.root(), &SAMPLE_LEAF);
    }

    #[test]
    fn tree_root_from_iter_depth_15() {
        const SAMPLE_LEAF: [u8; 32] = [0xabu8; 32];
        const SAMPLE_ROOT: [u8; 32] =
            hex!("44ad1490179db284f6fa21d8effbd1ba6a3028042b96be9b249f538de3f57a85");
        let depth = 15;
        let tree: MerkleTree<Sha3_256> = std::iter::repeat(SAMPLE_LEAF)
            .take(1 << (depth - 1))
            .collect();
        assert_eq!(tree.root(), &SAMPLE_ROOT);
    }

    #[test]
    fn tree_root_from_iter_depth_20() {
        const SAMPLE_LEAF: [u8; 32] = [0xabu8; 32];
        const SAMPLE_ROOT: [u8; 32] =
            hex!("d4490f4d374ca8a44685fe9471c5b8dbe58cdffd13d30d9aba15dd29efb92930");
        let depth = 20;
        let tree: MerkleTree<Sha3_256> = std::iter::repeat(SAMPLE_LEAF)
            .take(1 << (depth - 1))
            .collect();
        assert_eq!(tree.root(), &SAMPLE_ROOT);
    }

    #[test]
    fn tree_leaves_count() {
        for depth in 1..=10 {
            let tree: MerkleTree<Sha3_256> =
                std::iter::repeat([0u8; 32]).take(1 << depth - 1).collect();
            let want = 1 << depth - 1;
            assert_eq!(tree.leaves().count(), want);
        }
    }

    struct TreeBuilder;

    impl TreeBuilder {
        fn build(depth: usize) -> MerkleTree<Sha3_256> {
            (0..(1 << (depth - 1)))
                .map(|i| [0x11 * i as u8; 32])
                .collect()
        }
    }
}
