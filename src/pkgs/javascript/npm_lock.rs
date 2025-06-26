use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::types::{DependentPackage, Package};
use serde::Deserialize;
use serde_json::Value;

use crate::{error::SourcePkgError, PackageManifest};

/// Splits a path that looks like `node_modules/@babel/helper-define-polyfill-provider/node_modules/semver/node_modules/@a/b`
/// into `["@babel/helper-define-polyfill-provider", "semver", "@a/b"]`
fn split_path(p: &str) -> Vec<&str> {
    let mut ret = vec![];
    for part in p.split("node_modules/") {
        if part.is_empty() {
            continue;
        }

        let part = if let Some(part) = part.strip_suffix('/') {
            // @babel/helper-define-polyfill-provider/
            part
        } else {
            // @babel/helper-define-polyfill-provider, last
            part
        };

        ret.push(part);
    }
    ret
}

#[derive(Debug, Deserialize)]
struct PackageLock {
    /// Note that v1 locks do not contain the main package.
    #[serde(default)]
    dependencies: HashMap<String, NpmPackage>,
    #[serde(default)]
    packages: HashMap<String, NpmV2Package>,
}

impl PackageLock {
    fn process_v2_or_later(&self) -> Option<Vec<DependentPackage>> {
        if self.packages.is_empty() {
            return None;
        }

        log::info!("Processing v2+ lockfile");

        // We have a v2+ lockfile
        let mut path_to_parents: HashMap<String, HashSet<String>> = HashMap::new();

        // Pass 1, collect all parents
        for (name, pkg) in &self.packages {
            let pkg_purl = if name.is_empty() {
                String::new()
            } else {
                let pkg_name = {
                    let parts = split_path(name);
                    if let Some(last) = parts.last() {
                        *last
                    } else {
                        log::warn!("Could not find valid package name in {}", name);
                        continue;
                    }
                };
                let version = if let Some(v) = pkg.version.as_ref() {
                    v
                } else {
                    log::warn!("Package {} has no version", name);
                    continue;
                };
                super::make_purl(pkg_name, version)
            };

            for dep_name in pkg
                .dependencies
                .keys()
                .chain(pkg.dev_dependencies.keys())
                .chain(pkg.optional_dependencies.keys())
            {
                let mut path = name.split('/').collect::<Vec<_>>();
                loop {
                    let key = if path.is_empty() {
                        format!("node_modules/{}", dep_name)
                    } else {
                        format!("{}/node_modules/{}", path.join("/"), dep_name)
                    };

                    if self.packages.contains_key(&key) {
                        if let Some(parents) = path_to_parents.get_mut(&key) {
                            parents.insert(pkg_purl.clone());
                        } else {
                            let mut h = HashSet::new();
                            h.insert(pkg_purl.clone());
                            path_to_parents.insert(key, h);
                        }

                        break;
                    }

                    if path.is_empty() {
                        log::warn!("Could not find dependency {} in lockfile", dep_name);
                        break;
                    }
                    path.pop();
                }
            }
        }

        // Pass 2, collect all packages
        let mut ret2 = HashMap::new();
        for (name, pkg) in &self.packages {
            if name.is_empty() {
                continue; // No need to collect the main package
            }

            let pkg_name = {
                let parts = split_path(name);
                if let Some(last) = parts.last() {
                    *last
                } else {
                    log::warn!("Could not find valid package name in {}", name);
                    continue;
                }
            };
            let version = if let Some(v) = pkg.version.as_ref() {
                v
            } else {
                log::warn!("Package {} has no version", name);
                continue;
            };

            let pkg_purl = super::make_purl(pkg_name, version);
            let mut parents = if let Some(parents) = path_to_parents.remove(name) {
                parents
            } else {
                log::warn!("{} was never referenced in the lock file", name);
                HashSet::new()
            };

            let dp = ret2
                .entry(pkg_purl.clone())
                .or_insert_with(|| DependentPackage {
                    purl: pkg_purl,
                    requirement: version.clone(),
                    scope: (if pkg.dev { "dev" } else { "prod" }).into(),
                    is_runtime: !pkg.dev,
                    is_optional: pkg.optional,
                    is_resolved: true,
                    ..Default::default()
                });

            if !pkg.dev {
                // If any of the definitions is prod, this dependency is prod.
                dp.scope = "prod".into();
            }
            if !pkg.optional {
                // Same.
                dp.is_optional = false;
            }

            if parents.remove("") {
                // This package is a direct dependency
                dp.relation.insert(crate::types::Relation::Direct);
            }
            if !parents.is_empty() {
                // There are still parents left, so this is an indirect dependency
                dp.relation.insert(crate::types::Relation::Indirect);
            }
            dp.parents.extend(parents);
        }

        Some(ret2.into_values().collect())
    }
    /// Convert v1 lockfile to a flattened v2 lockfile.
    fn flatten(self) -> HashMap<Vec<String>, (DependentPackage, Vec<String>)> {
        let mut ret = HashMap::new();

        fn process_package(
            mut parents: Vec<String>,
            name: String,
            mut package: NpmPackage,
            ret: &mut HashMap<Vec<String>, (DependentPackage, Vec<String>)>,
        ) {
            parents.push(name.clone());

            for (name, package) in package.dependencies.drain() {
                process_package(parents.clone(), name, package, ret);
            }

            let version = if let Some(v) = package.version {
                v
            } else {
                log::warn!("Package {} has no version", name);
                return;
            };

            let purl = super::make_purl(&name, &version);

            ret.insert(
                parents,
                (
                    DependentPackage {
                        purl,
                        requirement: version,
                        scope: (if package.dev { "dev" } else { "prod" }).into(),
                        is_runtime: !package.dev,
                        is_optional: package.optional,
                        is_resolved: true,
                        relation: HashSet::new(),
                        parents: HashSet::new(),
                        reachable: Default::default(),
                    },
                    package.requires.into_keys().collect(),
                ),
            );
        }

        for (name, package) in self.dependencies {
            process_package(vec![], name, package, &mut ret);
        }

        ret
    }
}

