#![allow(clippy::new_without_default)]

pub mod error;
pub mod helper;
pub mod pkgs;
pub mod types;

use globset::{Glob, GlobSet, GlobSetBuilder};
pub use pkgs::common::model::{DependentPackage, Package, PackageManifest};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use crate::types::SupportedType;


#[derive(Clone)]
pub struct Scanner {

    scanners: Vec<Arc<dyn PackageManifest + Send + Sync>>,
    types: Vec<SupportedType>,

    glob_index_to_scanner_index: HashMap<usize, usize>,
    glob_set: GlobSet,
}

impl Scanner {
    pub fn new() -> Self {
        let scanners = pkgs::create_scanners();

        let mut glob_index_to_scanner_index = HashMap::new();
        let mut glob_set = GlobSetBuilder::new();
        let mut current_glob_index = 0usize;

        let mut supported_types = vec![];
        for (index, scanner) in scanners.iter().enumerate() {
            let supported_type = SupportedType {
                name: scanner.get_identifier(),
                filenames: scanner
                    .file_name_patterns()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                patterns: vec![],
            };
            supported_types.push(supported_type);

            for pat in scanner.file_name_patterns() {
                let glob = Glob::new(pat).expect("Failed to compile pattern");
                glob_set.add(glob);
                glob_index_to_scanner_index.insert(current_glob_index, index);
                current_glob_index += 1;
            }
        }

        let glob_set = glob_set.build().expect("Failed to build glob set");

        Self {
            scanners,
            types: supported_types,

            glob_index_to_scanner_index,
            glob_set,
        }
    }

    pub async fn scan(
        &self,
        path: impl AsRef<Path>,
        prefix: impl AsRef<Path>,
    ) -> Result<(String, Package), error::SourcePkgError> {
        let location = path.as_ref();
        let prefix = prefix.as_ref();

        if let Some(match_idx) = self
            .glob_set
            .matches(
                location
                    .file_name()
                    .ok_or(error::SourcePkgError::GenericsError(
                        "Invalid file name ending in '..'",
                    ))?,
            )
            .first()
        {
            let scanner_idx = self.glob_index_to_scanner_index[match_idx];
            let scanner = &self.scanners[scanner_idx];
            let ctx = pkgs::RecognizeContext {
                prefix: prefix.to_path_buf(),
            };

            return match scanner.recognize_with_config(location, &ctx).await {
                Ok(manifest) => Ok((scanner.get_name(), manifest)),
                Err(e) => Err(e),
            };
        }

        Err(error::SourcePkgError::NotSupported)
    }

    pub fn supported_types(&self) -> &[SupportedType] {
        &self.types
    }
}
