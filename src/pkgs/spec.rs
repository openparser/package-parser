use lazy_static::lazy_static;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::vec;

use crate::error::SourcePkgError;

#[derive(Debug, Clone, Default)]
pub struct SpecInfo {
    pub name: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub source: Option<String>,
    pub emails: Vec<String>,
    pub authors: Vec<String>,
}

/// Return line after comments and space.
fn pre_process(line: &mut String) -> String {
    let newline = line
        .find('#')
        .map(|i| line.split_off(i))
        .unwrap_or_else(|| line.to_string());
    newline.trim().to_string()
}

/// Return data after removing unnecessary special character
fn get_stripped_data(data: &mut str) -> String {
    let new_data = data
        .replace(['\'', '\"', '{', '}', '[', ']'], "")
        .replace("%q", "");

    new_data.trim().to_string()
}

/// Return description from spec file.
///   s.description  = <<-DESC
///                     Nanopb is a small code-size Protocol Buffers implementation
///                     in ansi C. It is especially suitable for use in
///                     microcontrollers, but fits any memory restricted system.
///                    DESC
fn get_description(path: impl AsRef<Path>) -> Option<String> {
    let location = path.as_ref();
    let fs = match File::open(location) {
        Ok(fs) => fs,
        Err(_) => return None,
    };

    let mut description = String::new();
    let mut enter_description_section = false;
    for line in BufReader::new(fs).lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue,
        };

        if line.contains(".description") {
            enter_description_section = true;
            continue;
        }

        if enter_description_section {
            let line = line.trim().to_string();
            if line.contains("DESC") {
                break;
            }
            description.push_str(&line);
        }
    }
    Some(description)
}

pub struct Spec;

lazy_static! {
    static ref PARSE_NAME: Regex = Regex::new(r".*\.name(\s*)=(?P<name>.*)").unwrap();
    static ref PARSE_VERSION: Regex = Regex::new(r".*\.version(\s*)=(?P<version>.*)").unwrap();
    static ref PARSE_LICENSE: Regex = Regex::new(r".*\.license(\s*)=(?P<license>.*)").unwrap();
    static ref PARSE_SUMMARY: Regex = Regex::new(r".*\.summary(\s*)=(?P<summary>.*)").unwrap();
    static ref PARSE_DESCRIPTION: Regex =
        Regex::new(r".*\.description(\s*)=(?P<description>.*)").unwrap();
    static ref PARSE_HOMEPAGE: Regex = Regex::new(r".*\.homepage(\s*)=(?P<homepage>.*)").unwrap();
    static ref PARSE_SOURCE: Regex = Regex::new(r".*\.source(\s*)=(?P<source>.*)").unwrap();
}

impl Spec {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_spec(&self, path: impl AsRef<Path>) -> Result<SpecInfo, SourcePkgError> {
        let location = path.as_ref();
        let fs = File::open(location).map_err(SourcePkgError::Io)?;

        let mut spec_info = SpecInfo::default();
        for line in BufReader::new(fs).lines() {
            let mut line = line.unwrap_or_default();
            let line = pre_process(&mut line);

            if let Some(name) = self.parse_name(&line) {
                spec_info.name = Some(name);
            }

            if let Some(version) = self.parse_version(&line) {
                spec_info.version = Some(version);
            }

            if let Some(license) = self.parse_license(&line) {
                spec_info.license = Some(license);
            }

            if let Some(summary) = self.parse_summary(&line) {
                spec_info.summary = Some(summary)
            }

            if let Some(homepage) = self.parse_homepage(&line) {
                spec_info.homepage = Some(homepage);
            }

            if let Some(description) = self.parse_description(&path, &line) {
                spec_info.description = Some(description);
            }

            let emails = self.parse_email(&line);
            if !emails.is_empty() {
                spec_info.emails.extend(emails);
            }
        }

        Ok(spec_info)
    }

    fn parse_name(&self, line: &str) -> Option<String> {
        if let Some(captures) = PARSE_NAME.captures(line) {
            let mut name = match captures.name("name") {
                Some(name) => name.as_str().to_string(),
                None => return None,
            };
            let name = get_stripped_data(&mut name);
            return Some(name);
        }

        None
    }

