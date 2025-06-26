use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use packageurl::PackageUrl;
use crate::types::{DependentPackage, Package};
use serde::Deserialize;

use crate::{error::SourcePkgError, PackageManifest};

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum ElmManifest {
    Application {
        #[serde(default)]
        dependencies: ApplicationDeps,
    },
    Package {
        #[serde(default)]
        dependencies: HashMap<String, String>,
    },
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ApplicationDeps {
    direct: HashMap<String, String>,
    indirect: HashMap<String, String>,
}

fn make_purl(name: &str, version: Option<&str>) -> Result<String, SourcePkgError> {
    let (ns, name) = name
        .split_once('/')
        .ok_or_else(|| SourcePkgError::GenericsError2(format!("Invalid package name: {name}")))?;

    let mut purl = PackageUrl::new("elm", name).unwrap();
    purl.with_namespace(ns);
    if let Some(v) = version {
        purl.with_version(v);
    }

    Ok(purl.to_string())
}

pub struct ElmJson {}

impl ElmJson {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: &Path) -> Result<Package, SourcePkgError> {
        let content = std::fs::read_to_string(path).map_err(SourcePkgError::Io)?;

        let parsed: ElmManifest =
            serde_json::from_str(&content).map_err(SourcePkgError::JsonParse)?;

        let mut ret = vec![];

        match parsed {
            ElmManifest::Application { dependencies } => {
                for (name, version) in dependencies.direct {
                    let purl = make_purl(&name, Some(&version))?;

                    ret.push(DependentPackage {
                        purl,
                        requirement: version,
                        scope: "prod".into(),
                        is_runtime: true,
                        is_optional: false,
                        is_resolved: true,
                        relation: maplit::hashset! { crate::types::Relation::Direct },
                        parents: HashSet::new(),
                        reachable: Default::default(),
                    });
                }

                for (name, version) in dependencies.indirect {
                    let purl = make_purl(&name, Some(&version))?;

                    ret.push(DependentPackage {
                        purl,
                        requirement: version,
                        scope: "prod".into(),
                        is_runtime: true,
                        is_optional: false,
                        is_resolved: true,
                        relation: maplit::hashset! { crate::types::Relation::Indirect },
                        parents: HashSet::new(),
                        reachable: Default::default(),
                    });
                }
            }
            ElmManifest::Package { dependencies } => {
                for (name, req) in dependencies {
                    let purl = make_purl(&name, None)?;

                    ret.push(DependentPackage {
                        purl,
                        requirement: req,
                        scope: "prod".into(),
                        is_runtime: true,
                        is_optional: false,
                        is_resolved: false,
                        relation: maplit::hashset! { crate::types::Relation::Direct },
                        parents: HashSet::new(),
                        reachable: Default::default(),
                    });
                }
            }
        }

        Ok(Package {
            dependencies: ret,
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl PackageManifest for ElmJson {
    fn get_name(&self) -> String {
        "elm".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["elm.json"]
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::ElmJson;

    #[test]
    fn package() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/elm/elm-package.json"
        ));

        dbg!(ElmJson::parse(&filepath).unwrap());
    }

    #[test]
    fn application() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/elm/elm-application.json"
        ));

        dbg!(ElmJson::parse(&filepath).unwrap());
    }
}