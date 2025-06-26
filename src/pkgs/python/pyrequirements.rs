use packageurl::PackageUrl;
use requirements::enums::Comparison;
use crate::types::DependentPackage;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::path::Path;

pub struct PyRequirements {}

impl PyRequirements {
    pub fn new() -> Self {
        Self {}
    }

    fn specs_to_string(specs: &[(Comparison, String)]) -> String {
        specs
            .iter()
            .map(|(comparison, version)| format!("{}{}", comparison, version))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn parse_requirement_content(
        content: &str,
    ) -> Result<Vec<DependentPackage>, SourcePkgError> {
        let requirements =
            requirements::parse_str(content).map_err(SourcePkgError::GenericsError2)?;

        let mut dependencies = vec![];

        for requirement in requirements {
            let name = if let Some(name) = requirement.name {
                super::normalize_name(&name)
            } else {
                continue;
            };

            let mut exact_version = None;

            if requirement.specs.len() == 1 {
                let (op, version) = &requirement.specs[0];

                if *op == Comparison::Equal || *op == Comparison::ArbitraryEqual {
                    exact_version = Some(version.clone());
                }
            }

            let mut purl = PackageUrl::new("pypi", name).unwrap();
            if let Some(v) = &exact_version {
                purl.with_version(v);
            }

            let dependency = DependentPackage {
                purl: purl.to_string(),
                is_resolved: exact_version.is_some(),
                requirement: if let Some(v) = exact_version {
                    v
                } else {
                    Self::specs_to_string(&requirement.specs)
                },
                ..Default::default()
            };

            dependencies.push(dependency);
        }

        Ok(dependencies)
    }

    pub fn parse_requirement(
        path: impl AsRef<Path>,
    ) -> Result<Vec<DependentPackage>, SourcePkgError> {
        let content_bytes = std::fs::read(path)?;

        let mut content = crate::pkgs::common::decode_string(&content_bytes)?;

        if !content.ends_with('\n') {
            content.push('\n');
        }

        Self::parse_requirement_content(&content)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PyRequirements {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let dependencies = Self::parse_requirement(path)?;
        let package = Package {
            dependencies,
            ..Default::default()
        };

        Ok(package)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &[
            "*requirement*.txt",
            "*requirement*.pip",
            "*requirement*.in",
            "*requires.txt",
            "*requirements/*.txt",
            "*requirements/*.pip",
            "*requirements/*.in",
            "*reqs.txt",
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::helper::testing::*;
    use std::path::Path;

    #[test]
    fn all_requirements_txt() {
        let dir_path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pypi/requirements_txt"
        ));

        for result in all_files_with_extensions(dir_path, &["txt"]) {
            let path = result.expect("Failed to walk");
            println!("{}", path.display());
            let _ = PyRequirements::parse_requirement(path).expect("Failed to parse");
        }
    }
}
