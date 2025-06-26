use packageurl::PackageUrl;
use crate::types::{DependentPackage, Relation};
use serde::Deserialize;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::Package;

use std::collections::HashMap;
use std::path::Path;

use super::pyproject::PoetryTool;

#[derive(Debug, Deserialize)]
struct LockFile {
    #[serde(default)]
    package: Vec<LockPackage>,
}

#[derive(Debug, Deserialize)]
struct LockPackage {
    name: String,
    version: String,
    optional: bool,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    dependencies: HashMap<String, LockDependency>,
}

fn default_category() -> String {
    "main".into()
}

impl LockPackage {
    fn purl(&self) -> String {
        PackageUrl::new("pypi", &self.name)
            .unwrap()
            .with_version(&self.version)
            .to_string()
    }

    fn has_dependency(&self, name: &str) -> bool {
        self.dependencies.contains_key(name)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(unused)]
enum LockDependency {
    Compact(String),
    Expanded(ExpandedDependency),
    List(Vec<ExpandedDependency>),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ExpandedDependency {
    version: String,
    optional: Option<bool>,
    markers: Option<String>,
}

pub fn process(path: &Path, poetry: &PoetryTool) -> Result<Package, SourcePkgError> {
    let lock_content = std::fs::read_to_string(path)?;
    let lock_file: LockFile = toml::from_str(&lock_content)?;

    let mut deps = vec![];

    for package in lock_file.package.iter() {
        let mut dep = DependentPackage {
            purl: package.purl(),
            requirement: package.version.clone(),
            is_resolved: true,
            is_runtime: package.category != "dev",
            is_optional: package.optional,
            scope: package.category.clone(),
            ..Default::default()
        };

        if poetry.has_dependency(&package.name) {
            dep.relation.insert(Relation::Direct);
        }

        for package1 in lock_file.package.iter() {
            if package1.has_dependency(&package.name) {
                dep.relation.insert(Relation::Indirect);
                dep.parents.insert(package1.purl());
            }
        }

        deps.push(dep);
    }

    Ok(Package {
        dependencies: deps,
        ..Default::default()
    })
}
