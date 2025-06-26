use std::{collections::HashMap, path::Path};

use maplit::hashset;
use packageurl::PackageUrl;
use crate::types::{DependentPackage, Package};
use serde::Deserialize;

use crate::{error::SourcePkgError, PackageManifest};

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case", default)]
struct Manifest {
    dependencies: HashMap<String, Dependency>,
    dev_dependencies: HashMap<String, Dependency>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[allow(unused)]
enum Dependency {
    External {
        git: Option<String>,
        rev: Option<String>,
        tag: Option<String>,
    },
    Builtin(String),
}

impl Dependency {
    fn to_purl_and_version(&self, key: &str) -> (String, String) {
        let mut purl = PackageUrl::new("fpm", key).unwrap();

        let version = match self {
            Dependency::External { git, rev, tag } => {
                if let Some(git) = git {
                    purl.add_qualifier("vcs_url", git).unwrap();
                }
                if let Some(rev) = rev {
                    rev.clone()
                } else if let Some(tag) = tag {
                    tag.clone()
                } else {
                    "*".to_string()
                }
            }
            Dependency::Builtin(_) => "*".to_string(),
        };

        if version != "*" {
            purl.with_version(&version);
        }

        (purl.to_string(), version)
    }
}

pub struct FpmToml {}

impl FpmToml {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: &Path) -> Result<Package, SourcePkgError> {
        let content = std::fs::read_to_string(path).map_err(SourcePkgError::Io)?;

        let parsed: Manifest = toml::from_str(&content).map_err(SourcePkgError::TomlDeserialize)?;

        let mut deps = vec![];

        for (key, dep) in parsed.dependencies {
            let (purl, version) = dep.to_purl_and_version(&key);

            deps.push(DependentPackage {
                purl,
                is_resolved: version != "*",
                requirement: version,
                scope: "prod".into(),
                is_runtime: true,
                relation: hashset! { crate::types::Relation::Direct },
                ..Default::default()
            });
        }

        for (key, dep) in parsed.dev_dependencies {
            let (purl, version) = dep.to_purl_and_version(&key);

            deps.push(DependentPackage {
                purl,
                is_resolved: version != "*",
                requirement: version,
                scope: "dev".into(),
                is_runtime: false,
                relation: hashset! { crate::types::Relation::Direct },
                ..Default::default()
            });
        }

        Ok(Package {
            dependencies: deps,
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl PackageManifest for FpmToml {
    fn get_name(&self) -> String {
        "fortran".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["fpm.toml"]
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::FpmToml;

    #[test]
    fn file1() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/fortran/fpm-1.toml"
        ));

        dbg!(FpmToml::parse(&filepath).unwrap());
    }

    #[test]
    fn file2() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/fortran/fpm-2.toml"
        ));

        dbg!(FpmToml::parse(&filepath).unwrap());
    }
}