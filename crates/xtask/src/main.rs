use anyhow::Context;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("arch-check") => arch_check(),
        Some(cmd) => anyhow::bail!("Unknown xtask command: {cmd}"),
        None => anyhow::bail!("Usage: cargo xtask <command>\n\nCommands:\n  arch-check"),
    }
}

#[derive(serde::Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    resolve: CargoResolve,
    workspace_members: Vec<String>,
}

#[derive(serde::Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
}

#[derive(serde::Deserialize)]
struct CargoResolve {
    nodes: Vec<CargoNode>,
}

#[derive(serde::Deserialize)]
struct CargoNode {
    id: String,
    #[serde(default)]
    deps: Vec<CargoNodeDep>,
}

#[derive(serde::Deserialize)]
struct CargoNodeDep {
    pkg: String,
}

fn arch_check() -> anyhow::Result<()> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()
        .context("running cargo metadata")?;

    if !output.status.success() {
        anyhow::bail!("cargo metadata failed")
    }

    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).context("parsing cargo metadata JSON")?;

    let workspace_ids: HashSet<&str> = metadata
        .workspace_members
        .iter()
        .map(|id| id.as_str())
        .collect();

    let mut id_to_name = HashMap::<&str, &str>::new();
    for pkg in &metadata.packages {
        if workspace_ids.contains(pkg.id.as_str()) {
            id_to_name.insert(pkg.id.as_str(), pkg.name.as_str());
        }
    }

    let rules = allowed_internal_deps();

    let mut violations: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut checked = 0usize;

    for node in &metadata.resolve.nodes {
        if !workspace_ids.contains(node.id.as_str()) {
            continue;
        }

        let Some(crate_name) = id_to_name.get(node.id.as_str()) else {
            continue;
        };

        checked += 1;

        let Some(allowed) = rules.get(*crate_name) else {
            anyhow::bail!(
                "arch-check missing rules for workspace crate '{crate_name}'. Add it to xtask." 
            );
        };

        for dep in &node.deps {
            if !workspace_ids.contains(dep.pkg.as_str()) {
                continue;
            }

            if let Some(dep_name) = id_to_name.get(dep.pkg.as_str()) {
                if !allowed.contains(*dep_name) {
                    violations
                        .entry((*crate_name).to_string())
                        .or_default()
                        .insert((*dep_name).to_string());
                }
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("Architecture violations (forbidden internal crate deps):");
        for (krate, deps) in violations {
            for dep in deps {
                eprintln!("  - {krate} -> {dep}");
            }
        }
        anyhow::bail!("arch-check failed")
    }

    check_no_cross_crate_shims()?;

    println!("arch-check OK ({checked} workspace crates checked)");
    Ok(())
}

fn check_no_cross_crate_shims() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    // Enforce across the whole workspace *except* the owning crates.
    // (Owning crates may legitimately `pub use` their own internals.)
    let enforced_dirs = [
        workspace_root.join("crates/engine/src"),
        workspace_root.join("crates/engine-app/src"),
        workspace_root.join("crates/engine-adapters/src"),
        workspace_root.join("crates/engine-ports/src"),
        workspace_root.join("crates/player/src"),
        workspace_root.join("crates/player-app/src"),
        workspace_root.join("crates/player-adapters/src"),
        workspace_root.join("crates/player-ports/src"),
        workspace_root.join("crates/player-ui/src"),
        workspace_root.join("crates/player-runner/src"),
    ];

    // Ban cross-crate re-export shims like: `pub use wrldbldr_*::...`
    // Regex avoids whitespace/newline sensitivity.
    let reexport_re =
        regex_lite::Regex::new(r"(?m)^\s*pub(?:\s*\([^)]*\))?\s+use\s+::?wrldbldr_")
            .context("compiling re-export shim regex")?;

    // Ban crate-alias shims like: `use wrldbldr_protocol as messages;`
    let crate_alias_re = regex_lite::Regex::new(
        r"(?m)^\s*use\s+::?wrldbldr_[A-Za-z0-9_]+\s+as\s+[A-Za-z0-9_]+\s*;",
    )
    .context("compiling crate-alias shim regex")?;

    // Ban crate-alias shims like: `extern crate wrldbldr_protocol as messages;`
    let extern_crate_alias_re = regex_lite::Regex::new(
        r"(?m)^\s*extern\s+crate\s+::?wrldbldr_[A-Za-z0-9_]+\s+as\s+[A-Za-z0-9_]+\s*;",
    )
    .context("compiling extern-crate-alias shim regex")?;

    let mut violations: Vec<String> = Vec::new();

    for dir in enforced_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in walkdir_rs_files(&dir)? {
            let contents = std::fs::read_to_string(&entry)
                .with_context(|| format!("reading {}", entry.display()))?;

            if let Some((line_no, line)) = first_match_line(&reexport_re, &contents) {
                violations.push(format!(
                    "{}:{} (re-export shim): {}",
                    entry.display(),
                    line_no,
                    line.trim_end()
                ));
            }

            if let Some((line_no, line)) = first_match_line(&crate_alias_re, &contents) {
                violations.push(format!(
                    "{}:{} (crate-alias shim): {}",
                    entry.display(),
                    line_no,
                    line.trim_end()
                ));
            }

            if let Some((line_no, line)) = first_match_line(&extern_crate_alias_re, &contents) {
                violations.push(format!(
                    "{}:{} (extern crate-alias shim): {}",
                    entry.display(),
                    line_no,
                    line.trim_end()
                ));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "Forbidden cross-crate shims (re-exports and crate aliases of `wrldbldr_*`):"
        );
        for v in violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed")
    }

    Ok(())
}

