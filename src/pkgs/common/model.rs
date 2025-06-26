use std::path::{Path, PathBuf};

use anyhow::Result;
use fnmatch_regex::glob_to_regex;
use packageurl::PackageUrl;
pub use crate::types::{DependentPackage, Package, Party};
use serde_json::Value;

use crate::error::SourcePkgError;

pub fn get_filename_as_string(path: impl AsRef<Path>) -> Option<String> {
    let location = path.as_ref();
    match location.file_name() {
        Some(name) => {
            let name = name.to_os_string();
            let name = name.to_string_lossy();
            Some(name.to_string())
        }
        None => None,
    }
}

pub trait BaseModel {
    fn to_json(&self) -> Value;
}

pub fn is_manifest_default(path: &Path, patterns: &Vec<String>, extensions: &Vec<String>) -> bool {
    let location = path;

    let filename = match get_filename_as_string(location) {
        Some(filename) => filename,
        None => return false,
    };

    for pattern in patterns {
        match glob_to_regex(&pattern.to_ascii_lowercase()) {
            Ok(regex) => {
                if regex.is_match(&filename.to_ascii_lowercase()) {
                    return true;
                }
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    for extension in extensions {
        match glob_to_regex(&extension.to_ascii_lowercase()) {
            Ok(regex) => {
                if regex.is_match(&filename.to_ascii_lowercase()) {
                    return true;
                }
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    false
}

#[derive(Debug, Default)]
pub struct RecognizeContext {
    /// Prefix of the current file.
    ///
    /// A file should not reference any other file outside of its prefix.
    pub prefix: PathBuf,
}

#[async_trait::async_trait]
pub trait PackageManifest: Sync {
    fn get_name(&self) -> String;

    fn get_identifier(&self) -> String {
        self.get_name()
    }

    fn file_name_patterns(&self) -> &'static [&'static str];

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError>;

    async fn recognize_with_config(
        &self,
        path: &Path,
        _context: &RecognizeContext,
    ) -> Result<Package, SourcePkgError> {
        self.recognize(path).await
    }
}

pub struct DependentPackageBuilder {
    ty: String,
    name: String,
    inner: DependentPackage,
}

impl DependentPackageBuilder {
    pub fn new(
        ty: impl Into<String>,
        name: impl Into<String>,
        requirement: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        Self {
            ty: ty.into(),
            name: name.into(),
            inner: DependentPackage {
                requirement: requirement.into(),
                scope: scope.into(),
                ..Default::default()
            },
        }
    }

    pub fn with_is_runtime(mut self, is_runtime: bool) -> Self {
        self.inner.is_runtime = is_runtime;
        self
    }

    pub fn with_is_optional(mut self, is_optional: bool) -> Self {
        self.inner.is_optional = is_optional;
        self
    }

    pub fn with_is_resolved(mut self, is_resolved: bool) -> Self {
        self.inner.is_resolved = is_resolved;
        self
    }

    pub fn with_parents(mut self, parents: impl IntoIterator<Item = String>) -> Self {
        self.inner.parents = parents.into_iter().collect();
        self
    }

    pub fn build(mut self) -> Result<DependentPackage> {
        self.inner.purl = PackageUrl::new(self.ty, self.name)?
            .add_qualifier("is_runtime", self.inner.is_runtime.to_string())?
            .add_qualifier("is_optional", self.inner.is_optional.to_string())?
            .to_string();

        Ok(self.inner)
    }
}
