//! RML grammar for tree-sitter.
//!
//! Provides the compiled parser C ABI (`language()`) and highlight/injection
//! query strings used by gpui-component's `LanguageRegistry`.

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_rml() -> *const ();
}

/// Compiled RML parser entry point.
pub fn language() -> LanguageFn {
    unsafe { LanguageFn::from_raw(tree_sitter_rml) }
}

/// Tree-sitter highlight query (41 standard capture names).
///
/// Maps RML syntactic constructs to gpui-component's `HighlightTheme`
/// vocabulary: `tag`, `type`, `attribute`, `keyword`, `function`, `string`,
/// `variable`, `comment`, `punctuation.bracket`.
pub const HIGHLIGHTS_QUERY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/queries/highlights.scm"
));

/// Tree-sitter injection query.
///
/// Routes `{expr}` bindings/interpolations to the `rust` language for
/// embedded highlighting of field paths and expressions.
pub const INJECTIONS_QUERY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/queries/injections.scm"
));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_loads() {
        let language = tree_sitter::Language::new(language());
        assert!(language.node_kind_count() > 0);
    }

    #[test]
    fn parses_simple_element() {
        let language = tree_sitter::Language::new(language());
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();

        let source = "<div if={x}>{y}</div>";
        let tree = parser.parse(source, None).expect("parse failed");
        let root = tree.root_node();

        assert_eq!(root.kind(), "document");
        assert!(root.child_count() >= 1);
    }

    #[test]
    fn parses_self_closing_element() {
        let language = tree_sitter::Language::new(language());
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();

        let source = r#"<input label="Click" />"#;
        let tree = parser.parse(source, None).expect("parse failed");
        let root = tree.root_node();
        assert!(root.to_sexp().contains("self_closing_element"));
    }

    #[test]
    fn parses_comment() {
        let language = tree_sitter::Language::new(language());
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();

        let source = "<!-- comment --><div></div>";
        let tree = parser.parse(source, None).expect("parse failed");
        let root = tree.root_node();
        assert!(root.to_sexp().contains("comment"));
    }

    #[test]
    fn parses_each_directive() {
        let language = tree_sitter::Language::new(language());
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).unwrap();

        let source = r#"<li each={item in items}>{item.name}</li>"#;
        let tree = parser.parse(source, None).expect("parse failed");
        let root = tree.root_node();
        let sexp = root.to_sexp();
        assert!(sexp.contains("attribute_name"), "sexp: {sexp}");
        assert!(sexp.contains("binding"), "sexp: {sexp}");
        assert!(sexp.contains("interpolation"), "sexp: {sexp}");
    }
}
