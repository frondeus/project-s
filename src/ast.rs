use std::{collections::HashMap, fmt};

use tree_sitter::Parser as TSParser;

#[derive(Debug, Default)]
pub struct ASTS {
    asts: HashMap<usize, AST>,
    generation: usize,
}

impl ASTS {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_ast_by_generation(&mut self, generation: usize) -> &mut AST {
        self.asts.get_mut(&generation).unwrap()
    }

    pub fn get_ast(&self, id: SExpId) -> &AST {
        self.asts.get(&id.generation()).unwrap()
    }

    pub fn get(&self, id: SExpId) -> &SExp {
        let ast = self.get_ast(id);
        ast.get(id)
    }

    pub fn maybe_get(&self, id: Option<SExpId>) -> Option<&SExp> {
        Some(self.get(id?))
    }

    pub fn new_ast(&mut self) -> AST {
        let ast = AST::new(self.generation);
        self.generation += 1;
        ast
    }

    pub fn add_ast(&mut self, ast: AST) -> usize {
        let generation = ast.generation();
        self.asts.insert(generation, ast);
        generation
    }

    pub fn fmt(&self, id: SExpId) -> SExpFmt<'_> {
        self.get(id).fmt(self)
    }

    pub fn fmt_list<'a>(&'a self, list: &'a [SExpId]) -> SExpFmtList<'a> {
        SExpFmtList { list, asts: self }
    }

    pub fn parse(&mut self, input: &str) -> Result<&AST, ParseError> {
        let parser = SExpParser::new(self)?;
        let ast = parser.parse(input)?;
        let root = ast.root_id().unwrap();
        self.add_ast(ast);
        Ok(self.get_ast(root))
    }
}

#[derive(Debug)]
pub struct AST {
    generation: usize,
    nodes: Vec<SExp>,
}

impl AST {
    fn new(generation: usize) -> Self {
        Self {
            generation,
            nodes: Vec::new(),
        }
    }
}

impl AST {
    fn new_id(&self, id: usize) -> SExpId {
        SExpId {
            id,
            generation: self.generation,
        }
    }

    pub fn add_node(&mut self, node: SExp) -> SExpId {
        let id = self.nodes.len();
        self.nodes.push(node);
        self.new_id(id)
    }

    pub fn reserve(&mut self) -> SExpId {
        self.add_node(SExp::Error)
    }

    pub fn set(&mut self, id: SExpId, node: SExp) {
        assert_eq!(id.generation, self.generation);
        self.nodes[id.id] = node;
    }

    pub fn get(&self, id: SExpId) -> &SExp {
        assert_eq!(id.generation, self.generation);
        &self.nodes[id.id]
    }

    pub fn maybe_get(&self, id: Option<SExpId>) -> Option<&SExp> {
        id.map(|id| self.get(id))
    }

    pub fn nodes(&self) -> &[SExp] {
        &self.nodes
    }

    pub fn root(&self) -> Option<&SExp> {
        self.nodes.first()
    }

    pub fn root_id(&self) -> Option<SExpId> {
        self.nodes.first().map(|_| self.new_id(0))
    }

    pub fn generation(&self) -> usize {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SExpId {
    id: usize,
    generation: usize,
}

impl SExpId {
    pub fn generation(self) -> usize {
        self.generation
    }
}

#[derive(Debug, Clone)]
pub enum SExp {
    Number(f64),
    String(String),
    Bool(bool),
    Symbol(String),
    Keyword(String), // Symbol that starts with :
    List(Vec<SExpId>),

    Error,
}

impl SExp {
    pub fn fmt<'a>(&'a self, asts: &'a ASTS) -> SExpFmt<'a> {
        SExpFmt { asts, expr: self }
    }

    pub fn as_keyword(&self) -> Option<&str> {
        match self {
            SExp::Keyword(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            SExp::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_symbol_or_keyword(&self) -> Option<&str> {
        match self {
            SExp::Symbol(s) => Some(s),
            SExp::Keyword(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_symbol_or_keyword_or_string(&self) -> Option<&str> {
        match self {
            SExp::Symbol(s) => Some(s),
            SExp::Keyword(s) => Some(s),
            SExp::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[SExpId]> {
        match self {
            SExp::List(list) => Some(list),
            _ => None,
        }
    }
}

pub struct SExpFmt<'a> {
    asts: &'a ASTS,
    expr: &'a SExp,
}

pub struct SExpFmtList<'a> {
    list: &'a [SExpId],
    asts: &'a ASTS,
}

impl std::fmt::Display for SExpFmtList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, item) in self.list.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            let item = self.asts.get(*item).fmt(self.asts);
            write!(f, "{}", item)?;
        }
        write!(f, ")")
    }
}

impl std::fmt::Debug for SExpFmtList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(
                self.list
                    .iter()
                    .map(|item| self.asts.get(*item).fmt(self.asts)),
            )
            .finish()
    }
}

impl std::fmt::Debug for SExpFmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expr {
            SExp::Number(n) => f.debug_tuple("Number").field(n).finish(),
            SExp::String(s) => f.debug_tuple("String").field(s).finish(),
            SExp::Symbol(s) => f.debug_tuple("Symbol").field(s).finish(),
            SExp::Keyword(s) => f.debug_tuple("Keyword").field(s).finish(),
            SExp::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            SExp::Error => f.debug_tuple("Error").finish(),
            SExp::List(items) => f
                .debug_tuple("List")
                .field(&SExpFmtList {
                    asts: self.asts,
                    list: items,
                })
                .finish(),
        }
    }
}
impl std::fmt::Display for SExpFmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expr {
            SExp::Number(n) => write!(f, "{}", n),
            SExp::String(s) => write!(f, "\"{}\"", s),
            SExp::Symbol(s) => write!(f, "{}", s),
            SExp::Keyword(s) => write!(f, ":{}", s),
            SExp::Bool(b) => write!(f, "{}", b),
            SExp::Error => write!(f, "<Error>"),
            SExp::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    let item = self.asts.get(*item).fmt(self.asts);
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),
    #[error("Unexpected node: {0}")]
    UnexpectedNode(String),
}

