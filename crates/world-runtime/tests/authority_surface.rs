use std::{
    fs,
    path::{Path, PathBuf},
};

struct Rule {
    pattern: &'static str,
    allowed_paths: &'static [&'static str],
}

impl Rule {
    fn allows(&self, path: &str) -> bool {
        self.allowed_paths.contains(&path)
    }
}

const RULES: &[Rule] = &[
    Rule {
        pattern: "AcceptedHardCommit::new(",
        allowed_paths: &[
            "crates/world-model/src/tests.rs",
            "crates/world-runtime/src/tests/helpers.rs",
        ],
    },
    Rule {
        pattern: "AcceptedHardCommit::with_control_changes(",
        allowed_paths: &[
            "crates/world-model/src/tests.rs",
            "crates/world-runtime/src/transaction/commit.rs",
        ],
    },
    Rule {
        pattern: "AcceptedRuntimeControlUpdate::new(",
        allowed_paths: &[
            "crates/world-model/src/tests.rs",
            "crates/world-runtime/src/control/draft.rs",
        ],
    },
    Rule {
        pattern: ".apply_hard_commit(",
        allowed_paths: &[
            "crates/world-model/src/tests.rs",
            "crates/world-runtime/src/runtime.rs",
            "crates/world-runtime/src/scheduler/drain.rs",
            "crates/world-runtime/src/tests/helpers.rs",
        ],
    },
    Rule {
        pattern: ".apply_runtime_control_update(",
        allowed_paths: &[
            "crates/world-model/src/tests.rs",
            "crates/world-runtime/src/runtime.rs",
            "crates/world-runtime/src/scheduler/drain.rs",
        ],
    },
];

#[test]
fn accepted_package_authority_surface_stays_on_allowlist() {
    let workspace_root = workspace_root();
    let mut rust_files = Vec::new();
    for source_root in crate_source_roots(&workspace_root) {
        collect_rust_files(&source_root, &mut rust_files);
    }
    rust_files.sort();

    let mut violations = Vec::new();
    for file in rust_files {
        let relative = relative_path(&workspace_root, &file);
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"));
        let code = code_without_comments_or_strings(&source);

        for rule in RULES {
            if code.contains(rule.pattern) && !rule.allows(&relative) {
                violations.push(format!("{} contains `{}`", relative, rule.pattern));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "accepted-package authority surface drifted outside the allowlist:\n{}",
        violations.join("\n")
    );
}

#[test]
fn authority_surface_scan_ignores_comments_and_strings() {
    let source = r###"
        // AcceptedHardCommit::new(
        let _message = "AcceptedRuntimeControlUpdate::new(";
        let _raw = r#"WorldModel::apply_hard_commit("#;
        AcceptedRuntimeControlUpdate::new(header, changes, invalidation)
    "###;

    let code = code_without_comments_or_strings(source);

    assert!(!code.contains("AcceptedHardCommit::new("));
    assert!(!code.contains(".apply_hard_commit("));
    assert!(code.contains("AcceptedRuntimeControlUpdate::new("));
}

#[test]
fn authority_surface_scan_handles_nested_block_comments_and_raw_strings() {
    let source = r###"
        /* outer
            /* inner AcceptedRuntimeControlUpdate::new( */
            .apply_runtime_control_update(
        */
        let _raw = r##"AcceptedHardCommit::with_control_changes("##;
        model.apply_runtime_control_update(update)
    "###;

    let code = code_without_comments_or_strings(source);

    assert!(!code.contains("AcceptedHardCommit::with_control_changes("));
    assert_eq!(code.matches(".apply_runtime_control_update(").count(), 1);
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|error| {
            panic!("workspace root should be reachable from world-runtime: {error}")
        })
}

fn crate_source_roots(workspace_root: &Path) -> Vec<PathBuf> {
    let crates_dir = workspace_root.join("crates");
    let entries = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", crates_dir.display()));
    let mut roots = Vec::new();

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("failed to read entry in {}: {error}", crates_dir.display())
        });
        let path = entry.path().join("src");
        if path.is_dir() {
            roots.push(path);
        }
    }

    roots
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

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or_else(|error| {
            panic!(
                "{} should be under {}: {error}",
                path.display(),
                root.display()
            )
        })
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn code_without_comments_or_strings(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                mask_byte(&mut output, bytes[index]);
                mask_byte(&mut output, bytes[index + 1]);
                index += 2;
                while index < bytes.len() {
                    let byte = bytes[index];
                    mask_byte(&mut output, byte);
                    index += 1;
                    if byte == b'\n' {
                        break;
                    }
                }
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                mask_byte(&mut output, bytes[index]);
                mask_byte(&mut output, bytes[index + 1]);
                index += 2;
                let mut depth = 1;
                while index < bytes.len() && depth > 0 {
                    if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                        depth += 1;
                        mask_byte(&mut output, bytes[index]);
                        mask_byte(&mut output, bytes[index + 1]);
                        index += 2;
                    } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        depth -= 1;
                        mask_byte(&mut output, bytes[index]);
                        mask_byte(&mut output, bytes[index + 1]);
                        index += 2;
                    } else {
                        mask_byte(&mut output, bytes[index]);
                        index += 1;
                    }
                }
            }
            b'"' => {
                index = mask_string(bytes, index, &mut output);
            }
            b'r' => {
                if let Some(next) = mask_raw_string(bytes, index, &mut output) {
                    index = next;
                } else {
                    output.push(bytes[index] as char);
                    index += 1;
                }
            }
            _ => {
                output.push(bytes[index] as char);
                index += 1;
            }
        }
    }

    output
}

fn mask_string(bytes: &[u8], mut index: usize, output: &mut String) -> usize {
    mask_byte(output, bytes[index]);
    index += 1;
    let mut escaped = false;

    while index < bytes.len() {
        let byte = bytes[index];
        mask_byte(output, byte);
        index += 1;

        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            break;
        }
    }

    index
}

fn mask_raw_string(bytes: &[u8], index: usize, output: &mut String) -> Option<usize> {
    let mut delimiter = index + 1;
    while bytes.get(delimiter) == Some(&b'#') {
        delimiter += 1;
    }
    if bytes.get(delimiter) != Some(&b'"') {
        return None;
    }

    for byte in &bytes[index..=delimiter] {
        mask_byte(output, *byte);
    }
    let hashes = delimiter - index - 1;
    let mut cursor = delimiter + 1;

    while cursor < bytes.len() {
        let byte = bytes[cursor];
        mask_byte(output, byte);
        cursor += 1;

        if byte == b'"'
            && cursor + hashes <= bytes.len()
            && bytes[cursor..cursor + hashes]
                .iter()
                .all(|byte| *byte == b'#')
        {
            for _ in 0..hashes {
                mask_byte(output, b'#');
                cursor += 1;
            }
            break;
        }
    }

    Some(cursor)
}

fn mask_byte(output: &mut String, byte: u8) {
    if byte == b'\n' {
        output.push('\n');
    } else {
        output.push(' ');
    }
}
