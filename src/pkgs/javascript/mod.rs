use packageurl::PackageUrl;

pub mod manifest;
mod npm_lock;
mod yarn_lock;
mod pnpm;
mod reachability;

fn split_name(name: &str) -> (Option<&str>, &str) {
    if let Some((ns, name)) = name.split_once('/') {
        (Some(ns), name)
    } else {
        (None, name)
    }
}

fn make_purl(full_name: &str, version: &str) -> String {
    let (ns, name) = split_name(full_name);
    let mut purl = PackageUrl::new("npm", name).unwrap();
    purl.with_version(version);
    if let Some(ns) = ns {
        purl.with_namespace(ns);
    }
    purl.to_string()
}
