use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXPECTED_MEMBERS: &[&str] = &[
    "crates/world-authoring",
    "crates/world-conformance",
    "crates/world-context",
    "crates/world-core",
    "crates/world-decision",
    "crates/world-defs",
    "crates/world-engine",
    "crates/world-model",
    "crates/world-runtime",
    "crates/world-standard",
    "crates/world-standard-runtime",
];

const EXPECTED_DIRECT_DEPENDENCIES: &[ExpectedDependency] = &[
    ExpectedDependency::normal("world-authoring", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-authoring", "world-defs", "0.0.0", &[]),
    ExpectedDependency::dev("world-conformance", "world-authoring", "0.0.0", &[]),
    ExpectedDependency::dev("world-conformance", "world-engine", "0.0.0", &[]),
    ExpectedDependency::dev("world-conformance", "world-standard", "0.0.0", &[]),
    ExpectedDependency::dev("world-conformance", "world-standard-runtime", "0.0.0", &[]),
    ExpectedDependency::normal("world-context", "minicbor", "2.3.0", &["alloc"]),
    ExpectedDependency::normal("world-context", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-context", "world-defs", "0.0.0", &[]),
    ExpectedDependency::normal("world-context", "world-model", "0.0.0", &[]),
    ExpectedDependency::normal("world-core", "blake3", "1.8.5", &[]),
    ExpectedDependency::normal("world-decision", "minicbor", "2.3.0", &["alloc"]),
    ExpectedDependency::normal("world-decision", "world-context", "0.0.0", &[]),
    ExpectedDependency::normal("world-decision", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-decision", "world-model", "0.0.0", &[]),
    ExpectedDependency::normal("world-defs", "minicbor", "2.3.0", &["alloc"]),
    ExpectedDependency::normal("world-defs", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-engine", "world-context", "0.0.0", &[]),
    ExpectedDependency::normal("world-engine", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-engine", "world-decision", "0.0.0", &[]),
    ExpectedDependency::normal("world-engine", "world-defs", "0.0.0", &[]),
    ExpectedDependency::normal("world-engine", "world-model", "0.0.0", &[]),
    ExpectedDependency::normal("world-engine", "world-runtime", "0.0.0", &[]),
    ExpectedDependency::normal("world-model", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-model", "world-defs", "0.0.0", &[]),
    ExpectedDependency::normal("world-runtime", "blake3", "1.8.5", &[]),
    ExpectedDependency::normal("world-runtime", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-runtime", "world-defs", "0.0.0", &[]),
    ExpectedDependency::normal("world-runtime", "world-model", "0.0.0", &[]),
    ExpectedDependency::normal("world-standard", "world-defs", "0.0.0", &[]),
    ExpectedDependency::normal("world-standard-runtime", "world-core", "0.0.0", &[]),
    ExpectedDependency::normal("world-standard-runtime", "world-defs", "0.0.0", &[]),
    ExpectedDependency::normal("world-standard-runtime", "world-model", "0.0.0", &[]),
    ExpectedDependency::normal("world-standard-runtime", "world-runtime", "0.0.0", &[]),
    ExpectedDependency::normal("world-standard-runtime", "world-standard", "0.0.0", &[]),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum DependencyKind {
    Normal,
    Build,
    Dev,
}

impl DependencyKind {
    const ALL: [Self; 3] = [Self::Normal, Self::Build, Self::Dev];

    const fn cargo_edge(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Build => "build",
            Self::Dev => "dev",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Dependency {
    owner: String,
    kind: DependencyKind,
    name: String,
    version: String,
    features: BTreeSet<String>,
}

#[derive(Clone, Copy)]
struct ExpectedDependency {
    owner: &'static str,
    kind: DependencyKind,
    name: &'static str,
    version: &'static str,
    features: &'static [&'static str],
}

impl ExpectedDependency {
    const fn normal(
        owner: &'static str,
        name: &'static str,
        version: &'static str,
        features: &'static [&'static str],
    ) -> Self {
        Self {
            owner,
            kind: DependencyKind::Normal,
            name,
            version,
            features,
        }
    }

    const fn dev(
        owner: &'static str,
        name: &'static str,
        version: &'static str,
        features: &'static [&'static str],
    ) -> Self {
        Self {
            owner,
            kind: DependencyKind::Dev,
            name,
            version,
            features,
        }
    }

    fn materialize(self) -> Dependency {
        Dependency {
            owner: self.owner.to_owned(),
            kind: self.kind,
            name: self.name.to_owned(),
            version: self.version.to_owned(),
            features: self
                .features
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CargoPackage {
    name: String,
    version: String,
    location: Option<String>,
    features: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CargoTreeEntry {
    depth: usize,
    package: CargoPackage,
}

#[test]
fn workspace_members_and_dependencies_match_the_architecture() {
    let root = workspace_root();
    let root_manifest_path = root.join("Cargo.toml");
    let expected_packages: BTreeSet<_> = EXPECTED_MEMBERS
        .iter()
        .map(|member| file_name(Path::new(member)))
        .collect();
    let all_features_tree = cargo_tree(
        &root,
        &[
            "--workspace",
            "--all-features",
            "--target",
            "all",
            "--no-dedupe",
            "--prefix",
            "depth",
            "--edges",
            "normal,build,dev",
            "--format",
            "{p}|{f}",
        ],
    );
    let all_entries = parse_cargo_tree(&all_features_tree);
    let actual_packages: BTreeSet<_> = all_entries
        .iter()
        .filter(|entry| entry.depth == 0)
        .map(|entry| entry.package.name.clone())
        .collect();
    assert_eq!(
        actual_packages, expected_packages,
        "workspace membership changed without updating the architecture allowlist"
    );

    let expected_manifests: BTreeSet<_> = std::iter::once(root_manifest_path.clone())
        .chain(
            EXPECTED_MEMBERS
                .iter()
                .map(|member| root.join(member).join("Cargo.toml")),
        )
        .map(canonical)
        .collect();
    let mut actual_manifests = BTreeSet::from([canonical(&root_manifest_path)]);
    visit_files(&root.join("crates"), &mut |path| {
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            actual_manifests.insert(canonical(path));
        }
    });
    assert_eq!(
        actual_manifests, expected_manifests,
        "a dormant or unselected crate manifest exists under crates/"
    );

    let active_roots: BTreeMap<_, _> = EXPECTED_MEMBERS
        .iter()
        .map(|member| {
            let path = canonical(root.join(member));
            (file_name(&path), path)
        })
        .collect();
    for entry in all_entries.iter().filter(|entry| entry.depth == 0) {
        let package = &entry.package;
        let expected_root = active_roots
            .get(&package.name)
            .unwrap_or_else(|| panic!("Cargo selected an unexpected package: {}", package.name));
        let location = package
            .location
            .as_deref()
            .unwrap_or_else(|| panic!("workspace package has no local source: {}", package.name));
        assert_eq!(
            canonical(location),
            *expected_root,
            "workspace package must be selected from its architecture-owned directory"
        );
        assert_eq!(
            package.version, "0.0.0",
            "workspace package version changed without updating the architecture contract"
        );
        assert!(
            package.features.is_empty(),
            "crate feature switches are outside the active architecture: {}",
            package.name
        );
    }
    validate_package_sources(&all_entries, &active_roots);

    let mut actual = BTreeSet::new();
    for kind in DependencyKind::ALL {
        let tree = cargo_tree(
            &root,
            &[
                "--workspace",
                "--target",
                "all",
                "--depth",
                "1",
                "--no-dedupe",
                "--prefix",
                "depth",
                "--edges",
                kind.cargo_edge(),
                "--format",
                "{p}|{f}",
            ],
        );
        actual.extend(direct_dependencies(&tree, kind));
    }
    let expected: BTreeSet<_> = EXPECTED_DIRECT_DEPENDENCIES
        .iter()
        .map(|dependency| dependency.materialize())
        .collect();
    assert_eq!(
        actual, expected,
        "Cargo-resolved direct dependencies changed without updating the architecture allowlist"
    );
    assert!(
        actual
            .iter()
            .filter(|dependency| dependency.owner != "world-conformance")
            .all(|dependency| dependency.name != "world-conformance"),
        "a production package depends on the conformance leaf"
    );
}

#[test]
fn active_paths_and_superseded_symbols_match_the_architecture() {
    let root = workspace_root();
    let crates = root.join("crates");
    let actual_directories: BTreeSet<_> = directories(&crates)
        .into_iter()
        .map(|path| file_name(&path))
        .collect();
    let expected_directories: BTreeSet<_> = EXPECTED_MEMBERS
        .iter()
        .map(|member| file_name(Path::new(member)))
        .collect();
    assert_eq!(
        actual_directories, expected_directories,
        "crates/ contains a package directory outside the active architecture"
    );
    let mut active_files = vec![root.join("Cargo.toml")];
    for member in EXPECTED_MEMBERS {
        let member_root = root.join(member);
        active_files.push(member_root.join("Cargo.toml"));
        visit_files(&member_root, &mut |path| {
            if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                active_files.push(path.to_owned());
            }
        });
    }

    let absent_symbols = [
        ("Definition", "Registry", "superseded"),
        ("Definition", "Id", "superseded"),
        ("Version", "Anchor", "superseded"),
        ("World", "Model", "superseded"),
        ("Causal", "Runtime", "superseded"),
        ("Decision", "Runner", "superseded"),
        ("Decision", "Profile", "superseded"),
        ("ActorContext", "Pipeline", "superseded"),
        ("Store", "Cursor", "superseded"),
        ("Query", "Epoch", "superseded"),
        ("Simulation", "Time", "superseded"),
        ("Accepted", "HardCommit", "superseded"),
        ("Accepted", "RuntimeControlUpdate", "superseded"),
        ("SchedulerLane", "V1", "superseded"),
        ("ExecutionConfigArtifact", "V1", "superseded"),
        ("ExecutionConfigArtifact", "V2", "superseded"),
        ("LifecycleProfiles", "V1", "superseded"),
        ("ActionExternalInvocation", "Proposal", "superseded"),
        ("apply_", "hard_commit", "superseded"),
        ("apply_", "runtime_control_update", "superseded"),
        ("Random", "Stream", "deferred authority"),
    ];
    for file in active_files {
        let source = read(&file);
        for (prefix, suffix, category) in absent_symbols {
            let symbol = format!("{prefix}{suffix}");
            assert!(
                !source.contains(&symbol),
                "{category} symbol {symbol} exists in {}",
                file.display()
            );
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(canonical)
        .unwrap_or_else(|| panic!("conformance crate must be nested under the workspace root"))
}

fn cargo_tree(root: &Path, arguments: &[&str]) -> String {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .arg("tree")
        .arg("--locked")
        .arg("--offline")
        .args(arguments)
        .current_dir(root)
        .output()
        .unwrap_or_else(|error| panic!("cannot run Cargo's resolved-graph inspection: {error}"));
    let stderr = String::from_utf8(output.stderr)
        .unwrap_or_else(|error| panic!("Cargo emitted non-UTF-8 diagnostics: {error}"));
    assert!(
        output.status.success(),
        "Cargo could not resolve the architecture graph:\n{stderr}"
    );
    assert_no_override_warning(&stderr);
    String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("Cargo emitted non-UTF-8 graph output: {error}"))
}

fn assert_no_override_warning(stderr: &str) {
    let diagnostics = stderr.to_ascii_lowercase();
    let unused_patch = diagnostics.contains("patch") && diagnostics.contains("not used");
    let unused_replacement =
        diagnostics.contains("replacement") && diagnostics.contains("not used");
    assert!(
        !unused_patch && !unused_replacement,
        "package overrides are outside the active architecture:\n{stderr}"
    );
}

fn parse_cargo_tree(output: &str) -> Vec<CargoTreeEntry> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(parse_cargo_tree_entry)
        .collect()
}

fn parse_cargo_tree_entry(line: &str) -> CargoTreeEntry {
    let depth_end = line
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or_else(|| panic!("Cargo tree entry contains only a depth: {line}"));
    assert!(depth_end > 0, "Cargo tree entry has no depth: {line}");
    let depth = line[..depth_end]
        .parse()
        .unwrap_or_else(|error| panic!("invalid Cargo tree depth in {line}: {error}"));
    let (package, raw_features) = line[depth_end..]
        .rsplit_once('|')
        .unwrap_or_else(|| panic!("Cargo tree entry has no feature projection: {line}"));
    let mut fields = package.splitn(3, ' ');
    let name = fields
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| panic!("Cargo tree entry has no package name: {line}"));
    let version = fields
        .next()
        .and_then(|version| version.strip_prefix('v'))
        .unwrap_or_else(|| panic!("Cargo tree entry has no package version: {line}"));
    let location = fields.next().map(|location| {
        location
            .strip_prefix('(')
            .and_then(|location| location.strip_suffix(')'))
            .unwrap_or_else(|| panic!("invalid Cargo package location in {line}"))
            .to_owned()
    });
    let features = raw_features
        .split(',')
        .map(str::trim)
        .filter(|feature| !feature.is_empty())
        .map(str::to_owned)
        .collect();
    CargoTreeEntry {
        depth,
        package: CargoPackage {
            name: name.to_owned(),
            version: version.to_owned(),
            location,
            features,
        },
    }
}

fn direct_dependencies(output: &str, kind: DependencyKind) -> BTreeSet<Dependency> {
    let mut owner = None;
    let mut dependencies = BTreeSet::new();
    for entry in parse_cargo_tree(output) {
        match entry.depth {
            0 => owner = Some(entry.package.name),
            1 => {
                let owner = owner
                    .as_ref()
                    .unwrap_or_else(|| panic!("Cargo listed a dependency before its owner"));
                dependencies.insert(Dependency {
                    owner: owner.clone(),
                    kind,
                    name: entry.package.name,
                    version: entry.package.version,
                    features: entry.package.features,
                });
            }
            depth => panic!("depth-limited Cargo tree returned depth {depth}"),
        }
    }
    dependencies
}

fn validate_package_sources(entries: &[CargoTreeEntry], active_roots: &BTreeMap<String, PathBuf>) {
    let active_roots: BTreeSet<_> = active_roots.values().cloned().collect();
    for package in entries.iter().map(|entry| &entry.package) {
        let Some(location) = &package.location else {
            continue;
        };
        let path = Path::new(location);
        assert!(
            path.is_absolute(),
            "non-registry package source is outside the active workspace: {} {location}",
            package.name
        );
        let location = canonical(path);
        assert!(
            active_roots.contains(&location),
            "path dependency or package override selects a source outside the active workspace: \
             {} at {}",
            package.name,
            location.display()
        );
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn canonical(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    fs::canonicalize(path)
        .unwrap_or_else(|error| panic!("cannot canonicalize {}: {error}", path.display()))
}

fn directories(root: &Path) -> Vec<PathBuf> {
    entries(root)
        .into_iter()
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_dir())
                .unwrap_or_else(|error| {
                    panic!("cannot inspect {}: {error}", entry.path().display())
                })
        })
        .map(|entry| entry.path())
        .collect()
}

fn visit_files(root: &Path, visitor: &mut impl FnMut(&Path)) {
    for entry in entries(root) {
        let path = entry.path();
        let kind = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        if kind.is_dir() {
            visit_files(&path, visitor);
        } else if kind.is_file() {
            visitor(&path);
        }
    }
}

fn entries(root: &Path) -> Vec<fs::DirEntry> {
    fs::read_dir(root)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", root.display()))
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("cannot read an entry under {}: {error}", root.display())
            })
        })
        .collect()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("path has no UTF-8 file name: {}", path.display()))
}
