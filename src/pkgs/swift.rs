use std::{fs::File, path::Path};

use crate::types::{DependentPackage, Package};
use serde_json::Value;

use crate::{error::SourcePkgError, PackageManifest};

mod v1 {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct LockFile {
        pub object: Object,
    }

    #[derive(Debug, Deserialize)]
    pub struct Object {
        #[serde(default)]
        pub pins: Vec<PinnedPackage>,
    }

    #[derive(Debug, Deserialize)]
    pub struct PinnedPackage {
        pub package: String,
        #[serde(rename = "repositoryURL")]
        pub repository_url: Option<String>,
        pub state: PinnedPackageState,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PinnedPackageState {
        pub version: Option<String>,
        // pub branch: Option<String>,
        pub revision: Option<String>,
    }
}

mod v2 {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct LockFile {
        pub pins: Vec<PinnedPackage>,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    pub struct PinnedPackage {
        pub location: String,
        pub state: PinnedPackageState,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    pub struct PinnedPackageState {
        pub version: Option<String>,
        pub revision: Option<String>,
    }
}

fn repo_url_to_purl(url: &str) -> Option<String> {
    let url = if let Some(url) = url.split_once("://") {
        url.1
    } else {
        return None;
    };

    let url = url.strip_suffix(".git").unwrap_or(url);

    Some(format!("pkg:swift/{}", url))
}

pub struct SwiftPmLock;

impl SwiftPmLock {
    pub fn new() -> Self {
        Self
    }

    fn parse(path: &Path) -> Result<Package, SourcePkgError> {
        let file = File::open(path)?;
        let resolved: Value = serde_json::from_reader(file)?;

        let mut deps = vec![];

        let version = resolved.get("version").and_then(|v| v.as_i64());
        match version {
            Some(1) => {
                let resolved = serde_json::from_value::<v1::LockFile>(resolved)?;
                for pkg in resolved.object.pins {
                    let url = if let Some(url) = pkg.repository_url {
                        url
                    } else {
                        log::warn!("No URL for package {}", pkg.package);
                        continue;
                    };

                    let mut purl = if let Some(purl) = repo_url_to_purl(&url) {
                        purl
                    } else {
                        log::warn!("Could not convert URL {} to purl", url);
                        continue;
                    };

                    let version = pkg.state.version.or(pkg.state.revision);

                    if let Some(v) = &version {
                        purl.push_str(&format!("@{}", v));
                    }

                    let dep = DependentPackage {
                        purl,
                        requirement: version.unwrap_or_default(),
                        scope: "".into(),
                        is_runtime: true,
                        is_optional: false,
                        is_resolved: true,
                        ..Default::default()
                    };

                    deps.push(dep);
                }
            }
            Some(2) => {
                let resolved = serde_json::from_value::<v2::LockFile>(resolved)?;

                for pkg in resolved.pins {
                    let url = pkg.location;

                    let mut purl = if let Some(purl) = repo_url_to_purl(&url) {
                        purl
                    } else {
                        log::warn!("Could not convert URL {} to purl", url);
                        continue;
                    };

                    let version = pkg.state.version.or(pkg.state.revision);

                    if let Some(v) = &version {
                        purl.push_str(&format!("@{}", v));
                    }

                    let dep = DependentPackage {
                        purl,
                        requirement: version.unwrap_or_default(),
                        scope: "".into(),
                        is_runtime: true,
                        is_optional: false,
                        is_resolved: true,
                        ..Default::default()
                    };

                    deps.push(dep);
                }
            }
            v => {
                return Err(SourcePkgError::GenericsError2(format!(
                    "Unrecognized swift package manager lock file version: {:?}",
                    v
                )));
            }
        }

        Ok(Package {
            dependencies: deps,
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl PackageManifest for SwiftPmLock {
    fn get_name(&self) -> String {
        "swift".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["Package.resolved"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/swift/v1_erik.resolved"
        ));

        let p = SwiftPmLock::parse(filepath).unwrap();
        println!("{:?}", p);
    }

    #[test]
    fn v2() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/swift/v2_firefox.resolved"
        ));

        let p = SwiftPmLock::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
