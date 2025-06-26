use packageurl::PackageUrl;
use crate::types::DependentPackage;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn is_gradle_lockfile_depline(line: &str) -> bool {
    let ret = line.starts_with('#') || line.starts_with("empty=");
    !ret
}

fn parse_to_gradle_dependency(line: &str) -> Option<DependentPackage> {
    let parts: Vec<&str> = line.splitn(3, ':').collect();
    if parts.len() < 3 {
        return None;
    }

    let group = parts[0];
    let artifact = parts[1];
    let (version, configurations) =
        if let Some((version, configurations)) = parts[2].split_once('=') {
            (Some(version), Some(configurations))
        } else {
            (None, None)
        };

    let is_runtime = configurations
        .map(|c| {
            c.contains("runtime")
                || c.contains("Runtime")
                || c.contains("compile")
                || c.contains("Compile")
        })
        .unwrap_or(false);

    let mut purl = PackageUrl::new("maven", artifact).unwrap();
    purl.with_namespace(group);
    if let Some(version) = version {
        purl.with_version(version);
    };

    Some(DependentPackage {
        purl: purl.to_string(),
        is_resolved: true,
        requirement: version.unwrap_or_default().to_string(),
        is_runtime,
        ..Default::default()
    })
}

pub struct GradleLock {}

impl GradleLock {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_gradle_lock(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut contents = vec![];
        if let Ok(lines) = read_lines(path) {
            lines.for_each(|line| {
                if let Ok(ip) = line {
                    contents.push(ip);
                }
            });
        }

        let mut package = Package {
            ..Default::default()
        };

        for line in contents {
            let line = line.trim();
            if !is_gradle_lockfile_depline(line) {
                continue;
            }

            if let Some(dep) = parse_to_gradle_dependency(line) {
                package.dependencies.push(dep);
            };
        }

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for GradleLock {
    fn get_name(&self) -> String {
        "maven".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse_gradle_lock(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["gradle.lockfile"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gradle_dep() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/gradle_lock/5-pkg"
        ));

        let package = GradleLock::parse_gradle_lock(filepath).unwrap();
        println!("{:#?}", package);
    }

    #[test]
    fn parse_gradle_lockfile() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/gradle_lock/gradle.lockfile"
        ));

        let package = GradleLock::parse_gradle_lock(filepath).unwrap();
        println!("{:#?}", package);
    }
}
