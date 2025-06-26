pub mod pipfile;
pub mod pipfilelock;
pub mod poetrylock;
pub mod pyconda;
pub mod pymetadata;
pub mod pyproject;
pub mod pyrequirements;
pub mod pysetup;
pub mod pysetup_cfg;

lazy_static::lazy_static! {
    static ref NORMALIZE_PATTERN: regex::Regex = regex::Regex::new(r"[-_.]+").unwrap();
}

pub fn normalize_name(name: &str) -> String {
    NORMALIZE_PATTERN.replace_all(name, "-").into_owned()
}
