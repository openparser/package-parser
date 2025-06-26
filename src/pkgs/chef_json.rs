use packageurl::PackageUrl;
use serde_json::from_reader;
use serde_json::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package, PackageManifest};

use std::fs::File;
use std::path::Path;

pub struct ChefJson {}

impl ChefJson {
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
        let license = match &root["license"] {
            Value::String(license) => license.into(),
            _ => "".into(),
        };
        let requirements = match &root["dependencies"] {
            Value::Object(dependencies) => {
                let mut requirements = vec![];
                for (name, version) in dependencies {
                    let version = match version {
                        Value::String(name) => name.into(),
                        _ => "".into(),
                    };

                    // FIXME: fix the purl name
                    requirements.push(DependentPackage {
                        purl: PackageUrl::new("chef", name)
                            .expect("purl arguments are invalid")
                            .to_string(),
                        requirement: version,
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
            declared_license: license,
            dependencies: requirements,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for ChefJson {
    fn get_name(&self) -> String {
        "chef".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["metadata.json"]
    }
}
