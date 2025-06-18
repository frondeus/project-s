use std::sync::Arc;

use tree_sitter::Parser as TSParser;

#[derive(Debug, thiserror::Error)]
#[error("Tree-sitter error: {0}")]
pub struct TreeSitterError(String);

pub struct Node(usize, usize);

/// A forest is a collection of trees.
pub struct Forest {
    pub trees: Vec<CST>,
}

impl Forest {
    pub fn new_tree(
        &mut self,
        source: Arc<str>,
        filename: Arc<str>,
    ) -> Result<&mut CST, TreeSitterError> {
        let generation = self.trees.len();
        let cst = CST::new(generation, source, filename)?;
        self.trees.push(cst);
        Ok(&mut self.trees[generation])
    }

    pub fn get_node(&self, node: Node) -> Option<tree_sitter::Node<'_>> {
        let cst = self.trees.get(node.1)?;
        cst.get(node)
    }

    pub fn get_mut(&mut self, generation: usize) -> Option<&mut CST> {
        self.trees.get_mut(generation)
    }
}

pub struct CST {
    pub source: Arc<str>,
    pub filename: Arc<str>,
    pub tree: tree_sitter::Tree,

    raw: Vec<tree_sitter::ffi::TSNode>,
    generation: usize,
}

impl CST {
    pub fn new(
        generation: usize,
        source: Arc<str>,
        filename: Arc<str>,
    ) -> Result<Self, TreeSitterError> {
        let mut parser = TSParser::new();
        parser
            .set_language(&tree_sitter_s::LANGUAGE.into())
            .map_err(|e| TreeSitterError(e.to_string()))?;

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or_else(|| TreeSitterError("Failed to parse input".to_string()))?;

        Ok(Self {
            source,
            filename,

            tree,
            raw: vec![],
            generation,
        })
    }

    pub fn add(&mut self, node: tree_sitter::Node<'_>) -> Node {
        let raw = node.into_raw();
        let node = Node(self.raw.len(), self.generation);
        self.raw.push(raw);
        node
    }

    pub fn get(&self, node: Node) -> Option<tree_sitter::Node<'_>> {
        assert_eq!(self.generation, node.1);

        let raw = *self.raw.get(node.0)?;
        let node = unsafe { tree_sitter::Node::from_raw(raw) };
        Some(node)
    }
}
