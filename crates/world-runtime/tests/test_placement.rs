use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn production_modules_do_not_contain_inline_test_modules() {
    let workspace = workspace_root();
    let crates_dir = workspace.join("crates");
    let mut rust_files = Vec::new();
    collect_rust_files(&crates_dir, &mut rust_files);
    rust_files.sort();

    let mut violations = Vec::new();
    for file in rust_files {
        if is_test_source(&file) {
            continue;
        }

        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        if contains_inline_test_module(&source) {
            violations.push(
                file.strip_prefix(&workspace)
                    .unwrap_or_else(|error| {
                        panic!(
                            "{} should be under {}: {error}",
                            file.display(),
                            workspace.display()
                        )
                    })
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        violations.is_empty(),
        "production modules must not contain inline test modules:\n{}",
        violations.join("\n")
    );
}

#[test]
fn inline_test_module_detector_allows_external_test_module_declarations() {
    assert!(!contains_inline_test_module(
        r#"
        #[cfg(test)]
        mod tests;
        "#
    ));
}

#[test]
fn inline_test_module_detector_flags_inline_modules() {
    assert!(contains_inline_test_module(
        r#"
        #[cfg(test)]
        mod tests {
            #[test]
            fn behavior_is_visible() {}
        }
        "#
    ));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or_else(|| {
            panic!(
                "workspace root should be reachable from {}",
                env!("CARGO_MANIFEST_DIR")
            )
        })
        .to_path_buf()
}

fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()));

    for entry in entries {
        let entry = entry
            .unwrap_or_else(|error| panic!("failed to read entry in {}: {error}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn is_test_source(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "tests.rs")
        || path
            .components()
            .any(|component| component.as_os_str() == "tests")
}

fn contains_inline_test_module(source: &str) -> bool {
    normalize_tokens(source).contains("#[cfg(test)]modtests{")
}

fn normalize_tokens(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
