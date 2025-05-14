package tree_sitter_s_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_s "github.com/tree-sitter/tree-sitter-s/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_s.Language())
	if language == nil {
		t.Errorf("Error loading S grammar")
	}
}
