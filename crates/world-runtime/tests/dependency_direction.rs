use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const FORBIDDEN_EDGES: &[(&str, &[&str])] = &[
    (
        "world-defs",
        &[
            "world-model",
            "world-runtime",
            "world-standard",
            "world-standard-runtime",
            "world-context",
            "world-decision",
        ],
    ),
    (
        "world-model",
        &[
            "world-runtime",
            "world-standard",
            "world-standard-runtime",
            "world-context",
            "world-decision",
        ],
    ),
    (
        "world-runtime",
        &[
            "world-standard",
            "world-standard-runtime",
            "world-context",
            "world-decision",
        ],
    ),
    (
        "world-standard",
        &[
            "world-model",
            "world-runtime",
            "world-standard-runtime",
            "world-context",
            "world-decision",
        ],
    ),
    (
        "world-standard-runtime",
        &["world-context", "world-decision"],
    ),
    (
        "world-context",
        &["world-runtime", "world-standard-runtime"],
    ),
    (
        "world-decision",
        &["world-runtime", "world-standard-runtime"],
    ),
    (
        "world-authoring",
        &[
            "world-model",
            "world-runtime",
            "world-standard-runtime",
            "world-engine",
        ],
    ),
];

#[test]
fn crate_dependency_direction_matches_architecture() {
    let root = workspace_root();
    let manifests = crate_manifests(&root);
    let mut violations = Vec::new();

    for (source, forbidden_targets) in FORBIDDEN_EDGES {
        let Some(dependencies) = manifests.get(*source) else {
            violations.push(format!("{source} manifest is missing"));
            continue;
        };
        for target in *forbidden_targets {
            if dependencies.contains(*target) {
                violations.push(format!("{source} must not depend on {target}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "crate dependency direction drifted:\n{}",
        violations.join("\n")
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|error| {
            panic!("workspace root should be reachable from world-runtime: {error}")
        })
}

fn crate_manifests(root: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let crates_dir = root.join("crates");
    let entries = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", crates_dir.display()));
    let mut manifests = BTreeMap::new();

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("failed to read entry in {}: {error}", crates_dir.display())
        });
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let package = entry.file_name().to_string_lossy().into_owned();
        manifests.insert(package, manifest_dependencies(&manifest));
    }

    manifests
}

fn manifest_dependencies(manifest: &Path) -> BTreeSet<String> {
    let source = fs::read_to_string(manifest)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest.display()));
    manifest_dependencies_from_str(&source)
}

fn manifest_dependencies_from_str(source: &str) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    let mut section = ManifestSection::Other;

    for line in source.lines() {
        let line = strip_toml_comment(line).trim();
        if line.starts_with('[') && line.ends_with(']') {
            let header = line.trim_start_matches('[').trim_end_matches(']');
            section = match dependency_table_name(header) {
                Some(name) => {
                    dependencies.insert(name.clone());
                    ManifestSection::DependencyTable
                }
                None if is_dependency_section(header) => ManifestSection::DependencySection,
                None => ManifestSection::Other,
            };
            continue;
        }
        if line.is_empty() {
            continue;
        }

        match &section {
            ManifestSection::DependencySection => {
                let Some((name, value)) = line.split_once('=') else {
                    continue;
                };
                dependencies.insert(unquote_key(name.trim()));
                if let Some(package) = inline_package_name(value) {
                    dependencies.insert(package);
                }
            }
            ManifestSection::DependencyTable => {
                let Some((name, value)) = line.split_once('=') else {
                    continue;
                };
                if name.trim() == "package"
                    && let Some(package) = quoted_value(value)
                {
                    dependencies.insert(package);
                }
            }
            ManifestSection::Other => {}
        };
    }

    dependencies
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ManifestSection {
    Other,
    DependencySection,
    DependencyTable,
}

fn is_dependency_section(header: &str) -> bool {
    matches!(
        header,
        "dependencies" | "dev-dependencies" | "build-dependencies"
    ) || (header.starts_with("target.")
        && (header.ends_with(".dependencies")
            || header.ends_with(".dev-dependencies")
            || header.ends_with(".build-dependencies")))
}

fn dependency_table_name(header: &str) -> Option<String> {
    dependency_table_suffix(header, "dependencies.")
        .or_else(|| dependency_table_suffix(header, "dev-dependencies."))
        .or_else(|| dependency_table_suffix(header, "build-dependencies."))
        .or_else(|| target_dependency_table_suffix(header, ".dependencies."))
        .or_else(|| target_dependency_table_suffix(header, ".dev-dependencies."))
        .or_else(|| target_dependency_table_suffix(header, ".build-dependencies."))
}

fn dependency_table_suffix(header: &str, prefix: &str) -> Option<String> {
    header.strip_prefix(prefix).map(unquote_key)
}

fn target_dependency_table_suffix(header: &str, marker: &str) -> Option<String> {
    header
        .strip_prefix("target.")
        .and_then(|_| header.split_once(marker))
        .map(|(_, name)| unquote_key(name))
}

fn inline_package_name(value: &str) -> Option<String> {
    let value = value.trim();
    if !(value.starts_with('{') && value.ends_with('}')) {
        return None;
    }

    value
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(',')
        .filter_map(|item| item.split_once('='))
        .find_map(|(key, value)| {
            (key.trim() == "package")
                .then(|| quoted_value(value))
                .flatten()
        })
}

fn quoted_value(value: &str) -> Option<String> {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_owned)
}

fn unquote_key(value: &str) -> String {
    value.trim().trim_matches('"').to_owned()
}

fn strip_toml_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut escaped = false;

    for (index, byte) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' && in_string {
            escaped = true;
        } else if *byte == b'"' {
            in_string = !in_string;
        } else if *byte == b'#' && !in_string {
            return &line[..index];
        }
    }

    line
}

#[test]
fn dependency_parser_covers_target_specific_and_renamed_dependencies() {
    let dependencies = manifest_dependencies_from_str(
        r#"
        [dependencies]
        world-core = { path = "../world-core" }
        model = { package = "world-model", path = "../world-model" }

        [dev-dependencies.test-runtime]
        package = "world-runtime"
        path = "../world-runtime"

        [build-dependencies]
        build_alias = { package = "world-standard-runtime", path = "../world-standard-runtime" }

        [target.'cfg(unix)'.dependencies]
        standard = { package = "world-standard", path = "../world-standard" }

        [target.'cfg(test)'.dev-dependencies.world-context]
        path = "../world-context"
        "#,
    );

    for expected in [
        "world-core",
        "world-model",
        "test-runtime",
        "world-runtime",
        "build_alias",
        "world-standard-runtime",
        "standard",
        "world-standard",
        "world-context",
    ] {
        assert!(
            dependencies.contains(expected),
            "missing dependency parser fixture entry {expected}"
        );
    }
}