    fn parse_version(&self, line: &str) -> Option<String> {
        if let Some(captures) = PARSE_VERSION.captures(line) {
            let mut version = match captures.name("version") {
                Some(version) => version.as_str().to_string(),
                None => return None,
            };
            let version = get_stripped_data(&mut version);
            return Some(version);
        }

        None
    }

    fn parse_license(&self, line: &str) -> Option<String> {
        if let Some(captures) = PARSE_LICENSE.captures(line) {
            let mut license = match captures.name("license") {
                Some(license) => license.as_str().to_string(),
                None => return None,
            };
            let license = get_stripped_data(&mut license);
            return Some(license);
        }

        None
    }

    fn parse_summary(&self, line: &str) -> Option<String> {
        if let Some(captures) = PARSE_SUMMARY.captures(line) {
            let mut summary = match captures.name("summary") {
                Some(summary) => summary.as_str().to_string(),
                None => return None,
            };
            let summary = get_stripped_data(&mut summary);
            return Some(summary);
        }

        None
    }

    fn parse_homepage(&self, line: &str) -> Option<String> {
        if let Some(captures) = PARSE_HOMEPAGE.captures(line) {
            let mut homepage = match captures.name("homepage") {
                Some(homepage) => homepage.as_str().to_string(),
                None => return None,
            };
            let homepage = get_stripped_data(&mut homepage);
            return Some(homepage);
        }

        None
    }

    #[allow(dead_code)]
    fn parse_source(&self, line: &str) -> Option<String> {
        lazy_static! {
            static ref SOURCE_REGEX1: Regex = Regex::new(r"/*.*source.*?>").unwrap();
            static ref SOURCE_REGEX2: Regex = Regex::new(r",.*").unwrap();
        };

        if let Some(captures) = PARSE_SOURCE.captures(line) {
            let source = match captures.name("source") {
                Some(source) => source.as_str().to_string(),
                None => return None,
            };

            let source = SOURCE_REGEX1.replace_all(&source, "");
            let mut stripped_source = SOURCE_REGEX2.replace_all(&source, "").to_string();
            let stripped_source = get_stripped_data(&mut stripped_source);
            return Some(stripped_source);
        }

        None
    }

    fn parse_description(&self, path: impl AsRef<Path>, line: &str) -> Option<String> {
        if let Some(captures) = PARSE_DESCRIPTION.captures(line) {
            let mut description = match captures.name("description") {
                Some(description) => description.as_str().to_string(),
                None => return None,
            };

            let location = path.as_ref();
            let location = location.to_string_lossy();
            if location.ends_with(".gemspec") {
                // FIXME: description can be in single or multi-lines
                // There are many different ways to write description.
                let description = get_stripped_data(&mut description);
                return Some(description);
            } else {
                return get_description(path);
            }
        }

        None
    }

    fn parse_email(&self, line: &str) -> Vec<String> {
        if line.contains(".email") {
            let stripped_emails = line.rfind('=').map(|index| {
                let email = &line[index + 1..line.len() - 1];
                email.to_string()
            });

            match stripped_emails {
                Some(mut email) => {
                    let email = get_stripped_data(&mut email);
                    let email = email.trim();
                    let emails = email
                        .split(',')
                        .map(|email| email.to_string())
                        .collect::<Vec<String>>();
                    return emails;
                }
                None => return vec![],
            }
        }

        vec![]
    }

    #[allow(dead_code)]
    fn parse_author(&self, line: &str) -> Vec<String> {
        lazy_static! {
            static ref AUTHOR_REGEX1: Regex = Regex::new(r"/*.*author.*?=").unwrap();
            static ref AUTHOR_REGEX2: Regex = Regex::new(r"(\s*=>\s*)").unwrap();
        }

        if line.contains(".author") {
            let mut stripped_authors = AUTHOR_REGEX1.replace_all(line, "").to_string();
            let stripped_authors = get_stripped_data(&mut stripped_authors);
            let stripped_authors = AUTHOR_REGEX2
                .replace_all(&stripped_authors, "=>")
                .to_string();
            let stripped_authors = stripped_authors.trim();
            let stripped_authors = stripped_authors
                .split(',')
                .map(|author| author.to_string())
                .collect::<Vec<String>>();

            return stripped_authors;
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_bower_json() {
        let filepath = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/cocoapods/podspec/SwiftLib.podspec"
        ));

        let p = Spec::new();
        let result = p.parse_spec(filepath).unwrap();
        println!("{:#?}", result);
    }
}
