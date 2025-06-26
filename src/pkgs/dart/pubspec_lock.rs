use maplit::hashset;
use packageurl::PackageUrl;
use crate::types::{DependentPackage, Relation};
use serde::Deserialize;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::Package;

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct LockPackage {
    dependency: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct PubspecLock {
    #[serde(default)]
    packages: HashMap<String, LockPackage>,
}

pub fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
    let mut file = File::open(path)?;

    let lock: PubspecLock = serde_yaml::from_reader(&mut file)?;

    let mut dependencies = vec![];

    for (name, package) in lock.packages {
        dependencies.push(DependentPackage {
            purl: PackageUrl::new("pub", name)
                .unwrap()
                .with_version(&package.version)
                .to_string(),
            requirement: package.version,
            scope: package.dependency.strip_prefix("direct ").unwrap_or_default()
            .into(),
            is_runtime: package.dependency != "direct dev",
            is_optional: false,
            is_resolved: true,
            relation: if package.dependency.starts_with("direct") {
                hashset! {Relation::Direct}
            } else {
                hashset! {Relation::Indirect}
            },
            ..Default::default()
        });
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

    #[test]
    fn test_parse_pubspec_lock() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pubspec/locks/stock-pubspec.lock"
        ));

        let p = parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
