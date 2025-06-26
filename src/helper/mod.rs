use std::path::Path;

use ignore::{types::Types, DirEntry, Walk, WalkBuilder};
use tree_sitter::{Node, Query, QueryCapture, QueryCursor};

pub fn query_matches<'tree>(
    node: Node<'tree>,
    query: &Query,
    code_raw: &[u8],
) -> Vec<Vec<QueryCapture<'tree>>> {
    let mut cursor = QueryCursor::new();

    let mut ret = vec![];
    for m in cursor.matches(query, node, code_raw) {
        ret.push(m.captures.to_vec())
    }
    ret
}

pub fn build_walker<P: AsRef<Path>>(path: P, types: &[&str]) -> (Walk, Types) {
    let mut types_builder = ignore::types::TypesBuilder::new();
    types_builder.add_defaults();
    for ty in types {
        types_builder.select(ty);
    }
    let matcher = types_builder.build().unwrap();

    (
        WalkBuilder::new(path)
            .git_global(false)
            .parents(false)
            .types(matcher.clone())
            .build(),
        matcher,
    )
}

pub fn match_ftyp(entry: &DirEntry, matcher: &Types) -> Option<String> {
    let file_type = entry.file_type()?;

    if file_type.is_dir() {
        return None;
    }

    if let ignore::Match::Whitelist(x) = matcher.matched(entry.path(), false) {
        x.file_type_def().map(|def| def.name().to_owned())
    } else {
        None
    }
}

#[cfg(test)]
pub mod testing {
    use ignore::WalkBuilder;
    use std::path::{Path, PathBuf};

    /// Finds all files in the given directory that match the given extension.
    pub fn all_files_with_extensions<'a>(
        dir: &Path,
        extensions: &'a [&'a str],
    ) -> impl Iterator<Item = Result<PathBuf, ignore::Error>> + 'a {
        WalkBuilder::new(dir)
            .build()
            .filter_map(|result| match result {
                Ok(entry) => {
                    if entry.file_type().unwrap().is_file() {
                        let path = entry.path();

                        if let Some(ext) = path.extension() {
                            if extensions.contains(&ext.to_str().unwrap()) {
                                Some(Ok(path.to_path_buf()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            })
    }
}
