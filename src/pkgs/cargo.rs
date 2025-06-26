use cargo_lock::Lockfile as CargoLockfile;
use cargo_manifest::Dependency;
use cargo_manifest::Error as CargoManifestError;
use cargo_manifest::Manifest as CargoManifest;
use maplit::hashset;
use packageurl::PackageUrl;
use crate::types::Relation;
use std::{path::Path, str::FromStr};
use toml::value::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use super::common::model::DependentPackage;

pub struct CargoLock {}

impl CargoLock {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_corrupted_lockfile(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let location = path.as_ref();
        let content = std::fs::read_to_string(location)?;
        let root = Value::from_str(&content)?;
        let root = match root.as_table() {
            Some(root) => root,
            None => {
                return Err(SourcePkgError::GenericsError(
                    "corrupted lockfile : the root element must be table",
                ))
            }
        };

        let package = match root.get("package") {
            Some(package) => package,
            None => {
                return Err(SourcePkgError::GenericsError(
                    "corrupted lockfile : the root element must have a package element",
                ))
            }
        };

        let package = match package.as_array() {
            Some(package) => package,
            None => {
                return Err(SourcePkgError::GenericsError(
                    "corrupted lockfile : the package element must be array",
                ))
            }
        };

        let mut dependencies = Vec::new();
        for pkg in package {
            let pkg = match pkg.as_table() {
                Some(pkg) => pkg,
                None => continue,
            };

            let name = pkg.get("name").and_then(|name| name.as_str());

            let name = match name {
                Some(name) => name,
                None => continue,
            };

            let version = pkg.get("version").and_then(|version| version.as_str());

            let version = match version {
                Some(version) => version,
                None => continue,
            };

            let dep = DependentPackage {
                purl: PackageUrl::new("cargo", name)
                    .expect("purl arguments are invalid")
                    .to_string(),

                requirement: version.to_string(),
                ..Default::default()
            };

            dependencies.push(dep);
        }

        let manifest = Package {
            dependencies,
            ..Default::default()
        };

        Ok(manifest)
    }

    fn parse_lockfile(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let lock = CargoLockfile::load(path)?;
        let dependencies = lock
            .packages
            .into_iter()
            .map(|pkg| DependentPackage {
                purl: PackageUrl::new("cargo", pkg.name.as_str())
                    .expect("purl arguments are invalid")
                    .to_string(),
                requirement: pkg.version.to_string(),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let manifest = Package {
            dependencies,
            ..Default::default()
        };

        Ok(manifest)
    }
}

#[async_trait::async_trait]
impl PackageManifest for CargoLock {
    fn get_name(&self) -> String {
        "crates".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let manifest = match Self::parse_lockfile(path) {
            Ok(manifest) => manifest,
            Err(_) => Self::parse_corrupted_lockfile(path)?,
        };

        Ok(manifest)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["Cargo.lock"]
    }
}

pub struct CargoToml {}

impl CargoToml {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_cargo_toml(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let metadata = match CargoManifest::from_path(path) {
            Ok(metadata) => metadata,
            Err(err) => match err {
                CargoManifestError::Io(err) => return Err(SourcePkgError::Io(err)),
                CargoManifestError::Parse(err) => return Err(SourcePkgError::TomlDeserialize(err)),
                CargoManifestError::Utf8(_) => {
                    return Err(SourcePkgError::GenericsError("invalid utf8"))
                }
            },
        };

        let convert = |scope: &str, is_dev: bool| {
            let scope = scope.to_string();

            move |(name, dep): (String, Dependency)| DependentPackage {
                purl: PackageUrl::new("cargo", name)
                    .expect("purl arguments are invalid")
                    .to_string(),
                scope: scope.clone(),
                is_runtime: !is_dev,
                is_optional: match &dep {
                    cargo_manifest::Dependency::Simple(_) => false,
                    cargo_manifest::Dependency::Detailed(detail) => {
                        detail.optional.unwrap_or(false)
                    }
                },
                is_resolved: false,
                requirement: match dep {
                    cargo_manifest::Dependency::Simple(version) => version,
                    cargo_manifest::Dependency::Detailed(detail) => {
                        // arbitrary version for None case
                        detail.version.unwrap_or_else(|| "*".into())
                    }
                },
                parents: Default::default(),
                relation: hashset! {Relation::Direct},
                reachable: Default::default(),
            }
        };

        let dependencies = metadata
            .dependencies
            .unwrap_or_default()
            .into_iter()
            .map(convert("dependencies", false));

        let dev_dependencies = metadata
            .dev_dependencies
            .unwrap_or_default()
            .into_iter()
            .map(convert("dev-dependencies", true));

        let dependencies = dependencies.chain(dev_dependencies).collect();

        // TODO: add authors
        let package = Package {
            dependencies,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for CargoToml {
    fn get_name(&self) -> String {
        "crates".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse_cargo_toml(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["Cargo.toml"]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parser_cargo_toml() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/cargo/cargo_toml/clippy/Cargo.toml"
        ));
        CargoToml::parse_cargo_toml(filepath).unwrap();
        // println!("{:#?}", metadata);
    }

    #[test]
    fn test_parser_cargo_lock_1() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/cargo/cargo_lock/sample5/Cargo.lock"
        ));

        CargoLock::parse_corrupted_lockfile(filepath).unwrap();
    }

    #[test]
    fn test_parser_cargo_lock_2() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/cargo/cargo_lock/sample6/Cargo.lock"
        ));

        CargoLock::parse_lockfile(filepath).unwrap();
    }
}
