use anyhow::bail;
use anyhow::Result as AnyhowResult;
use lazy_static::lazy_static;
use regex::Regex;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest};
use crate::pkgs::pyrequirements::PyRequirements;

use std::fs::File;
use std::io::Read;
use std::path::Path;

lazy_static! {
    static ref KEY_VALUE_REGEX: Regex = Regex::new("(?x)(?P<key>(.*))=(?P<value>(.*))").unwrap();
}

fn strip_comment(line: &str) -> &str {
    line.find(';').map(|i| &line[..i]).unwrap_or(line).trim()
}

fn parse_python_requirement(content: &str, key_name: &str) -> AnyhowResult<Vec<String>> {
    //configparser does not do multi line values
    //ini dies on them as well.
    //so we do our own poor man's parsing
    //debug!("Parsing {:?}", &setup_cfg_file);
    let raw = content;
    let mut res = Vec::new();
    match raw.find("[options]") {
        Some(options_start) => {
            let mut inside_value = false;
            let mut value_indention = 0;
            let mut value = "".to_string();
            for line in raw[options_start..].split('\n') {
                if !inside_value {
                    if line.contains(key_name) {
                        let wo_indent_len = (line.replace('\t', "    ").trim_start()).len();
                        value_indention = line.len() - wo_indent_len;
                        match line.find('=') {
                            Some(equal_pos) => {
                                let v = line[equal_pos + 1..].trim_end();
                                value += v;
                                value += "\n";
                                inside_value = true;
                            }
                            None => bail!("No = in install_requires line"),
                        }
                    }
                } else {
                    // inside value
                    let wo_indent_len = (line.replace('\t', "    ").trim_start()).len();
                    let indent = line.len() - wo_indent_len;
                    if indent > value_indention {
                        value += line.trim_start();
                        value += "\n"
                    } else {
                        break;
                    }
                }
            }
            for line in value.split('\n') {
                if !line.trim().is_empty() {
                    let line = strip_comment(line);
                    res.push(line.trim().to_string())
                }
            }
        }
        None => bail!("no [options] in setup.cfg"),
    };
    Ok(res)
}

fn parse_python_metadata(content: &str) -> AnyhowResult<Package> {
    let mut enter_metadata = false;
    let mut package = Package::default();
    for line in content.lines() {
        let line = strip_comment(line);
        if line.contains("[metadata]") {
            enter_metadata = true;
            continue;
        }
        if line.contains('[') {
            enter_metadata = false;
            continue;
        }

        if enter_metadata {
            let captures = match KEY_VALUE_REGEX.captures(line) {
                Some(captures) => captures,
                None => continue,
            };
            let key = captures
                .name("key")
                .map(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let value = captures
                .name("value")
                .map(|v| v.as_str())
                .unwrap_or("")
                .trim();

            if key == "name" {
                package.name = value.to_string();
            } else if key == "version" {
                package.version = value.to_string();
            }
        }
    }

    Ok(package)
}

pub struct PySetupCfg {}

impl PySetupCfg {
    pub fn new() -> Self {
        Self {}
    }

    fn parse(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut package = match parse_python_metadata(&content.clone()) {
            Ok(package) => package,
            Err(e) => {
                log::warn!("Failed to parse python metadata: {}", e);
                Package::default()
            }
        };

        let mut install_requires =
            match parse_python_requirement(&content.clone(), "install_requires") {
                Ok(install_requires) => {
                    PyRequirements::parse_requirement_content(&install_requires.join("\n"))
                        .unwrap_or_default()
                }
                Err(_) => vec![],
            };

        let mut setup_requires = match parse_python_requirement(&content.clone(), "setup_requires")
        {
            Ok(setup_requires) => {
                PyRequirements::parse_requirement_content(&setup_requires.join("\n"))
                    .unwrap_or_default()
            }
            Err(_) => vec![],
        };

        let mut test_requires = match parse_python_requirement(&content.clone(), "test_requires") {
            Ok(test_requires) => {
                PyRequirements::parse_requirement_content(&test_requires.join("\n"))
                    .unwrap_or_default()
            }
            Err(_) => vec![],
        };

        install_requires.iter_mut().for_each(|r| {
            r.is_runtime = true;
            r.is_optional = false;
            r.is_resolved = false;
            r.scope = "install_require".into();
        });

        setup_requires.iter_mut().for_each(|r| {
            r.is_runtime = false;
            r.is_optional = true;
            r.is_resolved = false;
            r.scope = "setup_require".into();
        });

        test_requires.iter_mut().for_each(|r| {
            r.is_runtime = false;
            r.is_optional = true;
            r.is_resolved = false;
            r.scope = "test_require".into();
        });

        install_requires.append(&mut setup_requires);
        install_requires.append(&mut test_requires);
        package.dependencies = install_requires;

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PySetupCfg {
    fn get_name(&self) -> String {
        "pypi".to_string()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["setup.cfg", "*setup.cfg"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_setup_cfg() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/pypi/unpacked_sdist/metadata-2.1/commoncode-21.5.12/setup.cfg"
        ));

        let p = PySetupCfg::parse(filepath).unwrap();
        println!("{:?}", p);
    }
}
