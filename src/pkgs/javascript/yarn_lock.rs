use crate::types::DependentPackage;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use super::make_purl;

pub struct YarnLock {}

impl YarnLock {
    pub fn new() -> Self {
        Self {}
    }
}

fn version_is_local(version: &str) -> bool {
    version == "0.0.0-use.local"
}

#[async_trait::async_trait]
impl PackageManifest for YarnLock {
    fn get_name(&self) -> String {
        "npm".to_string()
    }

    fn get_identifier(&self) -> String {
        "yarn-lock".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let yarn_lock_text = tokio::fs::read_to_string(path).await?;
        let entries = yarn_lock_parser::parse_str(&yarn_lock_text)?;

        let mut packages: HashMap<String, DependentPackage> = HashMap::new();

        let mut descriptors = HashMap::new();
        for entry in entries {
            let entry = Rc::new(entry);
            for desc in &entry.descriptors {
                descriptors.insert(*desc, entry.clone());
            }
        }

        for entry in descriptors.values() {
            let parent_purl = make_purl(entry.name, entry.version);

            let parent_is_local = version_is_local(entry.version);

            // Local deps should not be included in the graph.
            if !packages.contains_key(&parent_purl) && !parent_is_local {
                // Insert parent package
                packages.insert(
                    parent_purl.clone(),
                    DependentPackage {
                        purl: parent_purl.clone(),
                        requirement: entry.version.to_string(),
                        scope: "prod".into(),
                        is_resolved: true,
                        ..Default::default()
                    },
                );
            }

            for (name, version) in &entry.dependencies {
                // Insert dependencies
                let dep_entry = if let Some(x) = descriptors.get(&(name, version)) {
                    x.clone()
                } else {
                    continue;
                };

                if version_is_local(dep_entry.version) {
                    // Ignore local dependency
                    continue;
                }

                let dep_purl = make_purl(dep_entry.name, dep_entry.version);

                let pkg = packages
                    .entry(dep_purl.clone())
                    .or_insert_with(|| DependentPackage {
                        purl: dep_purl,
                        requirement: dep_entry.version.to_string(),
                        scope: "prod".into(),
                        is_resolved: true,
                        ..Default::default()
                    });

                if parent_is_local {
                    // Count as direct dependency
                    pkg.relation.insert(crate::types::Relation::Direct);
                } else {
                    // Count as indirect dependency
                    pkg.parents.insert(parent_purl.clone());
                    pkg.relation.insert(crate::types::Relation::Indirect);
                }
            }
        }

        // For packages that does not have parents, mark them as direct dependencies.
        for pkg in packages.values_mut() {
            if pkg.parents.is_empty() {
                pkg.relation.insert(crate::types::Relation::Direct);
            }
        }

        let package = Package {
            dependencies: packages.into_values().collect(),
            ..Default::default()
        };

        Ok(package)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["yarn.lock"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_yarnlock_1() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/yarn/yarn-v1.lock"
        ));

        let parser = YarnLock::new();
        let p = parser.recognize(filepath).await.unwrap();
        println!("{:?}", p);
    }

    #[tokio::test]
    async fn test_parse_yarnlock_v2_1() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/yarn/yarn-v2.lock"
        ));

        let parser = YarnLock::new();
        let p = parser.recognize(filepath).await.unwrap();
        println!("{:?}", p);
    }
}
