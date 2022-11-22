//! Merkle Tree
const NR_HASH_SIZE: usize = 32;

pub type Hash = [u8; NR_HASH_SIZE];

pub struct Builder {
    leaves: Vec<Hash>,
}

impl Builder {
    pub fn new() -> Self {
        let leaves = vec![];
        Self { leaves }
    }

    pub fn push(&mut self, hash: Hash) -> &mut Self {
        self.leaves.push(hash);
        self
    }

    pub fn build(&mut self) -> Tree {
        assert!(!self.leaves.is_empty());
        if self.leaves.len() / 2 == 1 {
            self.leaves.push(self.leaves[self.leaves.len() - 1]);
        }
        Tree::new(self.leaves.len())
    }
}

#[derive(Debug)]
pub struct Tree {
    hashes: Vec<Option<Hash>>,
    leaves: usize,
}

impl Tree {
    pub fn new(leaves: usize) -> Self {
        assert!(leaves % 1 == 0);
        let (leaves, total) = match leaves {
            0 => (0, 0),
            1 => (1, 1),
            n => {
                let leaves = if n & 1 == 1 { n + 1 } else { n };
                let depth = (leaves as f64).log2().ceil() as u32;
                let total = 2usize.pow(depth) - 1 + leaves;
                (leaves, total)
            }
        };
        let hashes = vec![None; total];
        Self { hashes, leaves }
    }

    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    pub fn root(&self) -> Node {
        Node {
            tree: self,
            index: 0,
        }
    }

    pub fn nodes(&self) -> Node {
        self.root()
    }

    pub fn leaves(&self) -> Node {
        let index = self.hashes.len() - self.leaves;
        Node { tree: self, index }
    }

    pub fn node(&self, index: usize) -> Option<Node> {
        if index < self.hashes.len() {
            Some(Node { tree: self, index })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Node<'a> {
    tree: &'a Tree,
    index: usize,
}

impl<'a> Iterator for Node<'a> {
    type Item = Option<Hash>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.tree.len() {
            let hash = self.tree.hashes[self.index];
            self.index += 1;
            Some(hash)
        } else {
            None
        }
    }
}

impl<'a> Node<'a> {
    pub fn is_root(&self) -> bool {
        self.index == 0
    }

    pub fn is_leaf(&self) -> bool {
        self.index * 2 + 1 >= self.tree.len()
    }

    pub fn up(&self) -> Option<Self> {
        if self.is_root() {
            None
        } else {
            let index = (self.index - 1) / 2;
            Some(Node {
                tree: self.tree,
                index,
            })
        }
    }

    pub fn left(&self) -> Option<Self> {
        if self.is_leaf() {
            None
        } else {
            let index = self.index * 2 + 1;
            Some(Node {
                tree: self.tree,
                index,
            })
        }
    }

    pub fn right(&self) -> Option<Self> {
        if self.is_leaf() {
            None
        } else {
            let index = self.index * 2 + 2;
            Some(Node {
                tree: self.tree,
                index,
            })
        }
    }

    pub fn hash(&self) -> Option<Hash> {
        self.tree.hashes[self.index]
    }
}

#[cfg(test)]
mod tests {
    use super::Tree;

    #[test]
    fn tree_len() {
        assert_eq!(Tree::new(0).len(), 0);
        assert_eq!(Tree::new(1).len(), 1);
        assert_eq!(Tree::new(2).len(), 3);
        assert_eq!(Tree::new(3).len(), 7);
        assert_eq!(Tree::new(4).len(), 7);
        assert_eq!(Tree::new(5).len(), 13);
        assert_eq!(Tree::new(6).len(), 13);
        assert_eq!(Tree::new(8).len(), 15);
        assert_eq!(Tree::new(9).len(), 25);
    }

    #[test]
    fn node_up() {
        let tree = Tree::new(15);
        assert!(tree.root().up().is_none());
        assert!(tree.node(1).and_then(|n| n.up()).is_some());
    }
}
