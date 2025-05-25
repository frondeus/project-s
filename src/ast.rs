use std::{
    collections::HashMap,
    fmt,
    sync::{
        LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use tree_sitter::Parser as TSParser;

static GENERATION: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

#[derive(Debug, Default)]
pub struct ASTS {
    asts: HashMap<usize, AST>,
}

impl ASTS {
    pub fn new(ast: AST) -> Self {
        let mut asts = Self::default();
        asts.add_ast(ast);
        asts
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

    pub fn add_ast(&mut self, ast: AST) {
        self.asts.insert(ast.generation(), ast);
    }
}

#[derive(Debug)]
pub struct AST {
    generation: usize,
    nodes: Vec<SExp>,
}

impl Default for AST {
    fn default() -> Self {
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
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

    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let parser = SExpParser::new()?;
        parser.parse(input)
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
    List(Vec<SExpId>),

    Error,
}

impl SExp {
    pub fn fmt<'a>(&'a self, asts: &'a ASTS) -> SExpFmt<'a> {
        SExpFmt { asts, expr: self }
    }

    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            SExp::Symbol(s) => Some(s),
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

struct SExpFmtList<'a> {
    list: &'a [SExpId],
    asts: &'a ASTS,
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
    pub fn new() -> Result<Self, ParseError> {
        let mut parser = TSParser::new();
        parser
            .set_language(&tree_sitter_s::LANGUAGE.into())
            .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;

        Ok(SExpParser {
            parser,
            ast: AST::default(),
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
                let mut items = Vec::new();
                items.push(self.ast.add_node(SExp::Symbol("struct".to_string())));

                let quote = self.ast.reserve();
                let mut quoted = Vec::new();
                quoted.push(self.ast.add_node(SExp::Symbol("quote".to_string())));

                let inner = self.ast.reserve();
                let mut children = Vec::new();

                let mut child = node.named_child(0);
                while let Some(n) = child {
                    children.push(self.node_to_sexp(n, source)?);
                    child = n.next_named_sibling();
                }

                self.ast.set(inner, SExp::List(children));
                quoted.push(inner);
                self.ast.set(quote, SExp::List(quoted));
                items.push(quote);
                self.ast.set(strukt, SExp::List(items));
                Ok(strukt)
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

        self.node_to_sexp(cursor.node(), input)?;
        Ok(self.ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare_f64(a: f64, b: f64) -> bool {
        let precision = 0.01;
        (a - b).abs() < precision
    }

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "cst", |input, _deps| {
            let ast = AST::parse(input).expect("Failed to parse");
            let root_id = ast.root_id().unwrap();
            let asts = ASTS::new(ast);
            let result = asts.get(root_id);
            let result = result.fmt(&asts);
            format!("{:?}", result)
        })
    }

    #[test]
    fn test_parse_simple_symbol() -> Result<(), ParseError> {
        let result = AST::parse("foo")?;
        let result = result.root().unwrap();
        assert!(matches!(result, SExp::Symbol(s) if s == "foo"));
        Ok(())
    }

    #[test]
    fn test_parse_numeric_symbol() -> Result<(), ParseError> {
        let result = AST::parse("42")?;
        let result = result.root().unwrap();
        dbg!(&result);
        assert!(matches!(result, SExp::Number(s) if compare_f64(*s, 42.0)));
        Ok(())
    }

    #[test]
    fn test_parse_operator_symbol() -> Result<(), ParseError> {
        let result = AST::parse("->")?;
        let result = result.root().unwrap();
        assert!(matches!(result, SExp::Symbol(s) if s == "->"));
        Ok(())
    }

    #[test]
    fn test_parse_string() -> Result<(), ParseError> {
        let result = AST::parse("\"foo\"")?;
        let result = result.root().unwrap();
        assert!(matches!(result, SExp::String(s) if s == "foo"));
        Ok(())
    }

    #[test]
    fn test_parse_empty_list() -> Result<(), ParseError> {
        let result = AST::parse("()")?;
        let result = result.root().unwrap();
        assert!(matches!(result, SExp::List(list) if list.is_empty()));
        Ok(())
    }

    #[test]
    fn test_parse_list_with_symbols() -> Result<(), ParseError> {
        let ast = AST::parse("(-> foo bar 12 ==)")?;
        let result = ast.root().unwrap();
        match result {
            SExp::List(items) => {
                assert_eq!(items.len(), 5);
                assert!(matches!(ast.get(items[0]), SExp::Symbol(s) if s == "->"));
                assert!(matches!(ast.get(items[1]), SExp::Symbol(s) if s == "foo"));
                assert!(matches!(ast.get(items[2]), SExp::Symbol(s) if s == "bar"));
                assert!(matches!(ast.get(items[3]), SExp::Number(s) if compare_f64(*s, 12.0)));
                assert!(matches!(ast.get(items[4]), SExp::Symbol(s) if s == "=="));
            }
            _ => panic!("Expected a list with five symbols"),
        }
        Ok(())
    }
}
