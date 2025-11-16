package tree_sitter_patchwork_test

import (
	"testing"

	tree_sitter "github.com/smacker/go-tree-sitter"
	"github.com/tree-sitter/tree-sitter-patchwork"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_patchwork.Language())
	if language == nil {
		t.Errorf("Error loading Patchwork grammar")
	}
}