pub struct SExpParser {
    parser: TSParser,
    ast: AST,
}

impl SExpParser {
    pub fn new(asts: &mut ASTS) -> Result<Self, ParseError> {
        let mut parser = TSParser::new();
        parser
            .set_language(&tree_sitter_s::LANGUAGE.into())
            .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;

        Ok(SExpParser {
            parser,
            ast: asts.new_ast(),
        })
    }

    #[allow(clippy::only_used_in_recursion)]
    fn node_to_sexp(
        &mut self,
        node: tree_sitter::Node,
        source: &str,
    ) -> Result<SExpId, ParseError> {
        match node.kind() {
            "float" | "integer" => {
                let text = node
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                let value = text
                    .parse::<f64>()
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(self.ast.add_node(SExp::Number(value)))
            }
            "boolean" => {
                let text = node
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                let value = text == "true";
                Ok(self.ast.add_node(SExp::Bool(value)))
            }
            "string" => {
                let inner = node
                    .child_by_field_name("inner")
                    .ok_or_else(|| ParseError::TreeSitterError("No inner node".to_string()))?;
                let text = inner
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(self.ast.add_node(SExp::String(text.to_string())))
            }
            "keyword" => {
                let text = node
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                let text = text.trim_start_matches(':');
                Ok(self.ast.add_node(SExp::Keyword(text.to_string())))
            }
            "symbol" => {
                let text = node
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(self.ast.add_node(SExp::Symbol(text.to_string())))
            }
            "list" => {
                let parent = self.ast.reserve();
                let mut items = Vec::new();
                let mut child = node.named_child(0);
                while let Some(n) = child {
                    items.push(self.node_to_sexp(n, source)?);
                    child = n.next_named_sibling();
                }
                self.ast.set(parent, SExp::List(items));
                Ok(parent)
            }
            "struct" => {
                let strukt = self.ast.reserve();
                let mut child = node.named_child(0);
                let mut children = Vec::new();
                children.push(self.ast.add_node(SExp::Symbol("struct".to_string())));
                while let Some(n) = child {
                    children.push(self.node_to_sexp(n, source)?);
                    child = n.next_named_sibling();
                }
                self.ast.set(strukt, SExp::List(children));
                Ok(strukt)
            }
            "array" => {
                let array = self.ast.reserve();
                let mut items = Vec::new();
                let mut child = node.named_child(0);
                items.push(self.ast.add_node(SExp::Symbol("list".to_string())));
                while let Some(n) = child {
                    items.push(self.node_to_sexp(n, source)?);
                    child = n.next_named_sibling();
                }
                self.ast.set(array, SExp::List(items));
                Ok(array)
            }
            "quote" => self.shortcut(node, source, "quote"),
            "quasiquote" => self.shortcut(node, source, "quasiquote"),
            "unquote" => self.shortcut(node, source, "unquote"),

            kind => Err(ParseError::UnexpectedNode(format!(
                "Unexpected node kind: {}",
                kind
            ))),
        }
    }

    fn shortcut(
        &mut self,
        node: tree_sitter::Node,
        source: &str,
        symbol: &str,
    ) -> Result<SExpId, ParseError> {
        let parent = self.ast.reserve();
        let mut items = Vec::new();
        items.push(self.ast.add_node(SExp::Symbol(symbol.to_string())));

        let inner = node
            .child_by_field_name("inner")
            .ok_or_else(|| ParseError::TreeSitterError("No inner node".to_string()))?;

        let inner = self.node_to_sexp(inner, source)?;
        items.push(inner);
        self.ast.set(parent, SExp::List(items));
        Ok(parent)
    }

    pub fn parse(mut self, input: &str) -> Result<AST, ParseError> {
        let tree = self
            .parser
            .parse(input, None)
            .ok_or_else(|| ParseError::TreeSitterError("Failed to parse input".to_string()))?;

        let root = tree.root_node();
        if root.kind() != "source_file" {
            return Err(ParseError::UnexpectedNode(format!(
                "Expected source_file, got {}",
                root.kind()
            )));
        }

        // Get the first child of source_file
        let mut cursor = root.walk();
        if !cursor.goto_first_child() {
            return Err(ParseError::UnexpectedNode("Empty source file".to_string()));
        }

        let do_list = self.ast.reserve();
        let mut ids = Vec::new();
        loop {
            let node = cursor.node();
            let id = self.node_to_sexp(node, input)?;
            ids.push(id);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        let do_symbol = self.ast.add_node(SExp::Symbol("do".to_string()));
        ids.insert(0, do_symbol);
        self.ast.set(do_list, SExp::List(ids));
        Ok(self.ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "cst", |input, _deps| {
            let mut asts = ASTS::new();
            let ast = asts.parse(input).expect("Failed to parse");
            let root_id = ast.root_id().unwrap();
            let result = asts.get(root_id);
            let result = result.fmt(&asts);
            format!("{:?}", result)
        })
    }
}
