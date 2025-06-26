use std::path::Path;

use packageurl::PackageUrl;
use crate::types::{DependentPackage, Package};
use serde::Deserialize;

use crate::{error::SourcePkgError, PackageManifest};

#[derive(Debug, Deserialize)]
struct Project {
    #[serde(rename = "ItemGroup")]
    item_group: ItemGroup,
}

#[derive(Debug, Deserialize)]
struct ItemGroup {
    #[serde(rename = "PackageVersion", default)]
    items: Vec<PackageVersion>,
}

#[derive(Debug, Deserialize)]
struct PackageVersion {
    #[serde(rename = "Include")]
    name: String,
    #[serde(rename = "Version")]
    version: String,
}

pub struct NuGetCentral;

impl NuGetCentral {
    pub fn new() -> Self {
        Self
    }

    async fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let content = tokio::fs::read_to_string(path).await?;
        let project: Project = quick_xml::de::from_str(&content)?;

        let mut dependencies = vec![];

        for item in project.item_group.items {
            dependencies.push(DependentPackage {
                purl: PackageUrl::new("nuget", item.name)
                    .unwrap()
                    .with_version(&item.version)
                    .to_string(),
                requirement: item.version,
                is_resolved: true,
                is_optional: false,
                is_runtime: true,
                ..Default::default()
            });
        }

        Ok(Package {
            dependencies,
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl PackageManifest for NuGetCentral {
    fn get_name(&self) -> String {
        "nuget".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let root = Self::parse(path).await?;
        Ok(root)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &[
            "Directory.packages.props",
            "Directory.Packages.props",
            "directory.packages.props",
            "directory.Packages.props",
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn central() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/nuget/Directory.packages.props"
        ));
        let package = NuGetCentral::new().recognize(path).await.unwrap();
        dbg!(package);
    }
}
