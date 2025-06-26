use maplit::hashset;
use packageurl::PackageUrl;
use quick_xml::Reader;
use crate::types::LockMavenParam;
use crate::types::Relation;
use serde::{Deserialize, Serialize};

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package, PackageManifest};

use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename = "metadata")]
pub struct Metadata {
    #[serde(rename = "groupId")]
    group_id: String,
    #[serde(rename = "artifactId")]
    artifact_id: String,
    versioning: Versioning,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "versioning")]
pub struct Versioning {
    latest: String,
    release: String,
    versions: Versions,
    #[serde(rename = "lastUpdated")]
    last_updated: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "versions")]
pub struct Versions {
    #[serde(rename = "version", default)]
    versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename = "dependencies")]
pub struct Dependencies {
    #[serde(default)]
    pub dependency: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parent {
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub version: Option<String>,
    pub relative_path: Option<String>,
}

// namespace = "http://maven.apache.org/POM/4.0.0"
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename = "project", rename_all = "camelCase")]
pub struct MavenPom {
    // Coordinates
    group_id: Option<String>,
    artifact_id: Option<String>,
    version: Option<String>,

    parent: Option<Parent>,
    name: Option<String>,
    #[serde(default)]
    dependencies: Dependencies,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ToolError {
    problems: Vec<String>,
}

fn parse_properties<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<HashMap<String, String>, SourcePkgError> {
    use quick_xml::events::Event;

    let mut buffer = Vec::new();

    let mut properties = HashMap::new();

    let mut current_key = None;
    let mut current_value = String::new();

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(element) => {
                current_key = Some(String::from_utf8(element.name().as_ref().to_vec())?);
            }
            Event::Text(text) => {
                let text = text.unescape()?;
                current_value.push_str(&text);
            }
            Event::End(element) => {
                let name = element.name();
                match current_key.take() {
                    Some(key) => {
                        if key.as_bytes() == name.as_ref() {
                            properties
                                .insert(format!("${{{key}}}"), current_value.trim().to_string());
                            current_value.clear();
                        } else {
                            return Err(SourcePkgError::GenericsError(
                                "Failed to parse properties, tags mismatch",
                            ));
                        }
                    }
                    None => {
                        if name.as_ref() == b"properties" {
                            return Ok(properties);
                        } else {
                            return Err(SourcePkgError::GenericsError(
                                "Failed to parse properties, tags mismatch",
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
        // Clear the buffer to keep memory usage low
        buffer.clear();
    }
}

#[allow(dead_code)]
pub struct JavaMavenPom {
}

impl JavaMavenPom {
    pub fn new() -> Self {
        Self {}
    }

    async fn parse_fallback(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<LockMavenParam>, SourcePkgError> {
        use quick_xml::events::Event;

        let path = path.as_ref();
        let file = tokio::fs::read(path).await?;
        let file_str = crate::pkgs::common::decode_string(&file)?;

        let mut reader = quick_xml::Reader::from_str(&file_str);
        let mut buffer = Vec::new();

        let mut properties = loop {
            match reader.read_event_into(&mut buffer)? {
                Event::Eof => break HashMap::new(),
                Event::Start(e) => {
                    if e.name().as_ref() == b"properties" {
                        break parse_properties(&mut reader)?;
                    }
                }
                _ => {}
            }
            // Clear the buffer to keep memory usage low
            buffer.clear();
        };

        let root = quick_xml::de::from_str::<MavenPom>(&file_str)?;

        if let Some(ref s) = root.artifact_id {
            properties.insert("${project.artifactId}".to_string(), s.clone());
        }
        if let Some(ref s) = root.group_id {
            properties.insert("${project.groupId}".to_string(), s.clone());
        }

        let process_properties = |version: &str| -> String {
            if version.contains("${") {
                let mut version = version.to_string();
                for (key, value) in &properties {
                    version = version.replace(key, value);
                }
                version
            } else {
                version.to_string()
            }
        };

        let requirements = root
            .dependencies
            .dependency
            .into_iter()
            .map(|dep| LockMavenParam {
                group_id: process_properties(&dep.group_id),
                artifact_id: process_properties(&dep.artifact_id),
                version: process_properties(&dep.version.unwrap_or_default()),
                scope: dep.scope,
                is_optional: false,
            })
            .collect();

        Ok(requirements)
    }

    async fn parse(&self, path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let path = path.as_ref();

        let dependencies = self.parse_fallback(path).await?;

        fn convert_to_package(dep: LockMavenParam) -> DependentPackage {
            let scope = dep.scope.unwrap_or_else(|| "compile".into());
            let is_runtime = matches!(scope.as_str(), "compile" | "runtime" | "provided");

            DependentPackage {
                purl: PackageUrl::new("maven", dep.artifact_id)
                    .unwrap()
                    .with_namespace(dep.group_id)
                    .to_string(),
                requirement: dep.version,
                scope,
                is_runtime,
                is_optional: false,
                is_resolved: false,
                relation: hashset! {Relation::Direct},
                ..Default::default()
            }
        }

        let dependent_packages = dependencies
            .into_iter()
            .map(convert_to_package)
            .collect();

        let package = Package {
            dependencies: dependent_packages,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for JavaMavenPom {
    fn get_name(&self) -> String {
        "maven".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        self.parse(path).await
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["pom.xml", "pom.xml", "*.pom"]
    }
}
