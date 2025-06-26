use packageurl::PackageUrl;
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package, PackageManifest};

use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename = "package")]
struct NuSpec {
    pub metadata: NuSpecMetadata,
    // #[serde(default)]
    // pub files: Vec<NuSpecFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NuSpecMetadata {
    // Required fields
    #[serde(rename = "id", default)]
    pub id: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "description")]
    pub description: String,
    // TODO: comma-separated
    #[serde(rename = "authors")]
    pub authors: String,

    // Attributes
    #[serde(rename = "minClientVersion")]
    pub min_client_version: Option<String>,

    // Optional fields
    // TODO: comma-separated
    #[serde(rename = "owners")]
    pub owners: Option<String>,
    #[serde(rename = "projectUrl")]
    pub project_url: Option<String>,
    #[serde(rename = "licenseUrl")]
    pub license_url: Option<String>,
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<String>,
    #[serde(rename = "icon")]
    pub icon: Option<String>,
    #[serde(rename = "readme")]
    pub readme: Option<String>,
    #[serde(rename = "requireLicenseAcceptance")]
    pub require_license_acceptance: Option<bool>,
    #[serde(rename = "license")]
    pub license: Option<String>,
    #[serde(rename = "copyright")]
    pub copyright: Option<String>,
    #[serde(rename = "developmentDependency")]
    pub development_dependency: Option<bool>,
    #[serde(rename = "releaseNotes")]
    pub release_notes: Option<String>,
    // TODO: space-separated
    #[serde(rename = "tags")]
    pub tags: Option<String>,
    #[serde(rename = "language")]
    pub language: Option<String>,
    #[serde(rename = "repository")]
    pub repository: Option<NuSpecRepository>,

    // Collections
    #[serde(rename = "dependencies")]
    pub dependencies: Option<NuSpecDependencies>,
    #[serde(rename = "frameworkAssemblies")]
    pub framework_assemblies: Option<Vec<NuSpecFrameworkAssembly>>,
    #[serde(rename = "packageTypes")]
    pub package_types: Option<Vec<NuSpecPackageType>>,
    #[serde(rename = "references")]
    pub references: Option<Vec<NuSpecReference>>,
    // #[serde(rename = "contentFiles")]
    // pub content_files: Option<Vec<NuSpecContentFiles>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NuSpecRepository {
    #[serde(rename = "type")]
    pub repo_type: Option<String>,
    pub url: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NuSpecFile {
    pub src: String,
    pub target: String,
    pub exclude: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NuSpecDependencies {
    #[serde(rename = "group", default)]
    groups: Vec<NuSpecDependencyGroup>,
    #[serde(rename = "dependency", default)]
    dependencies: Vec<NuSpecDependency>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NuSpecDependencyGroup {
    target_framework: Option<String>,
    #[serde(rename = "dependency", default)]
    dependencies: Vec<NuSpecDependency>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NuSpecDependency {
    pub id: String,
    pub version: String,
    pub exclude: Option<String>,
    pub include: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NuSpecFrameworkAssembly {
    pub assembly_name: Option<String>,
    pub target_framework: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NuSpecPackageType {
    Dependency,
    DotnetTool,
    Template,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum NuSpecReferenceOrGroup {
    Group {
        #[serde(rename = "targetFramework")]
        target_framework: String,
        #[serde(rename = "reference", default)]
        references: Vec<NuSpecReference>,
    },
    Reference(NuSpecReference),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NuSpecReference {
    pub file: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NuSpecContentFiles {
    pub include: String,
    pub exclude: Option<String>,
    #[serde(rename = "buildAction")]
    pub build_action: Option<String>,
    #[serde(rename = "copyToOutput")]
    pub copy_to_output: Option<bool>,
    pub flatten: Option<bool>,
}

pub struct DotnetNuSpec {}

impl DotnetNuSpec {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_dotnet_nuspec(path: impl AsRef<Path>) -> Result<NuSpec, SourcePkgError> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let root = from_str::<NuSpec>(&content)?;
        Ok(root)
    }
}

#[async_trait::async_trait]
impl PackageManifest for DotnetNuSpec {
    fn get_name(&self) -> String {
        "nuget".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let root = Self::parse_dotnet_nuspec(path)?;
        let package = Package {
            name: root.metadata.id,
            version: root.metadata.version,
            dependencies: root
                .metadata
                .dependencies
                .into_iter()
                .map(|dependencies| {
                    let mut deps = vec![];
                    for dependency_groups in dependencies.groups {
                        let scope = dependency_groups.target_framework.unwrap_or_default();

                        for dependency in dependency_groups.dependencies {
                            let dep = DependentPackage {
                                purl: PackageUrl::new("nuget", dependency.id)
                                    .expect("purl arguments are invalid")
                                    .to_string(),
                                requirement: dependency.version,
                                scope: scope.clone(),
                                ..Default::default()
                            };
                            deps.push(dep);
                        }
                    }
                    deps
                })
                .collect::<Vec<_>>()
                .concat(),
            ..Default::default()
        };

        Ok(package)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["*.nuspec"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::testing::*;
    use std::path::Path;

    #[tokio::test]
    async fn all_nuspec() {
        let dir_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/nuget"));
        let parser = DotnetNuSpec::new();

        for result in all_files_with_extensions(dir_path, &["nuspec"]) {
            let path = result.expect("Failed to walk");
            println!("{}", path.display());
            let _ = parser.recognize(&path).await.expect("Failed to parse");
        }
    }
}
