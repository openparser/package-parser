use tree_sitter::{Parser, Query, QueryCursor};

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use crate::pkgs::pyrequirements::PyRequirements;

use std::path::Path;

static SETUP_QUERY_STR: &str = "
(
	call
    function: (identifier) @function
    arguments: (
    	argument_list
        (
        	keyword_argument
            name: (identifier) @kw-install-requires-name
            value: (_) @kw-value
        )
    )
    (#eq? @function \"setup\")
    (#eq? @kw-install-requires-name \"install_requires\")
)
";

lazy_static::lazy_static! {
    static ref SETUP_QUERY: Query = {
        Query::new(&tree_sitter_python::language(), SETUP_QUERY_STR).unwrap()
    };
    static ref STRING_QUERY: Query = {
        Query::new(&tree_sitter_python::language(), "(string (string_content) @str)").unwrap()
    };
}

fn parse_strings(node: tree_sitter::Node, content: &[u8]) -> Vec<String> {
    let mut query_cursor = QueryCursor::new();
    let mut strings = vec![];
    for m in query_cursor.matches(&STRING_QUERY, node, content) {
        let value_node = m.captures.first().unwrap().node;
        let value = value_node.utf8_text(content).unwrap();
        strings.push(value.to_string());
    }

    strings
}

pub struct PySetup {}

impl PySetup {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let content_raw = content.as_bytes();

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::language())
            .unwrap();
        let tree = parser.parse(content_raw, None).unwrap();
        let root_node = tree.root_node();

        let mut query_cursor = QueryCursor::new();
        for m in query_cursor.matches(&SETUP_QUERY, root_node, content_raw) {
            let value_node = m.captures.get(2).unwrap().node;

            let packages = match value_node.grammar_name() {
                "list" => parse_strings(value_node, content_raw),
                "identifier" => {
                    let var_name = value_node.utf8_text(content_raw).unwrap();
                    let query = Query::new(
                        &tree_sitter_python::language(),
                        &format!(
                            "
                        (
                            assignment
                            left: (identifier) @name
                            right: (list) @value
                            (#eq? @name \"{}\")
                        )
                        ",
                            var_name
                        ),
                    )
                    .unwrap();

                    let mut query_cursor = QueryCursor::new();
                    if let Some(m) = query_cursor.matches(&query, root_node, content_raw).next() {
                        let value_node = m.captures.get(1).unwrap().node;
                        parse_strings(value_node, content_raw)
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            if packages.is_empty() {
                continue;
            }

            let mut requirements = packages.join("\n");
            requirements.push('\n');
            let parsed = PyRequirements::parse_requirement_content(&requirements)?;

            dbg!(parsed);
        }

        let package = Package::default();
        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PySetup {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["setup.py", "*setup.py", "setup*.py"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pysetup_1() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pypi/setup.py/simple-setup.py"
        ));

        let p = PySetup::parse(filepath).unwrap();
        println!("{:?}", p);
    }

    #[test]
    fn test_parse_pysetup_2() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pypi/setup.py/pipdeptree_setup.py"
        ));

        let p = PySetup::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
