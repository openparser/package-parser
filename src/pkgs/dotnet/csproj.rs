use packageurl::PackageUrl;

use serde::Deserialize;
use serde_xml_rs::Deserializer;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package, PackageManifest};

use std::path::Path;

const INVALID_CHAR: &str = "\u{feff}";

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct PackageReference {
    #[serde(alias = "Include", default)]
    pub name: String,

    #[serde(alias = "Version", default)]
    pub version: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ItemGroup {
    #[serde(alias = "PackageReference", default)]
    pub dependencies: Vec<PackageReference>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Project {
    #[serde(rename = "ItemGroup", default)]
    pub item_groups: Vec<ItemGroup>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackagesConfigItem {
    id: String,
    version: String,
    target_framework: String,
    development_dependency: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PackagesConfig {
    package: Vec<PackagesConfigItem>,
}

fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
    let path = path.as_ref();

    let mut deps = vec![];

    // Check for `packages.config` file (XML).
    let mut packages_config_path = path.to_owned();
    packages_config_path.pop();
    packages_config_path.push("packages.config");
    if packages_config_path.exists() {
        let content = std::fs::read_to_string(packages_config_path)?
            .trim_start_matches(INVALID_CHAR)
            .to_string();
        let mut de =
            Deserializer::new_from_reader(content.as_bytes()).non_contiguous_seq_elements(true);

        match PackagesConfig::deserialize(&mut de) {
            Ok(parsed) => {
                for pkg in parsed.package {
                    let dep = DependentPackage {
                        purl: PackageUrl::new("nuget", pkg.id)
                            .expect("purl arguments are invalid")
                            .to_string(),
                        requirement: pkg.version,
                        scope: pkg.target_framework,
                        is_resolved: true,
                        is_runtime: if let Some(t) = pkg.development_dependency {
                            t == "false"
                        } else {
                            true
                        },
                        ..Default::default()
                    };
                    deps.push(dep);
                }
            }
            Err(e) => {
                log::warn!("Failed to parse packages.config: {}", e);
            }
        }
    }

    let content = std::fs::read_to_string(path)?
        .trim_start_matches(INVALID_CHAR)
        .to_string();

    let mut de =
        Deserializer::new_from_reader(content.as_bytes()).non_contiguous_seq_elements(true);
    let parsed =
        Project::deserialize(&mut de).map_err(|e| SourcePkgError::GenericsError2(e.to_string()))?;

    for item_group in parsed.item_groups {
        deps.extend(item_group.dependencies.into_iter().map(|dependency| {
            // version come from the csharp.csproj file must met the semver principle
            // otherwise it can be dynamic upon the runtime
            let locked_version = match semver::Version::parse(&dependency.version) {
                Ok(version) => version.to_string(),
                Err(_) => "".into(),
            };

            DependentPackage {
                purl: PackageUrl::new("nuget", dependency.name)
                    .expect("purl arguments are invalid")
                    .to_string(),
                requirement: locked_version.clone(),
                is_resolved: !locked_version.is_empty(),
                ..Default::default()
            }
        }));
    }

    let package = Package {
        dependencies: deps,
        ..Default::default()
    };

    Ok(package)
}

pub struct CSharpCsproj {}

impl CSharpCsproj {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl PackageManifest for CSharpCsproj {
    fn get_name(&self) -> String {
        "nuget".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["*.csproj"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csproj() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/csharp/ICSharpCode.Decompiler.csproj"
        ));

        let p = parse(filepath).unwrap();
        println!("{:?}", p);
    }

    #[test]
    fn test_csproj_with_packages() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/csharp_with_package/Snyk.Common.csproj"
        ));

        let p = parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
