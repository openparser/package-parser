use std::{collections::{HashMap, HashSet}, path::Path};

use packageurl::PackageUrl;
use crate::types::{DependentPackage, Package, Relation};
use serde::Deserialize;

use crate::{error::SourcePkgError, PackageManifest};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct LockFile {
    #[serde(default)]
    packages: HashMap<String, LockPackage>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct LockPackage {
    package: String,
    version: String,
    // source: String,
    #[serde(default)]
    requirements: Vec<String>,
}

impl LockPackage {
    fn to_purl(&self) -> String {
        let mut purl = PackageUrl::new("cran", &self.package).unwrap();
        purl.with_version(&self.version);

        purl.to_string()
    }

    fn depends_on(&self, package: &str) -> bool {
        self.requirements.iter().any(|x| x.as_str() == package)
    }
}

pub struct RenvLock {}

impl RenvLock {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: &Path) -> Result<Package, SourcePkgError> {
        let reader = std::fs::File::open(path).map_err(SourcePkgError::Io)?;

        let parsed: LockFile =
            serde_json::from_reader(reader).map_err(SourcePkgError::JsonParse)?;

        let mut deps = vec![];

        for pkg in parsed.packages.values() {
            let purl = pkg.to_purl();

            let mut parents = HashSet::new();

            for parent in parsed.packages.values() {
                if parent.depends_on(&pkg.package) {
                    parents.insert(parent.to_purl());
                }
            }

            let mut d = DependentPackage {
                purl,
                requirement: pkg.version.clone(),
                scope: "main".into(),
                is_resolved: true,
                is_runtime: true,
                is_optional: false,
                relation: maplit::hashset! { Relation::Direct },
                parents,
                reachable: Default::default(),
            };

            if !d.parents.is_empty() {
                d.relation.insert(Relation::Indirect);
            }

            deps.push(d);
        }

        Ok(Package {
            dependencies: deps,
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl PackageManifest for RenvLock {
    fn get_name(&self) -> String {
        "renv".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["renv.lock"]
    }
}


#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::RenvLock;

    #[test]
    fn v4_2_2() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/renv/422-1.lock"
        ));

        dbg!(RenvLock::parse(&filepath).unwrap());
    }

    #[test]
    fn v3_6_3() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/renv/363-1.lock"
        ));

        dbg!(RenvLock::parse(&filepath).unwrap());
    }
}