use packageurl::PackageUrl;
use serde_json::from_reader as json_from_reader;
use serde_json::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package, PackageManifest};

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct Pipfilelock {}

impl Pipfilelock {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_pipfilelock(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let file = File::open(path)?;
        let mut rdr = BufReader::new(file);
        let root: Value = json_from_reader(&mut rdr)?;
        let root = match root.as_object() {
            Some(root) => root,
            None => {
                return Err(SourcePkgError::GenericsError(
                    "Pipfilelock root is not an object",
                ))
            }
        };

        let default_dependencies = root.get("default").and_then(|v| v.as_object());
        let dev_dependencies = root.get("develop").and_then(|v| v.as_object());

        let mut dependencies = vec![];
        if let Some(deps) = default_dependencies {
            for (name, spec) in deps {
                let version = spec
                    .as_object()
                    .and_then(|v| v.get("version").and_then(|x| x.as_str()))
                    .map(|x| {
                        if let Some(x) = x.strip_prefix("==") {
                            x
                        } else {
                            x
                        }
                    });

                let mut purl = PackageUrl::new("pypi", name).unwrap();
                if let Some(v) = version {
                    purl.with_version(v);
                }

                let dep = DependentPackage {
                    purl: purl.to_string(),
                    requirement: version.unwrap_or("").to_string(),
                    is_resolved: true,
                    ..Default::default()
                };
                dependencies.push(dep);
            }
        };

        if let Some(deps) = dev_dependencies {
            for (name, spec) in deps {
                let dep = DependentPackage {
                    purl: PackageUrl::new("pypi", name)
                        .expect("purl arguments are invalid")
                        .to_string(),
                    requirement: spec
                        .as_object()
                        .and_then(|v| v.get("version").and_then(|x| x.as_str()))
                        .unwrap_or("")
                        .to_string(),
                    is_optional: true,
                    is_runtime: false,
                    is_resolved: false,
                    ..Default::default()
                };
                dependencies.push(dep);
            }
        };

        let package = Package {
            primary_language: "Python".into(),
            dependencies,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for Pipfilelock {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse_pipfilelock(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["Pipfile.lock", "*Pipfile.lock"]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_pipfilelock() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pypi/pipfile.lock/sample1/Pipfile.lock"
        ));

        let parser = Pipfilelock::new();
        let p = parser.recognize(filepath).await.unwrap();
        println!("{:#?}", p);
    }
}
