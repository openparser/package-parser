use std::collections::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DependentPackage {
    /// Dependent package URL
    /// A compact purl package URL. Typically when there is an
    /// unresolved requirement, there is no version.
    /// If the dependency is resolved, the version should be added to the purl
    pub purl: String,

    /// dependent package version requirement
    /// A string defining version(s)requirements. Package-type specific.
    pub requirement: String,

    /// dependency scope
    /// The scope of this dependency, such as runtime, install, etc.
    /// This is package-type specific and is the original scope string.
    #[serde(default)]
    pub scope: String,

    /// is runtime flag
    /// True if this dependency is a runtime dependency.
    pub is_runtime: bool,

    /// is optional flag
    /// True if this dependency is an optional dependency'
    pub is_optional: bool,

    /// is resolved flag
    /// True if this dependency version requirement has
    /// been resolved and this dependency url points to an
    /// exact version.
    pub is_resolved: bool,

    /// is reachable flag
    #[serde(default)]
    pub reachable: Reachability,

    /// The relation of this dependency to the top package.
    #[serde(default)]
    pub relation: HashSet<Relation>,

    /// parents, in purl format
    #[serde(default)]
    pub parents: HashSet<String>,
}

impl Default for DependentPackage {
    fn default() -> Self {
        Self {
            purl: String::new(),
            requirement: String::new(),
            scope: String::new(),
            is_runtime: true,
            is_optional: false,
            is_resolved: false,
            reachable: Default::default(),
            relation: HashSet::new(),
            parents: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Relation {
    /// This is a direct dependency of the package
    Direct,
    /// This is a transitive dependency of the package
    Indirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Reachability {
    Yes,
    #[default]
    Possible,
    No,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Maintainer {
    pub name: Option<String>,
    pub email: Option<String>,
}

/// A party is a person, project or organization related to a package.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Party {
    /// the type of this party: One of:
    /// person, project, organization
    #[serde(rename = "type")]
    pub typ: String,

    /// A role for this party. Something such as author,
    /// maintainer, contributor, owner, packager, distributor,
    /// vendor, developer, owner, etc.
    pub role: String,

    /// The name of this party.
    pub name: String,

    /// The email address of this party.
    pub email: String,

    /// The url of this party.
    pub url: String,
}

/// A package object as represented by either data from one of its different types of
/// package manifests or that of a package instance created from one or more of these
/// package manifests, and files for that package.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Package {
    /// Optional namespace for this package.
    pub namespace: String,

    /// The name of this package.
    pub name: String,

    /// Optional version of this package.
    pub version: String,

    /// Primary programming language
    pub primary_language: String,

    /// The license expression for this package typically derived
    /// from its declared license or from some other type-specific
    /// routine or convention.
    pub license_expression: String,

    /// The declared license mention, tag or text as found in a
    /// package manifest. This can be a string, a list or dict of
    /// strings possibly nested, as found originally in the manifest.
    pub declared_license: String,

    /// A list of DependentPackage for this package.
    pub dependencies: Vec<DependentPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockMavenParam {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default)]
    pub is_optional: bool,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SupportedType {
    /// Name of the file type, for logging purposes
    pub name: String,
    /// Glob patterns for file names
    pub filenames: Vec<String>,
    /// Regular expression patterns to match against the output of `file` command
    pub patterns: Vec<String>,
}

