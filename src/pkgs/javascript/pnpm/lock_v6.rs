use std::collections::{HashMap, HashSet};

use crate::types::{DependentPackage, Relation};
use serde::Deserialize;

use crate::{error::SourcePkgError, pkgs::javascript::make_purl};

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct PnpmLockV6 {
    importers: HashMap<String, Importer>,
    dependencies: HashMap<String, ImporterDep>,
    dev_dependencies: HashMap<String, ImporterDep>,
    packages: HashMap<String, Package>,
}

impl PnpmLockV6 {
    pub fn process(&self) -> Result<Vec<DependentPackage>, SourcePkgError> {
        let mut ret = vec![];

        for (pkg_key, pkg) in &self.packages {
            let mut parents = HashSet::new();

            let (name, version) = pkg.parse_name_version(pkg_key, false)?;
            let (_, version_with_peer) = pkg.parse_name_version(pkg_key, true)?;

            for (k, p) in &self.packages {
                let mut is_dep = false;

                if p.depends_on(name, &version_with_peer) {
                    is_dep = true;
                }

                if p.depends_on(name, pkg_key) {
                    is_dep = true;
                }

                if is_dep {
                    parents.insert(p.to_purl(k)?);
                }
            }

            let mut dp = DependentPackage {
                purl: pkg.to_purl(pkg_key)?,
                requirement: version.to_string(),
                parents: parents.into_iter().map(|s| s.to_string()).collect(),
                scope: (if pkg.dev { "dev" } else { "prod" }).into(),
                is_runtime: !pkg.dev,
                is_optional: pkg.optional,
                is_resolved: true,
                ..Default::default()
            };

            if self.is_direct_dependency(name, &version_with_peer)
                || self.is_direct_dependency(name, pkg_key)
            {
                dp.relation.insert(Relation::Direct);
            }

            if !dp.parents.is_empty() {
                dp.relation.insert(Relation::Indirect);
            }

            ret.push(dp);
        }

        Ok(ret)
    }

    fn is_direct_dependency(&self, name: &str, version: &str) -> bool {
        for importer in self.importers.values() {
            if let Some(imp) = importer.dependencies.get(name) {
                if imp.version == version {
                    return true;
                }
            }

            if let Some(imp) = importer.dev_dependencies.get(name) {
                if imp.version == version {
                    return true;
                }
            }
        }

        if let Some(imp) = self.dependencies.get(name) {
            if imp.version == version {
                return true;
            }
        }

        if let Some(imp) = self.dev_dependencies.get(name) {
            if imp.version == version {
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

#[derive(Deserialize, Debug)]
pub struct ImporterDep {
    version: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    name: Option<String>,
    version: Option<String>,
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default)]
    optional_dependencies: HashMap<String, String>,
    #[serde(default)]
    dev: bool, // false
    #[serde(default)]
    optional: bool, // false
}

impl Package {
    fn parse_name_version<'a>(
        &'a self,
        key: &'a str,
        keep_peer: bool,
    ) -> Result<(&'a str, String), SourcePkgError> {
        let peer = if let Some((_, p)) = key.split_once('(') {
            format!("({}", p)
        } else {
            String::new()
        };

        if let (Some(name), Some(version)) = (&self.name, &self.version) {
            return Ok((
                name.as_str(),
                if keep_peer {
                    format!("{}{}", version, peer)
                } else {
                    version.clone()
                },
            ));
        }

        // /vitest@1.4.0(@vitest/ui@1.0.4)
        if let Some(mut spec) = key.strip_prefix('/') {
            // /vitest@1.4.0
            if let Some((a, _)) = spec.split_once('(') {
                spec = a;
            }

            if let Some((name, version)) = spec.rsplit_once('@') {
                return Ok((
                    name,
                    if keep_peer {
                        format!("{}{}", version, peer)
                    } else {
                        version.to_string()
                    },
                ));
            }
        }

        Err(SourcePkgError::GenericsError2(format!(
            "Failed to parse package key {}",
            key
        )))
    }

    fn to_purl(&self, key: &str) -> Result<String, SourcePkgError> {
        if let (Some(name), Some(version)) = (&self.name, &self.version) {
            return Ok(make_purl(name, version));
        }

        // /vitest@1.4.0(@vitest/ui@1.0.4)
        if let Some(mut spec) = key.strip_prefix('/') {
            // /vitest@1.4.0
            if let Some((a, _)) = spec.split_once('(') {
                spec = a;
            }

            if let Some((name, version)) = spec.rsplit_once('@') {
                return Ok(make_purl(name, version));
            }
        }

        Err(SourcePkgError::GenericsError2(format!(
            "Failed to parse package key {}",
            key
        )))
    }

    fn depends_on(&self, name: &str, version: &str) -> bool {
        if let Some(v) = self.dependencies.get(name) {
            return v == version;
        }

        if let Some(v) = self.optional_dependencies.get(name) {
            return v == version;
        }

        false
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, path::PathBuf};

    use super::*;

    #[test]
    fn v6_without_importer() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pnpm/v6/without-importer/pnpm-lock.yaml"
        ));

        let mut file = File::open(filepath).unwrap();

        let parsed: PnpmLockV6 = serde_yaml::from_reader(&mut file).unwrap();

        let p = parsed.process().unwrap();

        dbg!(p);
    }

    #[test]
    fn v6_with_mirror() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pnpm/v6/with-mirror/pnpm-lock.yaml"
        ));

        let mut file = File::open(filepath).unwrap();

        let parsed: PnpmLockV6 = serde_yaml::from_reader(&mut file).unwrap();

        let p = parsed.process().unwrap();

        dbg!(p);
    }
}
