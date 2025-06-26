use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};
use crate::pkgs::spec::Spec;

use std::path::Path;

pub struct RubyGems {}

impl RubyGems {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl PackageManifest for RubyGems {
    fn get_name(&self) -> String {
        "gem".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let spec = Spec::new();
        let spec_info = spec.parse_spec(path)?;

        let package = Package {
            name: spec_info.name.unwrap_or_default(),
            version: spec_info.version.unwrap_or_default(),
            primary_language: "Ruby".into(),
            declared_license: spec_info.license.unwrap_or_default(),
            ..Default::default()
        };

        Ok(package)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["*.gemspec"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_ruby_gems() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/rubygems/gemspec/arel.gemspec"
        ));

        let parser = RubyGems::new();
        let p = parser.recognize(&filepath).await.unwrap();
        println!("{:?}", p);
    }
}
