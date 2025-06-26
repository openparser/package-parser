use lib_ruby_parser::{Parser, ParserOptions, ParserResult};

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct Chef {}

impl Chef {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_send_node(
        statement: &lib_ruby_parser::Node,
        node_name: &'static str,
    ) -> Option<String> {
        match statement {
            lib_ruby_parser::Node::Send(send_node) => {
                // if send_node.name == "send" {
                //     if let lib_ruby_parser::Node::Send(send_node) = send_node.args[0] {
                //     }
                if send_node.method_name != node_name {
                    return None;
                }
                if send_node.args.is_empty() {
                    return None;
                }
                let first_node = &send_node.args[0];
                if let lib_ruby_parser::Node::Str(str_node) = first_node {
                    return Some(String::from_utf8(str_node.value.raw.clone()).unwrap_or_default());
                }

                None
            }
            _ => None,
        }
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut file = File::open(path)?;
        let mut content = vec![];
        file.read_to_end(&mut content)?;
        let options = ParserOptions {
            ..Default::default()
        };
        let parser = Parser::new(content, options);
        let ParserResult { ast, .. } = parser.do_parse();
        let mut package = Package::default();

        let ast = match ast {
            Some(ast) => ast,
            None => return Ok(package),
        };

        if let lib_ruby_parser::Node::Begin(begin) = *ast {
            for st in begin.statements {
                if let Some(name) = Self::parse_send_node(&st, "name") {
                    if package.name.is_empty() {
                        package.name = name;
                    }
                } else if let Some(version) = Self::parse_send_node(&st, "version") {
                    if package.version.is_empty() {
                        package.version = version;
                    }
                } else if let Some(license) = Self::parse_send_node(&st, "license") {
                    if package.declared_license.is_empty() {
                        package.declared_license = license;
                    }
                }
            }
        }

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for Chef {
    fn get_name(&self) -> String {
        "chef".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["metadata.rb"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chef_1() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/chef/dependencies/metadata.rb"
        ));

        let p = Chef::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
