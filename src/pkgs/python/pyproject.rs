//! Parser for `pyproject.toml` files.

use std::{collections::HashMap, path::Path};

use crate::types::Package;
use serde::Deserialize;

use crate::{error::SourcePkgError, pkgs::python::poetrylock, PackageManifest};

lazy_static::lazy_static! {
    static ref NORMALIZE_NAME: regex::Regex = regex::Regex::new(r"[-_.]+").unwrap();
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ProjectSpec {
    pub tool: ProjectSpecTool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ProjectSpecTool {
    pub poetry: Option<PoetryTool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
pub struct PoetryTool {
    dependencies: HashMap<String, PoetryDependency>,
    /// Poetry pre-1.2.x style, understood by Poetry 1.0–1.2
    dev_dependencies: HashMap<String, PoetryDependency>,
    group: HashMap<String, PoetryGroup>,
}

impl PoetryTool {
    pub fn has_dependency(&self, name: &str) -> bool {
        if self.dependencies.contains_key(name) {
            return true;
        }

        if self.dev_dependencies.contains_key(name) {
            return true;
        }

        for group in self.group.values() {
            if group.dependencies.contains_key(name) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct PoetryGroup {
    dependencies: HashMap<String, PoetryDependency>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(unused)]
enum PoetryDependency {
    Compact(String),
    Expanded(PoetryExpandedDependency),
    List(Vec<PoetryExpandedDependency>),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PoetryExpandedDependency {
    version: Option<String>,
    markers: Option<String>,
}

pub struct PyProject {}

impl PyProject {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: &Path) -> Result<Package, SourcePkgError> {
        let path_dir = path.parent().unwrap();

        let manifest_content = std::fs::read_to_string(path)?;
        let manifest: ProjectSpec = toml::from_str(&manifest_content)?;

        if let Some(mut poetry) = manifest.tool.poetry {
            // Normalize package names
            let normalize_name =
                |name: &str| -> String { NORMALIZE_NAME.replace_all(name, "-").to_lowercase() };

            let deps = poetry
                .dependencies
                .drain()
                .map(|(name, dep)| (normalize_name(&name), dep))
                .collect();
            poetry.dependencies = deps;

            let dev_deps = poetry
                .dev_dependencies
                .drain()
                .map(|(name, dep)| (normalize_name(&name), dep))
                .collect();
            poetry.dev_dependencies = dev_deps;

            for group in poetry.group.values_mut() {
                let deps = group
                    .dependencies
                    .drain()
                    .map(|(name, dep)| (normalize_name(&name), dep))
                    .collect();
                group.dependencies = deps;
            }

            // Resolve with lock file
            let lock_path = path_dir.join("poetry.lock");
            if lock_path.exists() {
                return poetrylock::process(&lock_path, &poetry);
            }
        }

        Ok(Default::default())
    }
}

#[async_trait::async_trait]
impl PackageManifest for PyProject {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["pyproject.toml"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poetry_with_lock() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pypi/poetry/pyproject.toml"
        ));

        let p = PyProject::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
