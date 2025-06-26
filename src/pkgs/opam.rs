use opam_file_rs::parse as opam_file_parse;
use opam_file_rs::value::{OpamFileItem, OpamFileSection, RelOp, RelOpKind, Value, ValueKind};
use packageurl::PackageUrl;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{Package, PackageManifest, Party};

use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::common::model::DependentPackage;

pub struct OcamlOpam {}

#[derive(Debug)]
struct OcamlOpamFile {
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub maintainers: Vec<Party>,
    pub dev_repo: Option<String>,
    pub license: Option<String>,
    pub dependencies: Vec<DependentPackage>,
}

impl OcamlOpamFile {
    fn new() -> Self {
        Self {
            homepage: None,
            description: None,
            maintainers: vec![],
            dev_repo: None,
            license: None,
            dependencies: vec![],
        }
    }
}

impl OcamlOpam {
    pub fn new() -> Self {
        Self {}
    }

    fn parse_opam_description(value: Value) -> Option<String> {
        match value.kind {
            ValueKind::String(value) => Some(value),
            _ => {
                log::error!("unexpected value kind : {:?}", value.kind);
                None
            }
        }
    }

    fn parse_opam_maintainer(value: Value) -> Vec<Party> {
        match value.kind {
            ValueKind::List(values) => {
                let mut maintainers = Vec::new();
                for maintainer in values {
                    match maintainer.kind {
                        ValueKind::String(maintainer) => {
                            maintainers.push(Party {
                                typ: "".into(),
                                name: "".into(),
                                email: maintainer.to_string(),
                                url: "".into(),
                                ..Default::default()
                            });
                        }
                        _ => {
                            log::error!("unexpected value kind : {:?}", maintainer.kind);
                        }
                    }
                }

                maintainers
            }
            _ => {
                log::error!("unexpected value kind : {:?}", value.kind);
                vec![]
            }
        }
    }

    fn parse_opam_homepage(value: Value) -> Option<String> {
        match value.kind {
            ValueKind::String(value) => Some(value),
            _ => {
                log::error!("unexpected value kind : {:?}", value.kind);
                None
            }
        }
    }

    fn parse_opam_devrepo(value: Value) -> Option<String> {
        match value.kind {
            ValueKind::String(value) => Some(value),
            _ => {
                log::error!("unexpected value kind : {:?}", value.kind);
                None
            }
        }
    }

    fn parse_opam_license(value: Value) -> Option<String> {
        match value.kind {
            ValueKind::String(value) => Some(value),
            _ => {
                log::error!("unexpected value kind : {:?}", value.kind);
                None
            }
        }
    }

    fn opam_relop_to_string(op: RelOp) -> String {
        match op.kind {
            RelOpKind::Eq => "=".into(),
            RelOpKind::Neq => "!=".into(),
            RelOpKind::Lt => "<".into(),
            RelOpKind::Leq => "<=".into(),
            RelOpKind::Gt => ">".into(),
            RelOpKind::Geq => ">=".into(),
            RelOpKind::Sem => "~".into(),
        }
    }

    fn parse_opam_depend_option_item(name: Value, props: Vec<Value>) -> Option<DependentPackage> {
        match name.kind {
            ValueKind::String(name) => {
                let mut version = None;
                let mut relop_string = None;
                for prop in props {
                    match prop.kind {
                        ValueKind::PrefixRelOp(rel_op, version_value) => match version_value.kind {
                            ValueKind::String(version_value) => {
                                version = Some(version_value);
                                relop_string = Some(Self::opam_relop_to_string(rel_op));
                            }
                            _ => {
                                log::error!("unexpected value kind : {:?}", version_value.kind);
                            }
                        },
                        _ => {
                            log::error!("unexpected value kind : {:?}", prop.kind);
                        }
                    }
                }
                return Some(DependentPackage {
                    purl: PackageUrl::new("opam", name.as_str())
                        .expect("purl arguments are invalid")
                        .to_string(),
                    requirement: format!(
                        "{}{}",
                        relop_string.unwrap_or_default(),
                        version.unwrap_or_default()
                    )
                    .trim()
                    .to_string(),
                    ..Default::default()
                });
            }
            _ => {
                log::error!("unexpected value kind : {:?}", name.kind);
            }
        }

        None
    }

    fn parse_opam_depends(value: Value) -> Vec<DependentPackage> {
        // depends : Value {
        //     kind: List(
        //         [
        //             Value {
        //                 kind: Option(
        //                     Value {
        //                         kind: String(
        //                             "ocaml",
        //                         ),
        //                     },
        //                     [
        //                         Value {
        //                             kind: PrefixRelOp(
        //                                 RelOp {
        //                                     kind: Geq,
        //                                 },
        //                                 Value {
        //                                     kind: String(
        //                                         "4.06.0",
        //                                     ),
        //                                 },
        //                             ),
        //                         },
        //                     ],
        //                 ),
        //             },

        match value.kind {
            ValueKind::List(values) => {
                let mut dependencies = Vec::new();
                for dependency in values {
                    match dependency.kind {
                        ValueKind::Option(name, props) => {
                            let dep = Self::parse_opam_depend_option_item(*name, props);
                            if let Some(dep) = dep {
                                dependencies.push(dep);
                            }
                        }
                        _ => {
                            log::error!("unexpected value kind : {:?}", dependency.kind);
                        }
                    }
                }

                dependencies
            }
            _ => {
                log::error!("unexpected value kind : {:?}", value.kind);
                vec![]
            }
        }
    }

    fn parse_opam_section(_section: OpamFileSection) {}

    fn parse_opam_variable(opam_file: &mut OcamlOpamFile, section_name: String, value: Value) {
        if section_name == "description" {
            opam_file.description = Self::parse_opam_description(value);
        } else if section_name == "maintainer" {
            opam_file.maintainers = Self::parse_opam_maintainer(value);
        } else if section_name == "homepage" {
            opam_file.homepage = Self::parse_opam_homepage(value);
        } else if section_name == "dev-repo" {
            opam_file.dev_repo = Self::parse_opam_devrepo(value);
        } else if section_name == "license" {
            opam_file.license = Self::parse_opam_license(value);
        } else if section_name == "depends" {
            opam_file.dependencies = Self::parse_opam_depends(value);
        } else if section_name == "authors" {
            opam_file.maintainers = Self::parse_opam_maintainer(value);
        }
    }

    fn parse_ocaml_opam(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
        let mut fs = File::open(path)?;
        let mut content = String::new();
        fs.read_to_string(&mut content)?;
        let mut opam_file = OcamlOpamFile::new();
        let opam = opam_file_parse(&content)?;
        for opam_item in opam.file_contents {
            match opam_item {
                OpamFileItem::Section(_, section) => {
                    Self::parse_opam_section(section);
                }
                OpamFileItem::Variable(_, string_value, value) => {
                    Self::parse_opam_variable(&mut opam_file, string_value, value);
                }
            }
        }

        let package = Package {
            declared_license: opam_file.license.unwrap_or_default(),
            dependencies: opam_file.dependencies,
            ..Default::default()
        };

        Ok(package)
    }
}

#[async_trait::async_trait]
impl PackageManifest for OcamlOpam {
    fn get_name(&self) -> String {
        "opam".into()
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        Self::parse_ocaml_opam(path)
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["*.opam"]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_opam() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/opam/sample1/sample1.opam"
        ));

        OcamlOpam::parse_ocaml_opam(filepath).unwrap();
    }
}
