use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};
use crate::pkgs::spec::Spec;

use std::path::Path;

pub struct CocoaPods {}

impl CocoaPods {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl PackageManifest for CocoaPods {
    fn get_name(&self) -> String {
        "cocoapods".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let spec = Spec::new();
        let spec_info = spec.parse_spec(path)?;

        let package = Package {
            name: spec_info.name.unwrap_or_default(),
            version: spec_info.version.unwrap_or_default(),
            declared_license: spec_info.license.unwrap_or_default(),
            ..Default::default()
        };

        Ok(package)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["*.podspec"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_cocoapods() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/cocoapods/podspec/BadgeHub.podspec"
        ));

        let parser = CocoaPods::new();
        let p = parser.recognize(&filepath).await.unwrap();
        println!("{:?}", p);
    }
}
