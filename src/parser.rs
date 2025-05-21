use std::fmt;

use tree_sitter::Parser as TSParser;

#[derive(Debug, Clone)]
pub enum SExp {
    Number(f64),
    String(String),
    Symbol(String),
    List(Vec<SExp>),
}

impl fmt::Display for SExp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SExp::Number(n) => write!(f, "{}", n),
            SExp::String(s) => write!(f, "\"{}\"", s),
            SExp::Symbol(s) => write!(f, "{}", s),
            SExp::List(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
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
}

impl SExpParser {
    pub fn new() -> Result<Self, ParseError> {
        let mut parser = TSParser::new();
        parser
            .set_language(&tree_sitter_s::LANGUAGE.into())
            .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;

        Ok(SExpParser { parser })
    }

    #[allow(clippy::only_used_in_recursion)]
    fn node_to_sexp(&self, node: tree_sitter::Node, source: &str) -> Result<SExp, ParseError> {
        match dbg!(node.kind()) {
            "float" | "integer" => {
                let text = node
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                let value = text
                    .parse::<f64>()
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(SExp::Number(value))
            }
            "string" => {
                let inner = node
                    .child_by_field_name("inner")
                    .ok_or_else(|| ParseError::TreeSitterError("No inner node".to_string()))?;
                let text = inner
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(SExp::String(text.to_string()))
            }
            "symbol" => {
                let text = node
                    .utf8_text(source.as_bytes())
                    .map_err(|e| ParseError::TreeSitterError(e.to_string()))?;
                Ok(SExp::Symbol(text.to_string()))
            }
            "list" => {
                let mut items = Vec::new();
                let mut child = node.named_child(0);
                while let Some(n) = child {
                    items.push(self.node_to_sexp(n, source)?);
                    child = n.next_named_sibling();
                }
                Ok(SExp::List(items))
            }
            kind => Err(ParseError::UnexpectedNode(format!(
                "Unexpected node kind: {}",
                kind
            ))),
        }
    }

    pub fn parse(&mut self, input: &str) -> Result<SExp, ParseError> {
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

        self.node_to_sexp(cursor.node(), input)
    }
}

pub fn parse(input: &str) -> Result<SExp, ParseError> {
    let mut parser = SExpParser::new()?;
    parser.parse(input)
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
            let result = parse(input).expect("Failed to parse");
            format!("{:?}", result)
        })
    }

    #[test]
    fn test_parse_simple_symbol() -> Result<(), ParseError> {
        let result = parse("foo")?;
        assert!(matches!(result, SExp::Symbol(s) if s == "foo"));
        Ok(())
    }

    #[test]
    fn test_parse_numeric_symbol() -> Result<(), ParseError> {
        let result = parse("42")?;
        dbg!(&result);
        assert!(matches!(result, SExp::Number(s) if compare_f64(s, 42.0)));
        Ok(())
    }

    #[test]
    fn test_parse_operator_symbol() -> Result<(), ParseError> {
        let result = parse("->")?;
        assert!(matches!(result, SExp::Symbol(s) if s == "->"));
        Ok(())
    }

    #[test]
    fn test_parse_string() -> Result<(), ParseError> {
        let result = parse("\"foo\"")?;
        assert!(matches!(result, SExp::String(s) if s == "foo"));
        Ok(())
    }

    #[test]
    fn test_parse_empty_list() -> Result<(), ParseError> {
        let result = parse("()")?;
        assert!(matches!(result, SExp::List(list) if list.is_empty()));
        Ok(())
    }

    #[test]
    fn test_parse_list_with_symbols() -> Result<(), ParseError> {
        let result = parse("(-> foo bar 12 ==)")?;
        match result {
            SExp::List(items) => {
                assert_eq!(items.len(), 5);
                assert!(matches!(items[0], SExp::Symbol(ref s) if s == "->"));
                assert!(matches!(items[1], SExp::Symbol(ref s) if s == "foo"));
                assert!(matches!(items[2], SExp::Symbol(ref s) if s == "bar"));
                assert!(matches!(items[3], SExp::Number(s) if compare_f64(s, 12.0)));
                assert!(matches!(items[4], SExp::Symbol(ref s) if s == "=="));
            }
            _ => panic!("Expected a list with five symbols"),
        }
        Ok(())
    }
}
