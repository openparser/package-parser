use std::{collections::HashMap, fs::File, path::Path};

use packageurl::PackageUrl;
use crate::types::{DependentPackage, Package};
use serde::Deserialize;

use crate::{error::SourcePkgError, PackageManifest};

/// Dependency graph node for version 0.4.
#[derive(Debug, Deserialize)]
struct GraphNodeV04 {
    #[serde(rename = "ref")]
    ref_: Option<String>,
    path: Option<String>,
    #[serde(default)]
    requires: Vec<String>,
    prev: Option<String>,
}

/// Dependency graph for version 0.4.
#[derive(Debug, Deserialize)]
struct GraphLockV04 {
    nodes: HashMap<String, GraphNodeV04>,
}

/// Parse a dependency string for version 0.5.
///
/// `pkg/1.1#revision%timestamp`
fn parse_deps_v05(dep: &str) -> Option<DependentPackage> {
    let (name, dep) = dep.split_once('/')?;
    let (version, dep) = dep.split_once('#')?;
    let (hash, timestamp) = dep.split_once('%')?;

    let mut purl = PackageUrl::new("conan", name).unwrap();
    purl.with_version(version);
    purl.add_qualifier("rev", hash).ok();
    purl.add_qualifier("timestamp", timestamp).ok();

    Some(DependentPackage {
        purl: purl.to_string(),
        requirement: version.into(),
        is_resolved: true,
        ..Default::default()
    })
}

#[derive(Debug, Deserialize)]
#[serde(tag = "version")]
enum LockFile {
    #[serde(rename = "0.5")]
    V05 {
        #[serde(default)]
        requires: Vec<String>,
        #[serde(default)]
        build_requires: Vec<String>,
        // #[serde(default)]
        // python_requires: Vec<String>,
    },
    #[serde(rename = "0.4")]
    V04 { graph_lock: GraphLockV04 },
}

pub struct ConanLock {}

impl ConanLock {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: &Path) -> Result<Package, SourcePkgError> {
        let file = File::open(path)?;
        let lock_file: LockFile = serde_json::from_reader(file)?;

        let mut deps = vec![];

        match lock_file {
            LockFile::V05 {
                requires,
                build_requires,
                // python_requires: _,
            } => {
                log::info!("Got Conan v0.5 lock file.");

                for req in requires {
                    let dep = if let Some(dep) = parse_deps_v05(&req) {
                        dep
                    } else {
                        log::warn!("Failed to parse dependency: {}", req);
                        continue;
                    };
                    deps.push(dep);
                }

                for req in build_requires {
                    let mut dep = if let Some(dep) = parse_deps_v05(&req) {
                        dep
                    } else {
                        continue;
                    };
                    dep.scope = "build".to_string();
                    dep.is_runtime = false;
                    deps.push(dep);
                }
            }
            LockFile::V04 { graph_lock } => {
                log::info!("Got Conan v0.4 lock file.");

                let mut nodes = HashMap::new();

                for (id, node) in &graph_lock.nodes {
                    if let Some(path) = &node.path {
                        if path.starts_with("conanfile") {
                            // This is the root node.
                            continue;
                        }
                    }

                    let ref_ = if let Some(ref_) = &node.ref_ {
                        ref_
                    } else {
                        continue;
                    };

                    let (name, version) = if let Some((n, v)) = ref_.split_once('/') {
                        (n, v)
                    } else {
                        log::warn!("Failed to parse dependency: {}", ref_);
                        continue;
                    };

                    let mut purl = PackageUrl::new("conan", name).unwrap();
                    purl.with_version(version);
                    if let Some(prev) = &node.prev {
                        purl.add_qualifier("prev", prev).ok();
                    }

                    nodes.insert(
                        id.clone(),
                        DependentPackage {
                            purl: purl.to_string(),
                            requirement: version.into(),
                            is_resolved: true,
                            ..Default::default()
                        },
                    );
                }

                for (id, node) in &graph_lock.nodes {
                    if let Some(path) = &node.path {
                        if path.starts_with("conanfile") {
                            // This is the root node.

                            for req in &node.requires {
                                if let Some(dep_node) = nodes.get_mut(req) {
                                    dep_node.relation.insert(crate::types::Relation::Direct);
                                }
                            }

                            continue;
                        }
                    }

                    let current_purl = if let Some(current_node) = nodes.get(id) {
                        current_node.purl.clone()
                    } else {
                        continue;
                    };

                    for req in &node.requires {
                        if let Some(dep_node) = nodes.get_mut(req) {
                            dep_node.parents.insert(current_purl.clone());
                            dep_node
                                .relation
                                .insert(crate::types::Relation::Indirect);
                        }
                    }
                }

                deps.extend(nodes.into_values());
            }
        }

        Ok(Package {
            dependencies: deps,
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl PackageManifest for ConanLock {
    fn get_name(&self) -> String {
        "conan".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["conan.lock"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v05() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/conan/v05.conan.lock"
        ));

        let package = ConanLock::parse(filepath).unwrap();
        println!("{:?}", package);
    }

    #[test]
    fn v04() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/conan/v04.conan.lock"
        ));

        let package = ConanLock::parse(filepath).unwrap();
        println!("{:?}", package);
    }
}
