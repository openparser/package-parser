use packageurl::PackageUrl;
use crate::types::DependentPackage;
use toml::de::from_str;
use toml::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct Pipfile {}

impl Pipfile {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let root: Value = from_str(&content)?;
        let requirements = root
            .as_table()
            .and_then(|root_table| root_table.get("packages").and_then(|v| v.as_table()))
            .map(|requires| {
                let mut dependencies = vec![];
                for (name, version_requirement) in requires {
                    let version = match version_requirement {
                        Value::String(version) => version,
                        Value::Table(version_table) => version_table
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        _ => "",
                    };
                    let dependency = DependentPackage {
                        purl: PackageUrl::new("pypi", name)
                            .expect("purl arguments are invalid")
                            .to_string(),
                        requirement: version.to_string(),
                        ..Default::default()
                    };
                    dependencies.push(dependency);
                }
                dependencies
            });

        let dev_requirements = root
            .as_table()
            .and_then(|root_table| root_table.get("dev-packages").and_then(|v| v.as_table()))
            .map(|requires| {
                let mut dependencies = vec![];
                for (name, version_requirement) in requires {
                    let version = match version_requirement {
                        Value::String(version) => version,
                        Value::Table(version_table) => version_table
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        _ => "",
                    };
                    let dependency = DependentPackage {
                        purl: PackageUrl::new("pypi", name)
                            .expect("purl arguments are invalid")
                            .to_string(),
                        requirement: version.to_string(),
                        is_runtime: false,
                        is_optional: true,
                        ..Default::default()
                    };
                    dependencies.push(dependency);
                }
                dependencies
            });

        let mut requirements = requirements.unwrap_or_default();
        let mut dev_requirements = dev_requirements.unwrap_or_default();
        requirements.append(&mut dev_requirements);

        let package = Package {
            primary_language: "Python".into(),
            dependencies: requirements,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for Pipfile {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Pipfile::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["pipfile", "*pipfile"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_pipfile() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pypi/pipfile/Pipfile"
        ));

        let parser = Pipfile::new();
        let p = parser.recognize(filepath).await.unwrap();
        println!("{:?}", p);
    }
}