#[derive(Debug, Deserialize)]
struct NpmPackage {
    /// a specifier that varies depending on the nature of the package, and is usable in fetching a new copy of it.
    version: Option<String>,
    /// If true then this dependency is either an optional dependency ONLY of the top level module or a transitive dependency of one.
    #[serde(default)]
    optional: bool,
    /// If true then this dependency is either a development dependency ONLY of the
    /// top level module or a transitive dependency of one.
    /// This is false for dependencies that are both a development dependency of the
    /// top level and a transitive dependency of a non-development dependency of the top level.
    #[serde(default)]
    dev: bool,
    /// This is a list of everything this module requires, regardless of where it will be installed.
    /// The version should match via normal matching rules a dependency either in our dependencies
    /// or in a level higher than us.
    #[serde(default)]
    requires: HashMap<String, String>,
    /// The dependencies of this dependency, exactly as at the top level.
    #[serde(default)]
    dependencies: HashMap<String, NpmPackage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NpmV2Package {
    version: Option<String>,
    /// If the package is strictly part of the devDependencies tree, then `dev` will be true.
    #[serde(default)]
    dev: bool,
    /// If it is strictly part of the optionalDependencies tree, then optional will be set.
    #[serde(default)]
    optional: bool,
    // /// If it is both a dev dependency and an optional dependency of a non-dev dependency, then devOptional will be set.
    // #[serde(default)]
    // dev_optional: bool,
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default)]
    dev_dependencies: HashMap<String, String>,
    #[serde(default)]
    optional_dependencies: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NpmManifest {
    #[serde(default)]
    dependencies: HashMap<String, Value>,
    #[serde(default)]
    dev_dependencies: HashMap<String, Value>,
    #[serde(default)]
    optional_dependencies: HashMap<String, Value>,
}

impl NpmManifest {
    fn depends_on(&self, name: &str) -> bool {
        self.dependencies.contains_key(name)
            || self.dev_dependencies.contains_key(name)
            || self.optional_dependencies.contains_key(name)
    }
}

pub struct NpmLock {}

impl NpmLock {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn recognize_reader<M: std::io::Read, L: std::io::Read>(
        manifest: Option<M>,
        lock_file: L,
    ) -> Result<Package, SourcePkgError> {
        let lock: PackageLock = serde_json::from_reader(lock_file)?;

        if let Some(r) = lock.process_v2_or_later() {
            return Ok(Package {
                dependencies: r,
                ..Default::default()
            });
        };

        let manifest: NpmManifest = if let Some(r) = manifest {
            serde_json::from_reader(r)?
        } else {
            NpmManifest::default()
        };

        let flat_lock = lock.flatten();

        let mut packages = HashMap::new();

        for (path, (package, deps)) in &flat_lock {
            let p = packages
                .entry(package.purl.clone())
                .or_insert_with(|| package.clone());

            // Specify direct dependencies, this should always reach all packages
            if path.len() == 1 && manifest.depends_on(&path[0]) {
                p.relation.insert(crate::types::Relation::Direct);
            }

            // Apply dependencies
            for dep in deps {
                // Current path
                let mut path = path.clone();
                // Start with +1 level
                path.push(dep.clone());

                // Find the item in the original lock file
                let dep_item = loop {
                    if path.is_empty() {
                        break None;
                    }

                    // Replace the last part of the path with the dependency
                    path.last_mut().unwrap().clone_from(dep);

                    if let Some((item, _)) = flat_lock.get(&path) {
                        break Some(item);
                    }

                    path.pop();
                };

                // Skip if the dependency is not found
                let dep_item = if let Some(p) = dep_item {
                    p
                } else {
                    continue;
                };

                let dep_package = packages
                    .entry(dep_item.purl.clone())
                    .or_insert_with(|| dep_item.clone());

                // Add the current item as a parent
                dep_package.parents.insert(package.purl.clone());
                dep_package
                    .relation
                    .insert(crate::types::Relation::Indirect);
            }
        }

        Ok(Package {
            dependencies: packages.into_values().collect(),
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl PackageManifest for NpmLock {
    fn get_name(&self) -> String {
        "npm".into()
    }

    fn get_identifier(&self) -> String {
        "npm-lock".into()
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["package-lock.json"]
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let lock_reader = std::fs::File::open(path)?;

        let path_dir = path.parent().unwrap();

        let manifest_path = path_dir.join("package.json");
        let manifest_reader = if manifest_path.exists() {
            Some(std::fs::File::open(&manifest_path)?)
        } else {
            None
        };

        Self::recognize_reader(manifest_reader, lock_reader).await
    }
}
