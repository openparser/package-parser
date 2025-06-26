# package-parser

A fast and extensible tool written in Rust for extracting dependency and metadata information from a wide range of programming language package managers. Ideal for Software Composition Analysis (SCA), and more.

## ✨ Features

- ⚡ High performance, memory-safe, and parallelizable (thanks to Rust)
- 🌍 Supports dozens of ecosystems and formats
- 🧩 Easily embeddable as a Rust library
- 🛠️ Suitable for SCA, SBOM generation, and reachability analysis

## 📦 Supported Ecosystems

The following package managers and ecosystems are supported:

- **Rust**: `Cargo.toml`, `Cargo.lock`
- **Python**: `requirements.txt`, `pyproject.toml`, `setup.py`, `Pipfile.lock`, `conda`
- **JavaScript/Node.js**: `package.json`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`
- **Java**: `pom.xml` (Maven), `build.gradle`, `gradle.lockfile`
- **.NET**: `*.csproj`, `.nuspec`, `nuget.config`
- **Ruby**: `Gemfile`, `Gemfile.lock`
- **PHP**: `composer.json`
- **Dart**: `pubspec.yaml`, `pubspec.lock`
- **R**: `renv.lock`, CRAN
- **Swift**: `Podfile.lock` (CocoaPods)
- **C/C++**: `conanfile.txt`, `conan.lock`
- **Haskell/OCaml/Elm/Fortran/Haxe**: experimental support
- and more...

> Ecosystem support is modular — each format has its own parser under `src/pkgs`.


