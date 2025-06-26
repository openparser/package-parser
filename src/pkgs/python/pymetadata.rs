use python_pkginfo::Metadata;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct PyMetadata {}

impl PyMetadata {
    pub fn new() -> Self {
        PyMetadata {}
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut fs = File::open(path)?;
        let mut content = Vec::new();
        fs.read_to_end(&mut content)?;
        let metadata = Metadata::parse(content.as_slice())?;
        let package = Package {
            name: metadata.name,
            version: metadata.version,
            primary_language: "Python".into(),
            declared_license: metadata.license.unwrap_or_default(),
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PyMetadata {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["PKG-INFO", "METADATA"]
    }
}
