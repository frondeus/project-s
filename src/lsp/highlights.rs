use tower_lsp_server::lsp_types::{SemanticToken, SemanticTokenType, SemanticTokensLegend};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

pub fn lsp_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::STRING,
            SemanticTokenType::COMMENT,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::NUMBER,
            SemanticTokenType::MACRO,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::OPERATOR,
        ],
        token_modifiers: vec![],
    }
}

fn highlight_legend() -> Vec<String> {
    lsp_legend()
        .token_types
        .iter()
        .map(|t| t.as_str().to_owned())
        .collect()
}

pub fn highlights(text: &str) -> Vec<SemanticToken> {
    let language = tree_sitter_s::LANGUAGE;

    let mut s_config = HighlightConfiguration::new(
        language.into(),
        "slang",
        tree_sitter_s::HIGHLIGHTS_QUERY,
        "", // tree_sitter_s::INJECTIONS_QUERY,
        "", // tree_sitter_s::LOCALS_QUERY,
    )
    .unwrap();

    let legend = highlight_legend();

    s_config.configure(&legend);

    let mut highlighter = Highlighter::new();

    let highlights = highlighter
        .highlight(&s_config, text.as_bytes(), None, |_| None)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let mut tokens = Vec::new();

    let mut current_highlight = None;
    let mut previous_source = None;
    for highlight in highlights {
        match highlight {
            HighlightEvent::Source { start, end } => {
                if let Some(current_highlight) = current_highlight {
                    let (p_line, p_offset) = previous_source.unwrap_or((0, 0));
                    let (line, offset) = offset_to_line(text, start);
                    let delta_line = line - p_line;
                    let delta_start = if delta_line == 0 {
                        offset - p_offset
                    } else {
                        offset
                    };

                    tokens.push(SemanticToken {
                        delta_line: delta_line as u32,
                        delta_start: delta_start as u32,
                        length: (end - start) as u32,
                        token_type: current_highlight,
                        token_modifiers_bitset: 0,
                    });
                    previous_source = Some((line, offset));
                }
            }
            HighlightEvent::HighlightStart(highlight) => {
                current_highlight = Some(highlight.0 as u32);
            }
            HighlightEvent::HighlightEnd => {
                current_highlight = None;
            }
        }
    }

    tokens
}

fn offset_to_line(text: &str, offset: usize) -> (usize, usize) {
    let max_counter = text.len();
    let mut counter = 0;
    for (line, line_content) in text.lines().enumerate() {
        let line_len = line_content.len();
        if counter + line_len > offset {
            return (line, offset - counter);
        }
        counter += line_len;
        // Adding newline
        while counter < max_counter {
            let c = &text[counter..counter + 1];
            if c == "\n" || c == "\r" {
                counter += 1;
            } else {
                break;
            }
        }
    }
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_to_line_offset_test() {
        let text = "Hello\nWorld";

        let res = offset_to_line(text, 0);
        assert_eq!(res, (0, 0));

        let res = offset_to_line(text, 1);
        assert_eq!(res, (0, 1));

        let res = offset_to_line(text, 6);
        assert_eq!(res, (1, 0));

        let res = offset_to_line(text, 7);
        assert_eq!(res, (1, 1));
    }
}
