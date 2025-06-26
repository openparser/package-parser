use nom::error::VerboseError;
use nom::IResult;
use python_pkginfo::Error as PythonPkgError;
use quick_xml::de::DeError;
use serde_json::Error as JsonError;
use serde_yaml::Error as YamlError;
use thiserror::Error;

pub type NomResult<T, U> = IResult<T, U, VerboseError<T>>;

#[derive(Debug, Error)]
pub enum SourcePkgError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonParse(#[from] JsonError),

    #[error("Toml deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("Cargo lock error: {0}")]
    CargoLockParse(#[from] cargo_lock::Error),

    #[error("Opam error: {0}")]
    OpamParse(#[from] opam_file_rs::OpamFileError),

    #[error("XML error: {0}")]
    XmlParse(#[from] DeError),

    #[error("XML error: {0}")]
    XmlError(#[from] quick_xml::Error),

    #[error("YAML error: {0}")]
    YamlParse(#[from] YamlError),

    #[error("UTF-8 decode error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Yarn lock error: {0}")]
    YarnParse(#[from] yarn_lock_parser::YarnLockError),

    #[error("Python pkginfo error: {0}")]
    PythonPkgParse(#[from] PythonPkgError),

    #[error("Invalid UTF-8: {0}")]
    Utf8Error2(#[from] std::str::Utf8Error),

    #[error("{0}")]
    GenericsError(&'static str),

    #[error("{0}")]
    GenericsError2(String),

    #[error("{0}")]
    GenericsError3(#[from] anyhow::Error),

    #[error("Not supported")]
    NotSupported,

    #[error("Skipped")]
    Skipped,
}
