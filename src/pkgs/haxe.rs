use packageurl::PackageUrl;
use serde_json::from_reader;
use serde_json::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use super::common::model::DependentPackage;

use std::fs::File;
use std::path::Path;

pub struct Haxe {}

impl Haxe {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut file = File::open(path)?;
        let root: Value = from_reader(&mut file)?;
        let name = match &root["name"] {
            Value::String(name) => name.into(),
            _ => "".into(),
        };
        let version = match &root["version"] {
            Value::String(version) => version.into(),
            _ => "".into(),
        };
        let requirements = match &root["dependencies"] {
            Value::Object(deps) => {
                let mut requirements = vec![];
                for (name, version) in deps {
                    let version: String = match version {
                        Value::String(s) => s.into(),
                        _ => "".into(),
                    };

                    requirements.push(DependentPackage {
                        purl: PackageUrl::new("haxe", name)
                            .expect("purl arguments are invalid")
                            .to_string(),
                        requirement: version.clone(),
                        is_resolved: !version.is_empty(),
                        ..Default::default()
                    });
                }

                requirements
            }
            _ => vec![],
        };

        let package = Package {
            name,
            version,
            dependencies: requirements,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for Haxe {
    fn get_name(&self) -> String {
        "haxe".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["haxelib.json"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haxe_parse() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/haxe/deps/haxelib.json"
        ));

        let p = Haxe::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
