use std::collections::HashSet;

use packageurl::PackageUrl;
use tree_sitter::{Parser, Query};

use crate::{error::SourcePkgError, helper::query_matches};

static JS_IMPORT_QUERY: &str = "(import_statement source: (string (string_fragment) @src))";
static JS_REQUIRE_QUERY: &str = "
    (
        call_expression function: (identifier) @function-name arguments: (arguments (string (string_fragment) @require-name))
        (#eq? @function-name \"require\")
    )
";

fn new_import_query(lang: &tree_sitter::Language) -> Query {
    Query::new(lang, JS_IMPORT_QUERY).unwrap()
}

fn new_require_query(lang: &tree_sitter::Language) -> Query {
    Query::new(lang, JS_REQUIRE_QUERY).unwrap()
}

pub fn js_import_to_purl(src: &str) -> Option<String> {
    if src.starts_with('.') {
        return None;
    }

    let parts = src.split('/').collect::<Vec<_>>();
    let first_part = parts.first().unwrap();

    if first_part.starts_with('@') {
        // Scoped package
        if parts.len() < 2 {
            return None;
        }
        let second_part = parts.get(1).unwrap();
        let mut purl = PackageUrl::new("npm", second_part.to_string()).unwrap();
        purl.with_namespace(first_part.to_string());
        Some(purl.to_string())
    } else {
        // Unscoped package
        let purl = PackageUrl::new("npm", first_part.to_string()).unwrap();
        Some(purl.to_string())
    }
}

pub fn process_es(
    code: &str,
    language: &tree_sitter::Language,
    import_query: &Query,
    require_query: &Query,
) -> Result<HashSet<String>, SourcePkgError> {
    let code_raw = code.as_bytes();
    let mut imports = HashSet::new();

    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let root = tree.root_node();

    for captures in query_matches(root, import_query, code_raw) {
        for capture in captures {
            let text = capture.node.utf8_text(code_raw)?;
            if let Some(t) = js_import_to_purl(text) {
                imports.insert(t);
            }
        }
    }

    for captures in query_matches(root, require_query, code_raw) {
        let text_capture = captures.get(1).expect("Expected text capture");
        let text = text_capture.node.utf8_text(code_raw)?;
        if let Some(t) = js_import_to_purl(text) {
            imports.insert(t);
        }
    }

    Ok(imports)
}

pub fn process_js(code: &str) -> Result<HashSet<String>, SourcePkgError> {
    lazy_static::lazy_static! {
        static ref IMPORT_QUERY: Query = new_import_query(&tree_sitter_javascript::language());
        static ref REQUIRE_QUERY: Query = new_require_query(&tree_sitter_javascript::language());
    }

    process_es(
        code,
        &tree_sitter_javascript::language(),
        &IMPORT_QUERY,
        &REQUIRE_QUERY,
    )
}

pub fn process_ts(code: &str) -> Result<HashSet<String>, SourcePkgError> {
    lazy_static::lazy_static! {
        static ref IMPORT_QUERY: Query = new_import_query(&tree_sitter_typescript::language_typescript());
        static ref REQUIRE_QUERY: Query = new_require_query(&tree_sitter_typescript::language_typescript());
    }

    process_es(
        code,
        &tree_sitter_typescript::language_typescript(),
        &IMPORT_QUERY,
        &REQUIRE_QUERY,
    )
}

pub fn process_tsx(code: &str) -> Result<HashSet<String>, SourcePkgError> {
    lazy_static::lazy_static! {
        static ref IMPORT_QUERY: Query = new_import_query(&tree_sitter_typescript::language_tsx());
        static ref REQUIRE_QUERY: Query = new_require_query(&tree_sitter_typescript::language_tsx());
    }

    process_es(
        code,
        &tree_sitter_typescript::language_tsx(),
        &IMPORT_QUERY,
        &REQUIRE_QUERY,
    )
}