fn first_match_line<'a>(re: &regex_lite::Regex, contents: &'a str) -> Option<(usize, &'a str)> {
    let mat = re.find(contents)?;
    let start = mat.range().start;

    let line_no = contents[..start].bytes().filter(|b| *b == b'\n').count() + 1;
    let line_start = contents[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = contents[start..]
        .find('\n')
        .map(|i| start + i)
        .unwrap_or(contents.len());

    Some((line_no, &contents[line_start..line_end]))
}

fn walkdir_rs_files(dir: &std::path::Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(path) = stack.pop() {
        let entries =
            std::fs::read_dir(&path).with_context(|| format!("listing {}", path.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("reading entry under {}", path.display()))?;
            let entry_path = entry.path();
            let metadata =
                entry.metadata().with_context(|| format!("stat {}", entry_path.display()))?;

            if metadata.is_dir() {
                stack.push(entry_path);
                continue;
            }

            if metadata.is_file() {
                if entry_path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    out.push(entry_path);
                }
            }
        }
    }

    Ok(out)
}

fn allowed_internal_deps() -> HashMap<&'static str, HashSet<&'static str>> {
    // These are *workspace-internal* crate dependencies only.
    // External crates (axum/sqlx/dioxus/etc) are ignored by this check.
    //
    // When the target architecture DAG changes, update this map.
    HashMap::from([
        ("wrldbldr-domain", HashSet::from([])),
        ("wrldbldr-protocol", HashSet::from([])),
        (
            "wrldbldr-engine-ports",
            HashSet::from(["wrldbldr-domain", "wrldbldr-protocol"]),
        ),
        (
            "wrldbldr-player-ports",
            HashSet::from(["wrldbldr-domain", "wrldbldr-protocol"]),
        ),
        (
            "wrldbldr-engine-app",
            HashSet::from([
                "wrldbldr-domain",
                "wrldbldr-protocol",
                "wrldbldr-engine-ports",
            ]),
        ),
        (
            "wrldbldr-player-app",
            HashSet::from([
                "wrldbldr-domain",
                "wrldbldr-protocol",
                "wrldbldr-player-ports",
            ]),
        ),
        (
            "wrldbldr-engine-adapters",
            HashSet::from([
                "wrldbldr-engine-app",
                "wrldbldr-engine-ports",
                "wrldbldr-protocol",
                "wrldbldr-domain",
            ]),
        ),
        (
            "wrldbldr-player-adapters",
            HashSet::from([
                "wrldbldr-player-app",
                "wrldbldr-player-ports",
                "wrldbldr-protocol",
                "wrldbldr-domain",
            ]),
        ),
        (
            "wrldbldr-player-ui",
            HashSet::from([
                "wrldbldr-player-app",
                "wrldbldr-player-ports",
                "wrldbldr-protocol",
                "wrldbldr-domain",
            ]),
        ),
        (
            "wrldbldr-player-runner",
            HashSet::from([
                "wrldbldr-player-ui",
                "wrldbldr-player-app",
                "wrldbldr-player-ports",
            ]),
        ),
        (
            "wrldbldr-engine",
            HashSet::from(["wrldbldr-engine-adapters"]),
        ),
        (
            "wrldbldr-player",
            HashSet::from([
                "wrldbldr-player-runner",
                "wrldbldr-player-adapters",
                "wrldbldr-player-app",
                "wrldbldr-player-ports",
            ]),
        ),
        ("xtask", HashSet::from([])),
    ])
}
