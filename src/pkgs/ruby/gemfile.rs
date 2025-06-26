use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::vec;

use anyhow::Result as AnyhowResult;
use lazy_static::lazy_static;
use packageurl::PackageUrl;
use regex::Regex;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package, PackageManifest};

lazy_static! {
    static ref GEMFILE_REGEXES: BTreeMap<&'static str, Regex> = {
        let mut m = BTreeMap::new();
        m.insert(
            "source",
            Regex::new(r"source:[ ]?(?P<source>[a-zA-Z:/\.-]+)").unwrap(),
        );
        m.insert(
            "git",
            Regex::new(r"git:[ ]?(?P<git>[a-zA-Z:/\.-]+)").unwrap(),
        );
        m.insert(
            "platform",
            Regex::new(r"platform:[ ]?(?P<platform>[a-zA-Z:/\.-]+)").unwrap(),
        );
        m.insert(
            "path",
            Regex::new(r"path:[ ]?(?P<path>[a-zA-Z:/\.-]+)").unwrap(),
        );
        m.insert(
            "branch",
            Regex::new(r"branch:[ ]?(?P<branch>[a-zA-Z:/\.-]+)").unwrap(),
        );
        m.insert(
            "autorequire",
            Regex::new(r"require:[ ]?(?P<autorequire>[a-zA-Z:/\.-]+)").unwrap(),
        );
        m.insert(
            "group",
            Regex::new(r"group:[ ]?(?P<group>[a-zA-Z:/\.-]+)").unwrap(),
        );
        m.insert(
            "name",
            Regex::new(r"(?P<name>[a-zA-Z]+[\.0-9a-zA-Z _-]*)").unwrap(),
        );
        m.insert(
            "requirement",
            Regex::new(r"(?P<requirement>([>|<|=|~>|\d]+[ ]*[0-9\.\w]+[ ,]*)+)").unwrap(),
        );
        m
    };
    static ref GROUP_BLOCK_REGEX: Regex =
        Regex::new(r"group[ ]?:[ ]?(?P<groupblock>.*?) do").unwrap();
    static ref GEMSPEC_ADD_DVTDEP_REGEX: Regex =
        Regex::new(r".*add_development_dependency(?P<line>.*)").unwrap();
    static ref GEMSPEC_ADD_RUNDEP_REGEX: Regex =
        Regex::new(r".*add_runtime_dependency(?P<line>.*)").unwrap();
    static ref GEMSPEC_ADD_DEP_REGEX: Regex = Regex::new(r".*dependency(?P<line>.*)").unwrap();
}

pub struct GemfileInner {
    current_group: String,
    dependencies: Vec<DependentPackage>,
    dependency_keys: HashSet<(String, String)>,
}

fn preprocess(line: &str) -> &str {
    if let Some(index) = line.find('#') {
        &line[..index]
    } else {
        line
    }
    .trim()
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

impl GemfileInner {
    pub fn new() -> Self {
        Self {
            current_group: "".into(),
            dependencies: vec![],
            dependency_keys: HashSet::new(),
        }
    }

    fn parse_line(&mut self, line: &str) {
        let mut column_list = vec![];
        let columns = line.split(',');
        for column in columns {
            let stripped_column = column.replace('\'', "");
            let stripped_column = stripped_column.replace('"', "");
            let stripped_column = stripped_column.replace("%q<", "");
            let stripped_column = stripped_column.replace('(', "");
            let stripped_column = stripped_column.replace(')', "");
            let stripped_column = stripped_column.replace('[', "");
            let stripped_column = stripped_column.replace(']', "");
            let stripped_column = stripped_column.trim().to_string();
            column_list.push(stripped_column)
        }

        let mut dep = DependentPackage {
            is_resolved: false,
            ..Default::default()
        };

        for column in column_list {
            for (criteria, criteria_regex) in GEMFILE_REGEXES.iter() {
                if let Some(captures) = criteria_regex.captures(&column) {
                    let criteria_value = captures[criteria.to_owned()].to_owned();
                    let criteria = criteria.to_string();

                    println!("{} {}", &criteria, &criteria_value);
                    if criteria == "requirement" {
                        dep.requirement = criteria_value;
                    } else if criteria == "group" {
                        dep.scope = criteria_value;
                    } else if criteria == "name" {
                        dep.purl = PackageUrl::new("gem", criteria_value)
                            .expect("purl arguments are invalid")
                            .to_string();
                    }
                    break;
                }
            }
        }

        if !dep.purl.is_empty()
            && !self
                .dependency_keys
                .contains(&(dep.purl.clone(), dep.requirement.clone()))
        {
            self.dependency_keys
                .insert((dep.purl.clone(), dep.requirement.clone()));
            self.dependencies.push(dep);
        }
    }

    fn parse_gemspec(&mut self, contents: Vec<String>) {
        for line in contents {
            let line = preprocess(&line);
            let mut matched = None;
            if let Some(captures) = GEMSPEC_ADD_DVTDEP_REGEX.captures(line) {
                self.current_group = "development".into();
                matched = Some(captures["line"].to_owned());
            } else if let Some(captures) = GEMSPEC_ADD_RUNDEP_REGEX.captures(line) {
                self.current_group = "runtime".into();
                matched = Some(captures["line"].to_owned());
            } else if let Some(captures) = GEMSPEC_ADD_DEP_REGEX.captures(line) {
                self.current_group = "dependency".into();
                matched = Some(captures["line"].to_owned());
            }

            if let Some(line) = matched {
                self.parse_line(&line);
            }
        }
    }

    pub fn parse_gemfile(&mut self, path: impl AsRef<Path>) -> AnyhowResult<()> {
        let mut contents = vec![];
        if let Ok(lines) = read_lines(path) {
            lines.for_each(|line| {
                if let Ok(ip) = line {
                    contents.push(ip);
                }
            });
        }

        let bk_contents = contents.clone();
        for line in contents {
            let line = preprocess(&line);
            if line.is_empty() || line.starts_with("source") {
                continue;
            } else if line.starts_with("group") {
                if let Some(captures) = GROUP_BLOCK_REGEX.captures(line) {
                    self.current_group = captures["groupblock"].into();
                }
            } else if line.starts_with("end") {
                self.current_group = "runtime".into();
            } else if line.starts_with("gemspec") {
                self.parse_gemspec(bk_contents.clone());
            } else if line.starts_with("gem ") {
                let line = &line[3..];
                self.parse_line(line);
            }
        }

        Ok(())
    }
}

pub struct Gemfile {}

impl Gemfile {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl PackageManifest for Gemfile {
    fn get_name(&self) -> String {
        "gemfile".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let lock_path = path.with_file_name("Gemfile.lock");
        if lock_path.exists() {
            log::info!("parsing Gemfile.lock");

            match super::gemfilelock::parse_file(&lock_path) {
                Ok(package) => return Ok(package),
                Err(err) => {
                    log::warn!("failed to parse Gemfile.lock: {}", err);
                }
            }
        }

        let mut parser = GemfileInner::new();
        parser.parse_gemfile(path)?;

        let package = Package {
            dependencies: parser.dependencies,
            ..Default::default()
        };

        Ok(package)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["Gemfile"]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gemfile_lock() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/gemfile/Gemfile"
        ));

        let mut parser = GemfileInner::new();
        parser.parse_gemfile(filepath).unwrap();
        println!("{:?}", parser.dependencies);
    }
}
