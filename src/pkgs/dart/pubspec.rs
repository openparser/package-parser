use packageurl::PackageUrl;
use crate::types::DependentPackage;
use serde_yaml::from_reader;
use serde_yaml::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::fs::File;
use std::path::Path;

pub struct PubSpec {}

impl PubSpec {
    pub fn new() -> Self {
        Self {}
    }

    fn collect_dependencies(
        dep_root: &Value,
        scope: &'static str,
        is_optional: bool,
        is_runtime: bool,
    ) -> Vec<DependentPackage> {
        match dep_root {
            Value::Mapping(deps) => {
                let mut requirements = vec![];
                for (name, version) in deps {
                    let name: String = match name {
                        Value::String(s) => s.into(),
                        _ => "".into(),
                    };
                    let version: String = match version {
                        Value::String(s) => s.into(),
                        _ => "".into(),
                    };
                    if !name.is_empty() {
                        requirements.push(DependentPackage {
                            purl: PackageUrl::new("pub", name)
                                .expect("purl arguments are invalid")
                                .to_string(),
                            requirement: version,
                            scope: scope.into(),
                            is_optional,
                            is_runtime,
                            ..Default::default()
                        });
                    }
                }

                requirements
            }
            _ => vec![],
        }
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let path = path.as_ref();

        let lock_path = path.with_file_name("pubspec.lock");
        if lock_path.exists() {
            log::info!("Found pubspec.lock, parsing it instead of pubspec.yaml");

            match super::pubspec_lock::parse(lock_path) {
                Ok(p) => return Ok(p),
                Err(e) => {
                    log::warn!("Failed to parse pubspec.lock: {}", e);
                }
            }
        }

        let mut file = File::open(path)?;
        let root: Value = from_reader(&mut file)?;

        let name = match &root["name"] {
            Value::String(s) => s.into(),
            _ => "".into(),
        };
        let version = match &root["version"] {
            Value::String(s) => s.into(),
            _ => "".into(),
        };
        let declared_license = match &root["license"] {
            Value::String(s) => s.into(),
            _ => "".into(),
        };

        let mut requirements =
            Self::collect_dependencies(&root["dependencies"], "dependencies", false, true);

        let mut dev_requirements =
            Self::collect_dependencies(&root["dev_dependencies"], "dev_dependencies", true, false);

        let mut env_requirements =
            Self::collect_dependencies(&root["dev_dependencies"], "environment", false, true);

        requirements.append(&mut dev_requirements);
        requirements.append(&mut env_requirements);

        let package = Package {
            name,
            version,
            declared_license,
            dependencies: requirements,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PubSpec {
    fn get_name(&self) -> String {
        "pubspec".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["pubspec.yaml"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pubspec() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pubspec/specs/authors-pubspec.yaml"
        ));

        let p = PubSpec::parse(filepath).unwrap();
        println!("{:?}", p);
    }

    #[test]
    fn parse_pubspec_publish() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pubspec/specs/publish-pubspec.yaml"
        ));

        let p = PubSpec::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
