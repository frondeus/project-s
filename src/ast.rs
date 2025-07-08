use std::{collections::HashMap, fmt, sync::Arc};

use tree_sitter::{Node, Parser as TSParser, Point, Tree};

use crate::source::{Source, SourceId, Span, Spanned};

#[derive(Debug, Default)]
pub struct ASTS {
    asts: HashMap<usize, AST>,
    generation: usize,
    source_to_ast: HashMap<SourceId, SExpId>,
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

    pub fn get(&self, id: SExpId) -> &Spanned<SExp> {
        let ast = self.get_ast(id);
        ast.get(id)
    }

    pub fn get_by_source(&self, source_id: SourceId) -> Option<SExpId> {
        self.source_to_ast.get(&source_id).copied()
    }

    pub fn maybe_get(&self, id: Option<SExpId>) -> Option<&Spanned<SExp>> {
        Some(self.get(id?))
    }

    pub fn new_ast_mut(&mut self) -> &mut AST {
        let ast = self.new_ast();
        let id = self.add_ast(ast);
        self.get_ast_by_generation(id)
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

    pub fn parse(&mut self, source_id: SourceId, source: &Source) -> Result<&AST, ParseError> {
        if self.source_to_ast.contains_key(&source_id) {
            return Ok(self.get_ast(self.source_to_ast[&source_id]));
        }
        let parser = SExpParser::new(self, source_id, source)?;
        let ast = parser.parse()?;
        let root = ast.root_id().unwrap();
        self.source_to_ast.insert(source_id, root);
        self.add_ast(ast);
        Ok(self.get_ast(root))
    }
}

type TSNodeID = usize;

#[derive(Debug)]
pub struct AST {
    generation: usize,
    nodes: Vec<Spanned<SExp>>,
    root: Option<SExpId>,
    ts_nodes: HashMap<TSNodeID, SExpId>,
    ts_tree: Option<Tree>,
}

impl AST {
    fn new(generation: usize) -> Self {
        Self {
            generation,
            nodes: Vec::new(),
            ts_nodes: HashMap::new(),
            root: None,
            ts_tree: None,
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

    pub fn set_root(&mut self, id: SExpId) {
        self.root = Some(id);
    }

    pub fn add_node(&mut self, node: SExp, span: Span, ts_node: Option<Node<'_>>) -> SExpId {
        let node = Spanned::new(node, span);
        let id = self.nodes.len();
        self.nodes.push(node);
        let id = self.new_id(id);
        if let Some(ts_node) = ts_node {
            self.ts_nodes.insert(ts_node.id(), id);
        }
        id
    }

    pub fn get_from_ts(&self, node: Node<'_>) -> Option<SExpId> {
        self.ts_nodes.get(&node.id()).copied()
    }

    pub fn get_by_point(&self, point: Point) -> Option<SExpId> {
        let tree = self.ts_tree.as_ref()?;
        let root = tree.root_node();
        let node = root.descendant_for_point_range(point, point)?;
        self.get_from_ts(node)
    }

    // pub fn reserve(&mut self) -> SExpId {
    //     self.add_node(SExp::Error, Span::default())
    // }

    pub fn set(&mut self, id: SExpId, node: SExp, span: Span) {
        let node = Spanned::new(node, span);
        assert_eq!(id.generation, self.generation);
        self.nodes[id.id] = node;
    }

    pub fn get(&self, id: SExpId) -> &Spanned<SExp> {
        assert_eq!(id.generation, self.generation);
        &self.nodes[id.id]
    }

    pub fn maybe_get(&self, id: Option<SExpId>) -> Option<&Spanned<SExp>> {
        id.map(|id| self.get(id))
    }

    pub fn nodes(&self) -> &[Spanned<SExp>] {
        &self.nodes
    }

    pub fn root(&self) -> Option<&Spanned<SExp>> {
        self.root.map(|id| self.get(id))
    }

    pub fn root_id(&self) -> Option<SExpId> {
        self.root
    }

    pub fn spanned_root_id(&self) -> Option<Spanned<SExpId>> {
        let span = self.root()?.span;
        let id = self.root_id()?;
        Some(Spanned::new(id, span))
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

impl Spanned<SExp> {
    pub fn fmt<'a>(&'a self, asts: &'a ASTS) -> SExpFmt<'a> {
        SExpFmt { asts, expr: self }
    }
}

impl SExp {
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
        let width = f.width().unwrap_or(0);
        write!(f, "(")?;
        for (i, item) in self.list.iter().enumerate() {
            if i > 0 {
                if f.alternate() {
                    writeln!(f)?;
                    write!(f, "{}", " ".repeat(width))?;
                } else {
                    write!(f, " ")?;
                }
            }
            let item = self.asts.get(*item).fmt(self.asts);
            if f.alternate() {
                // let width = width + 2;
                write!(f, "{item:#width$}")?;
            } else {
                write!(f, "{item}")?;
            }
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
        let width = f.width().unwrap_or(0);
        // write!(f, "{}", " ".repeat(width))?;

        match self.expr {
            SExp::Number(n) => write!(f, "{n}"),
            SExp::String(s) => write!(f, "\"{s}\""),
            SExp::Symbol(s) => write!(f, "{s}"),
            SExp::Keyword(s) => write!(f, ":{s}"),
            SExp::Bool(b) => write!(f, "{b}"),
            SExp::Error => write!(f, "<Error>"),
            SExp::List(items) => {
                if f.alternate() {
                    write!(
                        f,
                        "{:#width$}",
                        &SExpFmtList {
                            asts: self.asts,
                            list: items
                        },
                        width = width + 2
                    )
                } else {
                    write!(
                        f,
                        "{}",
                        &SExpFmtList {
                            asts: self.asts,
                            list: items
                        }
                    )
                }
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
    ast: AST,
    source_id: SourceId,
    source: Arc<str>,
}

impl SExpParser {
    pub fn parser() -> Result<TSParser, ParseError> {
        let mut parser = TSParser::new();
        parser
            .set_language(&tree_sitter_s::LANGUAGE.into())
            .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
        Ok(parser)
    }

    pub fn parse_tree(source: &str) -> Result<Tree, ParseError> {
        let mut parser = Self::parser()?;

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or_else(|| ParseError::TreeSitterError("Failed to parse input".to_string()))?;

        Ok(tree)
    }

    pub fn new(asts: &mut ASTS, source_id: SourceId, source: &Source) -> Result<Self, ParseError> {
        Ok(SExpParser {
            ast: asts.new_ast(),
            source_id,
            source: source.source.clone(),
        })
    }

    #[allow(clippy::only_used_in_recursion)]
    fn node_to_sexp(&mut self, node: tree_sitter::Node) -> Result<SExpId, ParseError> {
        let span = Span {
            range: node.range(),
            source_id: self.source_id,
        };
        match node.kind() {
            "float" | "integer" => {
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                let value = text
                    .parse::<f64>()
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(self.ast.add_node(SExp::Number(value), span, Some(node)))
            }
            "boolean" => {
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                let value = text == "true";
                Ok(self.ast.add_node(SExp::Bool(value), span, Some(node)))
            }
            "string" => {
                let inner = node
                    .child_by_field_name("inner")
                    .ok_or_else(|| ParseError::TreeSitterError("No inner node".to_string()))?;
                let text = inner
                    .utf8_text(self.source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(self
                    .ast
                    .add_node(SExp::String(text.to_string()), span, Some(node)))
            }
            "keyword" => {
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                let text = text.trim_start_matches(':');
                Ok(self
                    .ast
                    .add_node(SExp::Keyword(text.to_string()), span, Some(node)))
            }
            "symbol" => {
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(self
                    .ast
                    .add_node(SExp::Symbol(text.to_string()), span, Some(node)))
            }
            "list" => {
                let mut items = Vec::new();
                let mut child = node.named_child(0);
                while let Some(n) = child {
                    if !n.is_extra() {
                        items.push(self.node_to_sexp(n)?);
                    }
                    child = n.next_named_sibling();
                }
                Ok(self.ast.add_node(SExp::List(items), span, Some(node)))
            }
            "struct" => {
                let mut child = node.named_child(0);
                let mut children = Vec::new();
                children.push(self.ast.add_node(
                    SExp::Symbol("obj/struct".to_string()),
                    span,
                    Some(node),
                ));
                while let Some(n) = child {
                    if !n.is_extra() {
                        children.push(self.node_to_sexp(n)?);
                    }
                    child = n.next_named_sibling();
                }
                Ok(self.ast.add_node(SExp::List(children), span, Some(node)))
            }
            "array" => {
                let mut items = Vec::new();
                let mut child = node.named_child(0);
                items.push(
                    self.ast
                        .add_node(SExp::Symbol("list".to_string()), span, Some(node)),
                );
                while let Some(n) = child {
                    if !n.is_extra() {
                        items.push(self.node_to_sexp(n)?);
                    }
                    child = n.next_named_sibling();
                }
                Ok(self.ast.add_node(SExp::List(items), span, Some(node)))
            }
            "quote" => self.shortcut(span, node, "quote"),
            "quasiquote" => self.shortcut(span, node, "quasiquote"),
            "unquote" => self.shortcut(span, node, "unquote"),
            "splice" => self.shortcut(span, node, "splice"),

            kind => Err(ParseError::UnexpectedNode(format!(
                "Unexpected node kind: {kind}",
            ))),
        }
    }

    fn shortcut(
        &mut self,
        span: Span,
        node: tree_sitter::Node,
        symbol: &str,
    ) -> Result<SExpId, ParseError> {
        let mut items = Vec::new();
        items.push(
            self.ast
                .add_node(SExp::Symbol(symbol.to_string()), span, Some(node)),
        );

        let inner = node
            .child_by_field_name("inner")
            .ok_or_else(|| ParseError::TreeSitterError("No inner node".to_string()))?;

        let inner = self.node_to_sexp(inner)?;
        items.push(inner);
        Ok(self.ast.add_node(SExp::List(items), span, Some(node)))
    }

    fn parse_with_tree(mut self, tree: Tree) -> Result<AST, ParseError> {
        {
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

            let mut ids = Vec::new();
            loop {
                let node = cursor.node();
                if !node.is_extra() {
                    let id = self.node_to_sexp(node)?;
                    ids.push(id);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            let span = Span {
                range: root.range(),
                source_id: self.source_id,
            };
            let do_symbol = self
                .ast
                .add_node(SExp::Symbol("do".to_string()), span, None);
            ids.insert(0, do_symbol);
            let root = self.ast.add_node(SExp::List(ids), span, None);
            self.ast.root = Some(root);
        }
        self.ast.ts_tree = Some(tree);
        Ok(self.ast)
    }

    pub fn parse(self) -> Result<AST, ParseError> {
        let tree = Self::parse_tree(&self.source)?;
        self.parse_with_tree(tree)
    }
}

#[cfg(test)]
mod tests {
    use crate::source::Sources;

    use super::*;

    #[test]
    fn cst() -> test_runner::Result {
        test_runner::test_snapshots("docs/", &["s", ""], "cst", |input, _deps, _args| {
            let mut parser = TSParser::new();
            parser
                .set_language(&tree_sitter_s::LANGUAGE.into())
                .unwrap();

            let tree = parser.parse(input, None).unwrap();

            let mut output = String::new();
            let mut cursor = tree.root_node().walk();
            let mut indent = 0;
            // let mut cost = 100;
            loop {
                // cost -= 1;
                let node = cursor.node();
                output.push_str(&" ".repeat(indent));
                output.push_str(&format!(
                    "{} - {:?}\n",
                    node.kind(),
                    node.utf8_text(input.as_bytes()).unwrap()
                ));

                if !cursor.goto_first_child() {
                    if !cursor.goto_next_sibling() {
                        if !cursor.goto_parent() {
                            break;
                        } else {
                            indent -= 1;
                            if !cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                } else {
                    indent += 1;
                }
            }
            output
        })
    }

    #[test]
    fn ast() -> test_runner::Result {
        test_runner::test_snapshots("docs/", &["s", ""], "ast", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let (sources, source_id) = Sources::single("<input>", input);
            let ast = asts
                .parse(source_id, sources.get(source_id))
                .expect("Failed to parse");
            let root_id = ast.root_id().unwrap();
            let result = asts.get(root_id);
            let result = result.fmt(&asts);
            format!("{result}")
        })
    }
}
