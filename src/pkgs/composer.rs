use packageurl::PackageUrl;
use crate::types::Relation;
use serde::Deserialize;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package, PackageManifest};

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ComposerJson {
    #[serde(default)]
    require: HashMap<String, String>,
    #[serde(default)]
    require_dev: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ComposerLockJson {
    #[serde(default)]
    packages: Vec<ComposerLockPackage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ComposerLockPackage {
    name: String,
    version: String,
    #[serde(default)]
    require: HashMap<String, String>,
}

fn make_dep_unresolved(name: &str, constraint: &str, is_runtime: bool) -> DependentPackage {
    let (namespace, name) = if let Some((n, ns)) = name.split_once('/') {
        (Some(ns), n)
    } else {
        (None, name)
    };

    let mut purl = PackageUrl::new("composer", name).unwrap();
    if let Some(namespace) = namespace {
        purl.with_namespace(namespace);
    }

    DependentPackage {
        purl: purl.to_string(),
        requirement: constraint.to_string(),
        scope: (if is_runtime { "runtime" } else { "dev" }).into(),
        is_runtime,
        is_resolved: false,
        relation: maplit::hashset! {Relation::Direct},
        ..Default::default()
    }
}

fn make_purl_versioned(name: &str, version: &str) -> String {
    let (namespace, name) = if let Some((ns, n)) = name.split_once('/') {
        (Some(ns), n)
    } else {
        (None, name)
    };

    let mut purl = PackageUrl::new("composer", name).unwrap();
    purl.with_version(version);
    if let Some(namespace) = namespace {
        purl.with_namespace(namespace);
    }

    purl.to_string()
}

pub struct PhpComposer {}

impl PhpComposer {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let path = path.as_ref();

        let manifest: ComposerJson = {
            let mut file = File::open(path)?;
            serde_json::from_reader(&mut file)?
        };

        let lock_path = path.with_file_name("composer.lock");
        if lock_path.exists() {
            match process_composer_lock(&manifest, lock_path) {
                Ok(package) => return Ok(package),
                Err(err) => {
                    log::error!("Failed to process composer.lock: {}", err);
                }
            }
        }

        let mut dependencies = vec![];

        for dep in manifest.require.into_iter() {
            dependencies.push(make_dep_unresolved(&dep.0, &dep.1, true));
        }

        for dep in manifest.require_dev.into_iter() {
            dependencies.push(make_dep_unresolved(&dep.0, &dep.1, false));
        }

        let package = Package {
            dependencies,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PhpComposer {
    fn get_name(&self) -> String {
        "composer".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["composer.json"]
    }
}

fn process_composer_lock(
    manifest: &ComposerJson,
    path: impl AsRef<Path>,
) -> Result<Package, SourcePkgError> {
    let lock: ComposerLockJson = {
        let mut file = File::open(path)?;
        serde_json::from_reader(&mut file)?
    };

    let mut dependencies = vec![];

    for pkg in &lock.packages {
        let mut dep = DependentPackage {
            purl: make_purl_versioned(&pkg.name, &pkg.version),
            requirement: pkg.version.clone(),
            is_optional: false,
            is_runtime: true,
            is_resolved: true,
            ..Default::default()
        };

        for parent_pkg in &lock.packages {
            if parent_pkg.require.contains_key(pkg.name.as_str()) {
                // `require-dev` is root-only, development deps for other deps are
                // not installed
                dep.parents
                    .insert(make_purl_versioned(&parent_pkg.name, &parent_pkg.version));
            }
        }
        if !dep.parents.is_empty() {
            dep.relation.insert(Relation::Indirect);
        }

        if manifest.require.contains_key(&pkg.name) {
            dep.scope = "runtime".into();
            dep.is_runtime = true;
            dep.relation.insert(Relation::Direct);
        }

        if manifest.require_dev.contains_key(&pkg.name) && dep.parents.is_empty() {
            // This package is not a dependency of any other package, but it is
            // a dev dependency of the root package
            dep.scope = "dev".into();
            dep.is_runtime = false;
            dep.relation.insert(Relation::Direct);
        }

        dependencies.push(dep);
    }

    let package = Package {
        dependencies,
        ..Default::default()
    };

    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_parse_composer() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/phpcomposer/AdminPanel/composer.json"
        ));

        let parser = PhpComposer::new();
        let p = parser.recognize(filepath).await.unwrap();
        println!("{:#?}", p);
    }
}
