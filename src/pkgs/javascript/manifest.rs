use maplit::hashset;
use packageurl::PackageUrl;
use crate::types::{Reachability, Relation};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::SourcePkgError;
use crate::helper::{build_walker, match_ftyp};
use crate::pkgs::common::model::{Package, PackageManifest, RecognizeContext};
use crate::DependentPackage;

use super::npm_lock::NpmLock;
use super::yarn_lock::YarnLock;
use super::{pnpm, reachability};

pub type DepsSet = HashMap<String, String>;

fn walk_reachability(path: &Path) -> HashSet<String> {
    let (walk, matcher) = build_walker(path, &["js", "ts"]);

    let mut ret = HashSet::new();

    for entry in walk {
        let entry = if let Ok(entry) = entry {
            entry
        } else {
            log::error!("Failed to walk entry: {:?}", entry);
            continue;
        };

        let ftyp = if let Some(t) = match_ftyp(&entry, &matcher) {
            t
        } else {
            continue;
        };

        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to read {}: {}", entry.path().display(), e);
                continue;
            }
        };

        let res = match ftyp.as_str() {
            "js" => reachability::process_js(&content),
            "ts" => {
                if let Some(ext) = entry.path().extension() {
                    if ext.eq_ignore_ascii_case("tsx") {
                        reachability::process_tsx(&content)
                    } else {
                        reachability::process_ts(&content)
                    }
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        let purls = match res {
            Ok(purls) => purls,
            Err(e) => {
                log::error!("Failed to parse {}: {}", entry.path().display(), e);
                continue;
            }
        };

        ret.extend(purls);
    }

    ret
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NpmManifest {
    /// The package name.
    pub name: Option<String>,
    /// The package version.
    pub version: Option<String>,
    /// The optional list of dependencies.
    #[serde(default)]
    pub dependencies: DepsSet,
    /// The optional list of development dependencies.
    #[serde(default)]
    pub dev_dependencies: DepsSet,
    /// The optional list of optional dependencies.
    #[serde(default)]
    pub optional_dependencies: DepsSet,
}

pub struct PackageJson {
    lock_scanner: NpmLock,
    yarn_scanner: YarnLock,
}

impl PackageJson {
    pub fn new() -> Self {
        Self {
            lock_scanner: NpmLock::new(),
            yarn_scanner: YarnLock::new(),
        }
    }

    fn recognize_fallback(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let fs = std::fs::File::open(path).map_err(SourcePkgError::Io)?;

        let package_data: NpmManifest =
            serde_json::from_reader(fs).map_err(SourcePkgError::JsonParse)?;

        let convert = |scope: &str, is_dev: bool, is_optional: bool| {
            let scope = scope.to_string();

            move |(name, version): (String, String)| DependentPackage {
                purl: PackageUrl::new("npm", name)
                    .expect("purl arguments are invalid")
                    .to_string(),
                scope: scope.clone(),
                requirement: version,
                is_runtime: !is_dev,
                is_optional,
                is_resolved: false,
                parents: Default::default(),
                relation: hashset! {Relation::Direct},
                reachable: Default::default(),
            }
        };

        let dependencies = package_data
            .dependencies
            .into_iter()
            .map(convert("prod", false, false));
        let dev_dependencies = package_data
            .dev_dependencies
            .into_iter()
            .map(convert("dev", true, true));
        let optional_dependencies = package_data
            .optional_dependencies
            .into_iter()
            .map(convert("prod", false, true));

        let pkg = Package {
            // FIXME: having no name may not be a problem See #1514
            name: package_data.name.unwrap_or_default(),
            version: package_data.version.unwrap_or_default(),
            dependencies: dependencies
                .chain(dev_dependencies)
                .chain(optional_dependencies)
                .collect(),
            ..Default::default()
        };

        Ok(pkg)
    }

    async fn recognize_internal(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let path_dir = path.parent().unwrap();

        {
            let lock_path = path_dir.join("yarn.lock");
            if tokio::fs::metadata(&lock_path).await.is_ok() {
                log::info!("Found yarn.lock file, using yarn scanner");
                match self.yarn_scanner.recognize(&lock_path).await {
                    Ok(x) => return Ok(x),
                    Err(e) => {
                        log::warn!("Failed to recognize yarn.lock: {}", e);
                    }
                }
            }
        }

        {
            let lock_path = path_dir.join("package-lock.json");
            if tokio::fs::metadata(&lock_path).await.is_ok() {
                log::info!("Found package-lock.json file, using npm scanner");
                match self.lock_scanner.recognize(&lock_path).await {
                    Ok(x) => return Ok(x),
                    Err(e) => {
                        log::warn!("Failed to recognize package-lock.json: {}", e);
                    }
                }
            }
        }

        {
            let lock_path = path_dir.join("pnpm-lock.yaml");
            if tokio::fs::metadata(&lock_path).await.is_ok() {
                log::info!("Found pnpm-lock.yaml file, using it to provide resolved versions");
                match pnpm::parse(&lock_path) {
                    Ok(x) => return Ok(x),
                    Err(e) => {
                        log::warn!("Failed to recognize pnpm-lock.yaml: {}", e);
                    }
                }
            }
        }

        self.recognize_fallback(path)
    }

    async fn recognize_with_reachability(&self, path: &Path) -> Result<Package, SourcePkgError> {
        let mut res = self.recognize_internal(path).await?;

        if res.dependencies.is_empty() {
            return Ok(res);
        }

        let parent = if let Some(parent) = path.parent() {
            parent.to_path_buf()
        } else {
            return Ok(res);
        };

        let purls = tokio::task::spawn_blocking(move || walk_reachability(&parent))
            .await
            .unwrap();

        log::debug!("Reachable purls: {:?}", purls);
        
        for dep in res.dependencies.iter_mut() {
            if dep.is_runtime && dep.relation.contains(&Relation::Direct) {
                let purl_without_version = dep.purl.split('@').next().unwrap();

                if !purls.contains(purl_without_version) {
                    dep.reachable = Reachability::No;
                }
            }
        }

        Ok(res)
    }
}

#[async_trait::async_trait]
impl PackageManifest for PackageJson {
    fn get_name(&self) -> String {
        "npm".into()
    }

    fn get_identifier(&self) -> String {
        "npm-manifest".into()
    }

    async fn recognize_with_config(
        &self,
        path: &Path,
        _context: &RecognizeContext,
    ) -> Result<Package, SourcePkgError> {
        self.recognize_with_reachability(path).await
    }

    async fn recognize(&self, path: &Path) -> Result<Package, SourcePkgError> {
        self.recognize_with_config(path, &RecognizeContext::default())
            .await
    }

    fn file_name_patterns(&self) -> &'static [&'static str] {
        &["package.json", "bower.json"]
    }
}
