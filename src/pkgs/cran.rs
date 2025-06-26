use serde_yaml::from_reader;
use serde_yaml::Value;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::fs::File;
use std::path::Path;

pub struct Cran {}

impl Cran {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let file = File::open(path)?;
        let root: Value = from_reader(file)?;
        let package = Package {
            name: match &root["Package"] {
                Value::String(s) => s.into(),
                _ => "".into(),
            },
            declared_license: match &root["License"] {
                Value::String(s) => s.into(),
                _ => "".into(),
            },
            version: match &root["Version"] {
                Value::String(s) => s.into(),
                _ => "".into(),
            },
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for Cran {
    fn get_name(&self) -> String {
        "cran".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["DESCRIPTION"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cran() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/cran/geometry/DESCRIPTION"
        ));

        let p = Cran::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
