use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn decision_source_does_not_import_privileged_crates() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir.join("src");
    let mut rust_files = Vec::new();
    collect_rust_files(&source_root, &mut rust_files);
    rust_files.sort();

    let forbidden = ["world_model", "world_runtime", "world_standard_runtime"];
    let mut violations = Vec::new();

    for file in rust_files {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        let code = code_without_comments_or_strings(&source);
        let normalized = normalize_tokens(&code);

        for pattern in forbidden {
            if normalized.contains(pattern) {
                violations.push(format!(
                    "{} imports privileged crate `{pattern}`",
                    file.strip_prefix(manifest_dir)
                        .unwrap_or_else(|error| panic!(
                            "{} should be under {}: {error}",
                            file.display(),
                            manifest_dir.display()
                        ))
                        .display()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "decision substrate must stay behind actor-context and checked-declaration boundaries:\n{}",
        violations.join("\n")
    );
}

#[test]
fn source_scanner_ignores_comments_and_strings_but_catches_code() {
    let source = r##"
        // use world_model::WorldModel;
        const TEXT: &str = "world_runtime::CausalRuntime";
        const RAW: &str = r#"world_standard_runtime"#;
        use world_model :: WorldModel;
    "##;

    let masked = code_without_comments_or_strings(source);
    let normalized = normalize_tokens(&masked);

    assert!(!masked.contains("world_runtime"));
    assert!(!masked.contains("world_standard_runtime"));
    assert!(normalized.contains("world_model::WorldModel"));
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

fn normalize_tokens(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
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
    let mut cursor = index + 1;
    let mut hashes = 0;

    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }

    for byte in &bytes[index..=cursor] {
        mask_byte(output, *byte);
    }
    cursor += 1;

    while cursor < bytes.len() {
        let byte = bytes[cursor];
        mask_byte(output, byte);
        cursor += 1;

        if byte == b'"' && raw_string_hashes_match(bytes, cursor, hashes) {
            for _ in 0..hashes {
                mask_byte(output, bytes[cursor]);
                cursor += 1;
            }
            break;
        }
    }

    Some(cursor)
}

fn raw_string_hashes_match(bytes: &[u8], cursor: usize, hashes: usize) -> bool {
    (0..hashes).all(|offset| bytes.get(cursor + offset) == Some(&b'#'))
}

fn mask_byte(output: &mut String, byte: u8) {
    if byte == b'\n' {
        output.push('\n');
    } else {
        output.push(' ');
    }
}
