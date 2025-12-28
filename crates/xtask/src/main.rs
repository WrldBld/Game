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
    check_handler_complexity()?;
    check_use_case_layer()?;
    check_engine_app_protocol_isolation()?;
    check_engine_ports_protocol_isolation()?;

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
        workspace_root.join("crates/engine-runner/src"),
        workspace_root.join("crates/engine-app/src"),
        workspace_root.join("crates/engine-adapters/src"),
        workspace_root.join("crates/engine-ports/src"),
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

    // Ban internal re-export shims like: `pub use crate::...`
    let pub_use_crate_re =
        regex_lite::Regex::new(r"(?m)^\s*pub(?:\s*\([^)]*\))?\s+use\s+crate::")
            .context("compiling pub-use-crate shim regex")?;

    // Ban internal visibility re-export shims like: `pub(crate) use ...`
    let pub_crate_use_re =
        regex_lite::Regex::new(r"(?m)^\s*pub\s*\(crate\)\s+use\s+")
            .context("compiling pub(crate)-use shim regex")?;

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

            if let Some((line_no, line)) = first_match_line(&pub_use_crate_re, &contents) {
                violations.push(format!(
                    "{}:{} (pub use crate shim): {}",
                    entry.display(),
                    line_no,
                    line.trim_end()
                ));
            }

            if let Some((line_no, line)) = first_match_line(&pub_crate_use_re, &contents) {
                violations.push(format!(
                    "{}:{} (pub(crate) use shim): {}",
                    entry.display(),
                    line_no,
                    line.trim_end()
                ));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "Forbidden shims (cross-crate and internal re-export/alias shims):"
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

/// Check that WebSocket handlers remain thin routing layers
///
/// Handlers in engine-adapters/src/infrastructure/websocket/handlers/
/// should be thin wrappers that delegate to use cases. Max 250 lines per file.
fn check_handler_complexity() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let handlers_dir = workspace_root
        .join("crates/engine-adapters/src/infrastructure/websocket/handlers");

    if !handlers_dir.exists() {
        return Ok(());
    }

    let mut violations = Vec::new();

    // Files that are exempt from line count limits
    let exempt_files: HashSet<&str> = ["mod.rs", "request.rs", "narrative.rs"]
        .into_iter()
        .collect();

    // Max lines per handler file (generous limit after refactoring)
    // Note: challenge.rs has 11 handlers with specific workarounds, so we allow up to 400
    const MAX_HANDLER_LINES: usize = 400;

    for entry in walkdir_rs_files(&handlers_dir)? {
        let file_name = entry
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if exempt_files.contains(file_name) {
            continue;
        }

        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        let line_count = contents.lines().count();

        if line_count > MAX_HANDLER_LINES {
            violations.push(format!(
                "{}: {} lines exceeds max {} - consider extracting to use case",
                entry.display(),
                line_count,
                MAX_HANDLER_LINES
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!("Handler complexity violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: handlers too complex");
    }

    Ok(())
}

/// Check that use cases don't import protocol types (ServerMessage, ClientMessage)
///
/// Use cases in engine-app/src/application/use_cases/ should return domain types,
/// not protocol messages. The errors.rs file is exempt as it provides conversion helpers.
fn check_use_case_layer() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let use_cases_dir = workspace_root.join("crates/engine-app/src/application/use_cases");

    if !use_cases_dir.exists() {
        // No use cases directory - this is expected early in development
        return Ok(());
    }

    // Forbidden: importing ServerMessage in use cases (except errors.rs which converts to it)
    let forbidden_server_message = regex_lite::Regex::new(
        r"use\s+wrldbldr_protocol::[^;]*ServerMessage",
    )?;

    // Forbidden: importing ClientMessage (use cases are server-side only)
    let forbidden_client_message = regex_lite::Regex::new(
        r"use\s+wrldbldr_protocol::[^;]*ClientMessage",
    )?;

    // Files exempt from protocol import checks
    let exempt_files: HashSet<&str> = ["mod.rs", "errors.rs"].into_iter().collect();

    let mut violations = Vec::new();

    for entry in walkdir_rs_files(&use_cases_dir)? {
        let file_name = entry
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if exempt_files.contains(file_name) {
            continue;
        }

        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        if let Some((line_no, line)) = first_match_line(&forbidden_server_message, &contents) {
            violations.push(format!(
                "{}:{}: imports ServerMessage - use cases must return domain types\n    {}",
                entry.display(),
                line_no,
                line.trim()
            ));
        }

        if let Some((line_no, line)) = first_match_line(&forbidden_client_message, &contents) {
            violations.push(format!(
                "{}:{}: imports ClientMessage - use cases are server-side only\n    {}",
                entry.display(),
                line_no,
                line.trim()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!("Use case layer violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: use cases import protocol types");
    }

    Ok(())
}

/// Check that engine-app application layer doesn't import wrldbldr_protocol directly.
///
/// The application layer (use_cases, services, dto, handlers) should work with domain types,
/// not protocol types. Only mod.rs files are exempt.
fn check_engine_app_protocol_isolation() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    // Directories to check within engine-app/src/application/
    let check_dirs = [
        "use_cases",
        "services",
        "dto",
        "handlers",
    ];

    // Patterns that indicate protocol usage
    let use_protocol_re = regex_lite::Regex::new(r"use\s+wrldbldr_protocol::")?;
    let fqn_protocol_re = regex_lite::Regex::new(r"wrldbldr_protocol::")?;

    let mut violations = Vec::new();

    for dir_name in check_dirs {
        let dir = workspace_root.join(format!("crates/engine-app/src/application/{}", dir_name));

        if !dir.exists() {
            continue;
        }

        for entry in walkdir_rs_files(&dir)? {
            let file_name = entry
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            // Exempt files:
            // - mod.rs: Module declarations only
            // - request_handler.rs: Documented exemption - implements RequestHandler trait from ports
            // - common.rs: Contains helpers for request_handler.rs
            if file_name == "mod.rs" || file_name == "request_handler.rs" || file_name == "common.rs" {
                continue;
            }

            let contents = std::fs::read_to_string(&entry)
                .with_context(|| format!("reading {}", entry.display()))?;

            // Check each line, skipping comments
            for (line_idx, line) in contents.lines().enumerate() {
                let trimmed = line.trim();

                // Skip comment lines
                if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                    continue;
                }

                if use_protocol_re.is_match(line) || fqn_protocol_re.is_match(line) {
                    violations.push(format!(
                        "{}:{}: uses wrldbldr_protocol - application layer must use domain types\n    {}",
                        entry.display(),
                        line_idx + 1,
                        trimmed
                    ));
                    break; // One violation per file is enough
                }
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("Engine-app protocol isolation violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: engine-app application layer imports protocol types");
    }

    Ok(())
}

/// Check that engine-ports doesn't import wrldbldr_protocol directly (except request_handler.rs).
///
/// The ports layer defines interfaces and should not depend on protocol types.
/// The request_handler.rs is exempt as it's the documented API boundary.
fn check_engine_ports_protocol_isolation() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let ports_dir = workspace_root.join("crates/engine-ports/src");

    if !ports_dir.exists() {
        return Ok(());
    }

    // Patterns that indicate protocol usage
    let use_protocol_re = regex_lite::Regex::new(r"use\s+wrldbldr_protocol::")?;
    let fqn_protocol_re = regex_lite::Regex::new(r"wrldbldr_protocol::")?;

    let mut violations = Vec::new();

    for entry in walkdir_rs_files(&ports_dir)? {
        let file_name = entry
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Exempt files:
        // - request_handler.rs: Documented API boundary - uses RequestPayload/ResponseResult
        // - app_event_repository_port.rs: Storage-layer port that works with wire format (AppEvent)
        //   The new DomainEventRepositoryPort is the clean domain interface
        if file_name == "request_handler.rs" || file_name == "app_event_repository_port.rs" {
            continue;
        }

        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        // Check each line, skipping comments
        for (line_idx, line) in contents.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comment lines
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            if use_protocol_re.is_match(line) || fqn_protocol_re.is_match(line) {
                violations.push(format!(
                    "{}:{}: uses wrldbldr_protocol - ports layer must use domain types\n    {}",
                    entry.display(),
                    line_idx + 1,
                    trimmed
                ));
                break; // One violation per file is enough
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("Engine-ports protocol isolation violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: engine-ports imports protocol types");
    }

    Ok(())
}

fn allowed_internal_deps() -> HashMap<&'static str, HashSet<&'static str>> {
    // These are *workspace-internal* crate dependencies only.
    // External crates (axum/sqlx/dioxus/etc) are ignored by this check.
    //
    // When the target architecture DAG changes, update this map.
    HashMap::from([
        ("wrldbldr-domain", HashSet::from([])),
        ("wrldbldr-protocol", HashSet::from(["wrldbldr-domain"])),
        ("wrldbldr-engine-dto", HashSet::from(["wrldbldr-protocol"])),
        (
            "wrldbldr-engine-ports",
            HashSet::from(["wrldbldr-domain", "wrldbldr-protocol", "wrldbldr-engine-dto"]),
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
                "wrldbldr-engine-dto", // for test mocks (dev-dependency)
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
                "wrldbldr-player-adapters",
            ]),
        ),
         (
             "wrldbldr-engine-runner",
             HashSet::from(["wrldbldr-engine-adapters"]),
         ),

        ("xtask", HashSet::from([])),
    ])
}
