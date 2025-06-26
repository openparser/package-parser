use std::path::Path;

use crate::types::Package;
use serde::Deserialize;

use crate::error::SourcePkgError;

mod lock_v6;
mod lock_v9;

#[derive(Deserialize)]
#[serde(tag = "lockfileVersion")]
enum PnpmLock {
    #[serde(rename = "6.0")]
    V6(lock_v6::PnpmLockV6),
    #[serde(rename = "7.0")]
    V7(lock_v9::PnpmLockV9),
    #[serde(rename = "9.0")]
    V9(lock_v9::PnpmLockV9),
    #[serde(other)]
    Unsupported,
}

pub fn parse(path: &Path) -> Result<Package, SourcePkgError> {
    let reader = std::fs::File::open(path).map_err(SourcePkgError::Io)?;

    let lock: PnpmLock = serde_yaml::from_reader(reader).map_err(SourcePkgError::YamlParse)?;

    match lock {
        PnpmLock::V6(l) => {
            let deps = l.process()?;

            Ok(Package {
                dependencies: deps,
                ..Default::default()
            })
        }
        PnpmLock::V7(l) | PnpmLock::V9(l) => {
            let deps = l.process()?;

            Ok(Package {
                dependencies: deps,
                ..Default::default()
            })
        }
        PnpmLock::Unsupported => Err(SourcePkgError::GenericsError("Unsupported lockfileVersion")),
    }
}
