use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::complete::{line_ending, none_of, space0},
    combinator::recognize,
    error::VerboseError,
    multi::{many1, many_till},
    sequence::{delimited, tuple},
    IResult,
};
use packageurl::PackageUrl;

use crate::error::SourcePkgError;
use crate::pkgs::common::model::{DependentPackage, Package};

use std::fs::File;
use std::io::Read;
use std::path::Path;

type NomResult<T, U> = IResult<T, U, VerboseError<T>>;

fn take_till_line_end(input: &str) -> NomResult<&str, &str> {
    recognize(tuple((
        alt((take_until("\n"), take_until("\r\n"))),
        take(1usize),
    )))(input)
}

fn take_till_blank_line(input: &str) -> NomResult<&str, &str> {
    recognize(alt((take_until("\n\n"), take_until("\r\n\r\n"))))(input)
}

fn gem_header(input: &str) -> NomResult<&str, &str> {
    let (input, _) = recognize(take_until("GEM"))(input)?;
    recognize(tuple((tag("GEM"), line_ending)))(input)
}

fn specs(input: &str) -> NomResult<&str, &str> {
    let (input, _consumed) = recognize(many_till(
        take_till_line_end,
        recognize(tuple((space0, tag("specs:"), line_ending))),
    ))(input)?;

    take_till_blank_line(input)
}

fn package_name(input: &str) -> NomResult<&str, &str> {
    let (input, _) = recognize(space0)(input)?;
    recognize(take_until(" "))(input)
}

fn package_version(input: &str) -> NomResult<&str, &str> {
    let (input, _) = space0(input)?;
    delimited(tag("("), recognize(many1(none_of(" \t()"))), tag(")"))(input)
}

fn package(input: &str) -> Option<(String, String)> {
    let (input, name) = package_name(input).ok()?;
    let (_, version) = package_version(input).ok()?;

    Some((name.to_string(), version.to_string()))
}

fn parse(input: &str) -> Result<Vec<(String, String)>, SourcePkgError> {
    let (input, _) =
        gem_header(input).map_err(|e| SourcePkgError::GenericsError2(e.to_string()))?;

    let (_, consumed) = specs(input).map_err(|e| SourcePkgError::GenericsError2(e.to_string()))?;

    let pkgs = consumed.lines().filter_map(package).collect::<Vec<_>>();

    Ok(pkgs)
}

// NOTICE:
// DO NOT parse the 'unlocked' gemfile file
// because some version requirements are unable to resolve
// which maybe cause the false positive result

pub fn parse_file(path: impl AsRef<Path>) -> Result<Package, SourcePkgError> {
    let mut fs = File::open(path)?;
    let mut content = String::new();
    fs.read_to_string(&mut content)?;

    let pkgs = parse(&content)?;

    let mut dependencies = vec![];

    for (name, version) in pkgs {
        let dependency = DependentPackage {
            purl: PackageUrl::new("gem", name).unwrap().to_string(),
            requirement: version,
            is_resolved: true,
            ..Default::default()
        };

        dependencies.push(dependency);
    }

    let package = Package {
        dependencies,
        ..Default::default()
    };

    Ok(package)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_gemfile_lock() {
        let filepath = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/gemfile_lock/as_deps/Gemfile.lock"
        ));

        let p = parse_file(filepath).unwrap();
        println!("{:?}", p);
    }
}
