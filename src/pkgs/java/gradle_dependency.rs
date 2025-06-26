use nom::sequence::preceded;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::not_line_ending,
    combinator::recognize,
    error::VerboseError,
    sequence::delimited,
    IResult,
};
use packageurl::PackageUrl;
use crate::types::DependentPackage;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::fs::File;
use std::io::Read;
use std::path::Path;

type NomResult<T, U> = IResult<T, U, VerboseError<T>>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct PackageDescriptor {
    pub name: String,
    pub version: String,
}

// https://github.com/phylum-dev/cli/blob/ecd02687f694356f3c71630d40cff73b9e84084a/cli/tests/fixtures/gradle-dependencies.txt

pub fn parse(input: &str) -> NomResult<&str, Vec<PackageDescriptor>> {
    let pkgs = input.lines().filter_map(package).collect::<Vec<_>>();
    Ok((input, pkgs))
}

fn group_id(input: &str) -> NomResult<&str, &str> {
    recognize(take_until(":"))(input)
}

fn artifact_id_version(input: &str) -> NomResult<&str, &str> {
    let (input, artifact_id) = delimited(tag(":"), take_until(":"), tag(":"))(input)?;
    let (_, version) = recognize(alt((take_until(" ("), not_line_ending)))(input)?;
    Ok((artifact_id, version))
}

fn filter_line(input: &str) -> NomResult<&str, &str> {
    let (input, _) = recognize(alt((
        take_until("+---"),
        take_until("\\---"),
        not_line_ending,
    )))(input)?;
    preceded(alt((tag("+--- "), tag("\\--- "))), not_line_ending)(input)
}

fn package(input: &str) -> Option<PackageDescriptor> {
    let (_, input) = filter_line(input).ok()?;
    let (input, group_id) = group_id(input).ok()?;
    let (artifact_id, version) = artifact_id_version(input).ok()?;

    Some(PackageDescriptor {
        name: format!("{}:{}", group_id, artifact_id),
        version: version.to_string(),
    })
}

pub struct GradleDependencies {}

impl GradleDependencies {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_gradle_lock(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let (_, requirements) =
            parse(&content).map_err(|e| SourcePkgError::GenericsError2(e.to_string()))?;

        let requirements = requirements
            .into_iter()
            .map(|desc| DependentPackage {
                purl: PackageUrl::new("maven", desc.name)
                    .expect("purl arguments are invalid")
                    .to_string(),
                requirement: desc.version,
                is_resolved: true,
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let package = Package {
            dependencies: requirements,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for GradleDependencies {
    fn get_name(&self) -> String {
        "maven".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse_gradle_lock(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["gradle-dependencies.txt", "dependencies.txt"]
    }
}

