use serde_yaml::from_str;
use serde_yaml::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use crate::pkgs::pyrequirements::PyRequirements;

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct PyConda {}

impl PyConda {
    pub fn new() -> Self {
        Self {}
    }

    fn convert_to_pure_yaml(content: &str) -> String {
        let mut newlines = vec![];
        for line in content.lines() {
            if line.starts_with('{') {
                continue;
            }
            newlines.push(line);
        }

        newlines.join("\n")
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let content = Self::convert_to_pure_yaml(&content);
        let root: Value = from_str(&content)?;
        let package_name = &root["package"]["name"].as_str();
        let package_license = &root["about"]["license"].as_str();
        let requirements_section = &root["requirements"]["run"].as_sequence().map(|seq| {
            let mut requirements = vec![];
            for s in seq {
                if s.is_string() {
                    requirements.push(s.as_str().unwrap());
                }
            }

            requirements.join("\n")
        });

        let requirements = match requirements_section {
            Some(requirements) => requirements.to_string(),
            None => "".to_string(),
        };

        let requirements =
            PyRequirements::parse_requirement_content(&requirements).unwrap_or_default();

        let package = Package {
            name: package_name.unwrap_or("").to_string(),
            declared_license: package_license.unwrap_or("").to_string(),
            dependencies: requirements,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PyConda {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["conda.yaml", "conda*.yaml"]
    }
}


