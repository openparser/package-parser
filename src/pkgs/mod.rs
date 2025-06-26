use std::sync::Arc;

pub use self::common::model::{PackageManifest, RecognizeContext};

pub mod cargo;
pub mod chef;
pub mod chef_json;
pub mod cocoapods;
pub mod common;
pub mod composer;
pub mod conan;
pub mod cran;
pub mod dart;
pub mod dotnet;
pub mod elm;
pub mod fortran;
pub mod haxe;
pub mod java;
pub mod javascript;
pub mod opam;
pub mod python;
pub mod renv;
pub mod ruby;
pub mod rubygems;
pub mod spec;
pub mod swift;

use java::*;
use python::*;

fn wrap_scanner<S>(scanner: S) -> Arc<dyn PackageManifest + Send + Sync>
where
    S: PackageManifest + Send + Sync + 'static,
{
    Arc::new(scanner)
}

pub fn create_scanners() -> Vec<Arc<dyn PackageManifest + Send + Sync>> {
    vec![
        wrap_scanner(javascript::manifest::PackageJson::new()),
        wrap_scanner(cargo::CargoToml::new()),
        wrap_scanner(cargo::CargoLock::new()),
        wrap_scanner(chef::Chef::new()),
        wrap_scanner(cocoapods::CocoaPods::new()),
        wrap_scanner(composer::PhpComposer::new()),
        wrap_scanner(conan::ConanLock::new()),
        wrap_scanner(cran::Cran::new()),
        wrap_scanner(dotnet::csproj::CSharpCsproj::new()),
        wrap_scanner(elm::ElmJson::new()),
        wrap_scanner(fortran::FpmToml::new()),
        wrap_scanner(ruby::gemfile::Gemfile::new()),
        wrap_scanner(gradlelock::GradleLock::new()),
        wrap_scanner(gradle_dependency::GradleDependencies::new()),
        wrap_scanner(haxe::Haxe::new()),
        wrap_scanner(maven::JavaMavenPom::new()),
        wrap_scanner(dotnet::nuspec::DotnetNuSpec::new()),
        wrap_scanner(dotnet::nuget_central::NuGetCentral::new()),
        wrap_scanner(opam::OcamlOpam::new()),
        wrap_scanner(pipfile::Pipfile::new()),
        wrap_scanner(pipfilelock::Pipfilelock::new()),
        wrap_scanner(renv::RenvLock::new()),
        wrap_scanner(dart::pubspec::PubSpec::new()),
        wrap_scanner(pyconda::PyConda::new()),
        wrap_scanner(pymetadata::PyMetadata::new()),
        wrap_scanner(pyrequirements::PyRequirements::new()),
        wrap_scanner(pysetup_cfg::PySetupCfg::new()),
        wrap_scanner(pyproject::PyProject::new()),
        wrap_scanner(pysetup::PySetup::new()),
        wrap_scanner(rubygems::RubyGems::new()),
        wrap_scanner(swift::SwiftPmLock::new()),
    ]
}
