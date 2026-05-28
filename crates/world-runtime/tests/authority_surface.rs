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
        let analysis = SourceAnalysis::new(&code);

        for rule in RULES {
            if rule_matches(rule.pattern, &analysis) && !rule.allows(&relative) {
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
    let analysis = SourceAnalysis::new(&code);

    assert!(!rule_matches("AcceptedHardCommit::new(", &analysis));
    assert!(!rule_matches(".apply_hard_commit(", &analysis));
    assert!(rule_matches(
        "AcceptedRuntimeControlUpdate::new(",
        &analysis
    ));
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
    let analysis = SourceAnalysis::new(&code);

    assert!(!rule_matches(
        "AcceptedHardCommit::with_control_changes(",
        &analysis
    ));
    assert!(rule_matches(".apply_runtime_control_update(", &analysis));
}

#[test]
fn authority_surface_scan_normalizes_whitespace_ufcs_and_aliases() {
    let source = r#"
        use world_model::AcceptedRuntimeControlUpdate as ControlUpdate;
        AcceptedHardCommit :: new (transaction, events, changes, invalidation);
        ControlUpdate::new(header, changes, invalidation);
        WorldModel::apply_hard_commit(&mut model, commit);
    "#;
    let code = code_without_comments_or_strings(source);
    let analysis = SourceAnalysis::new(&code);

    assert!(rule_matches("AcceptedHardCommit::new(", &analysis));
    assert!(rule_matches(
        "AcceptedRuntimeControlUpdate::new(",
        &analysis
    ));
    assert!(rule_matches(".apply_hard_commit(", &analysis));
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

struct SourceAnalysis {
    code: String,
    normalized: String,
}

impl SourceAnalysis {
    fn new(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            normalized: normalize_tokens(code),
        }
    }
}

fn rule_matches(pattern: &str, analysis: &SourceAnalysis) -> bool {
    let normalized_pattern = normalize_tokens(pattern);
    if analysis.normalized.contains(&normalized_pattern) {
        return true;
    }

    match pattern {
        "AcceptedHardCommit::new(" => simple_aliases(&analysis.code, "AcceptedHardCommit")
            .iter()
            .any(|alias| analysis.normalized.contains(&format!("{alias}::new("))),
        "AcceptedHardCommit::with_control_changes(" => {
            simple_aliases(&analysis.code, "AcceptedHardCommit")
                .iter()
                .any(|alias| {
                    analysis
                        .normalized
                        .contains(&format!("{alias}::with_control_changes("))
                })
        }
        "AcceptedRuntimeControlUpdate::new(" => {
            simple_aliases(&analysis.code, "AcceptedRuntimeControlUpdate")
                .iter()
                .any(|alias| analysis.normalized.contains(&format!("{alias}::new(")))
        }
        ".apply_hard_commit(" => {
            analysis
                .normalized
                .contains("WorldModel::apply_hard_commit(")
                || simple_aliases(&analysis.code, "WorldModel")
                    .iter()
                    .any(|alias| {
                        analysis
                            .normalized
                            .contains(&format!("{alias}::apply_hard_commit("))
                    })
        }
        ".apply_runtime_control_update(" => {
            analysis
                .normalized
                .contains("WorldModel::apply_runtime_control_update(")
                || simple_aliases(&analysis.code, "WorldModel")
                    .iter()
                    .any(|alias| {
                        analysis
                            .normalized
                            .contains(&format!("{alias}::apply_runtime_control_update("))
                    })
        }
        _ => false,
    }
}

fn normalize_tokens(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn simple_aliases(source: &str, canonical: &str) -> Vec<String> {
    let needle = format!("{canonical} as ");
    let mut aliases = source
        .lines()
        .filter_map(|line| line.split_once(&needle).map(|(_, alias)| alias))
        .filter_map(|alias| take_identifier(alias.trim()))
        .collect::<Vec<_>>();

    let type_target = format!("= {canonical}");
    aliases.extend(source.lines().filter_map(|line| {
        let line = line.trim();
        let alias = line.strip_prefix("type ")?.split_once(&type_target)?.0;
        take_identifier(alias.trim())
    }));
    aliases
}

fn take_identifier(source: &str) -> Option<String> {
    let identifier = source
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    (!identifier.is_empty()).then_some(identifier)
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
