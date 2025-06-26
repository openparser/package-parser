use std::collections::HashMap;

use packageurl::PackageUrl;
use crate::types::DependentPackage;
use serde::Deserialize;

use crate::error::SourcePkgError;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PnpmLockV9 {
    importers: HashMap<String, Importer>,
    snapshots: HashMap<String, SnapshotPackage>,
    packages: HashMap<String, LockPackage>,
}

#[derive(Deserialize, Default)]
struct LockPackage {
    version: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SnapshotPackage {
    dependencies: HashMap<String, String>,
    optional_dependencies: HashMap<String, String>,
    #[serde(default)]
    optional: bool, // false
}

impl SnapshotPackage {
    fn depends_on(&self, name: &str, version_with_peer: &str) -> bool {
        if let Some(version) = self.dependencies.get(name) {
            if version == version_with_peer {
                return true;
            }
        }

        if let Some(version) = self.optional_dependencies.get(name) {
            if version == version_with_peer {
                return true;
            }
        }

        false
    }
}

#[derive(Deserialize, Default, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Importer {
    dependencies: HashMap<String, ImporterDep>,
    dev_dependencies: HashMap<String, ImporterDep>,
}

impl Importer {
    fn depends_on(&self, name: &str, version_with_peer: &str) -> bool {
        if let Some(imp) = self.dependencies.get(name) {
            if imp.version == version_with_peer {
                return true;
            }
        }

        if let Some(imp) = self.dev_dependencies.get(name) {
            if imp.version == version_with_peer {
                return true;
            }
        }

        false
    }
}

#[derive(Deserialize, Debug)]
pub struct ImporterDep {
    version: String,
}

impl PnpmLockV9 {
    pub fn process(&self) -> Result<Vec<DependentPackage>, SourcePkgError> {
        let mut ret = vec![];

        for (snap_key, snap) in self.snapshots.iter() {
            let mut parents = vec![];

            let (_, version_with_peer) = split_key(snap_key, true).ok_or_else(|| {
                SourcePkgError::GenericsError2(format!("Failed to split key {}", snap_key))
            })?;

            let (name, mut version_without_peer) = split_key(snap_key, false).ok_or_else(|| {
                SourcePkgError::GenericsError2(format!("Failed to split key {}", snap_key))
            })?;

            for (dep_snap_key, dep_snap) in self.snapshots.iter() {
                if dep_snap.depends_on(&name, &version_with_peer) {
                    parents.push(dep_snap_key.as_str());
                }
            }

            let mut is_direct = false;

            for imp in self.importers.values() {
                if imp.depends_on(&name, &version_with_peer) {
                    is_direct = true;
                    break;
                }
            }

            if let Some(pkg) = self.packages.get(snap_key) {
                if let Some(v) = &pkg.version {
                    version_without_peer = v.to_string(); // override
                }
            }

            let mut pkg = DependentPackage {
                purl: self.key_to_purl(snap_key)?,
                requirement: version_without_peer.to_string(),
                scope: "prod".into(),
                is_runtime: true,
                is_optional: snap.optional,
                is_resolved: true,
                ..Default::default()
            };

            if !parents.is_empty() {
                pkg.relation.insert(crate::types::Relation::Indirect);
            }

            for parent in parents {
                pkg.parents.insert(self.key_to_purl(parent)?);
            }

            if is_direct {
                pkg.relation.insert(crate::types::Relation::Direct);
            }

            ret.push(pkg);
        }

        Ok(ret)
    }

    fn key_to_purl(&self, key: &str) -> Result<String, SourcePkgError> {
        let (name, version_without_peer) = split_key(key, false).ok_or_else(|| {
            SourcePkgError::GenericsError2(format!("Failed to split key {}", key))
        })?;

        let mut version = version_without_peer.to_string();

        if let Some(pkg) = self.packages.get(key) {
            if let Some(v) = &pkg.version {
                version = v.to_string(); // override
            }
        }

        let (name, namespace) = if let Some((ns, n)) = name.split_once('/') {
            (n, Some(ns))
        } else {
            (name.as_str(), None)
        };

        let mut purl = PackageUrl::new("npm", name).unwrap();
        purl.with_version(version);
        if let Some(ns) = namespace {
            purl.with_namespace(ns);
        }

        Ok(purl.to_string())
    }
}

fn split_key(mut key: &str, keep_peer: bool) -> Option<(String, String)> {
    if !keep_peer {
        key = if let Some((a, _)) = key.split_once('(') {
            a
        } else {
            key
        };
    }

    if key.starts_with('@') {
        let (ns, name) = key.split_once('/')?;
        let (name, version) = name.split_once('@')?;
        Some((format!("{}/{}", ns, name), version.to_string()))
    } else {
        let (name, version) = key.split_once('@')?;
        Some((name.to_string(), version.to_string()))
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, path::PathBuf};

    use super::*;

    #[test]
    fn v9_default() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pnpm/v9/default/pnpm-lock.yaml"
        ));

        let mut file = File::open(filepath).unwrap();

        let parsed: PnpmLockV9 = serde_yaml::from_reader(&mut file).unwrap();

        let p = parsed.process().unwrap();

        dbg!(p);
    }

    #[test]
    fn v9_with_git() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pnpm/v9/git-dep/pnpm-lock.yaml"
        ));

        let mut file = File::open(filepath).unwrap();

        let parsed: PnpmLockV9 = serde_yaml::from_reader(&mut file).unwrap();

        let p = parsed.process().unwrap();

        dbg!(p);
    }
}
