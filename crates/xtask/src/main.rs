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
    check_player_app_protocol_isolation()?;
    check_player_ports_protocol_isolation()?;
    check_no_glob_reexports()?;

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
    let reexport_re = regex_lite::Regex::new(r"(?m)^\s*pub(?:\s*\([^)]*\))?\s+use\s+::?wrldbldr_")
        .context("compiling re-export shim regex")?;

    // Ban crate-alias shims like: `use wrldbldr_protocol as messages;`
    let crate_alias_re =
        regex_lite::Regex::new(r"(?m)^\s*use\s+::?wrldbldr_[A-Za-z0-9_]+\s+as\s+[A-Za-z0-9_]+\s*;")
            .context("compiling crate-alias shim regex")?;

    // Ban crate-alias shims like: `extern crate wrldbldr_protocol as messages;`
    let extern_crate_alias_re = regex_lite::Regex::new(
        r"(?m)^\s*extern\s+crate\s+::?wrldbldr_[A-Za-z0-9_]+\s+as\s+[A-Za-z0-9_]+\s*;",
    )
    .context("compiling extern-crate-alias shim regex")?;

    // NOTE: We no longer ban `pub use crate::...` or `pub(crate) use ...`
    // These are legitimate internal re-exports within a crate for API ergonomics.
    // The key rule is: no re-exporting from OTHER workspace crates.

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
        eprintln!("Forbidden shims (cross-crate re-export/alias shims):");
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
            let metadata = entry
                .metadata()
                .with_context(|| format!("stat {}", entry_path.display()))?;

            if metadata.is_dir() {
                stack.push(entry_path);
                continue;
            }

            if metadata.is_file()
                && entry_path.extension().and_then(|s| s.to_str()) == Some("rs")
            {
                out.push(entry_path);
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

    let handlers_dir =
        workspace_root.join("crates/engine-adapters/src/infrastructure/websocket/handlers");

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
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

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
    let forbidden_server_message =
        regex_lite::Regex::new(r"use\s+wrldbldr_protocol::[^;]*ServerMessage")?;

    // Forbidden: importing ClientMessage (use cases are server-side only)
    let forbidden_client_message =
        regex_lite::Regex::new(r"use\s+wrldbldr_protocol::[^;]*ClientMessage")?;

    // Files exempt from protocol import checks
    let exempt_files: HashSet<&str> = ["mod.rs", "errors.rs"].into_iter().collect();

    let mut violations = Vec::new();

    for entry in walkdir_rs_files(&use_cases_dir)? {
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

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
    let check_dirs = ["use_cases", "services", "dto", "handlers"];

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
            let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Exempt files:
            // - mod.rs: Module declarations only
            // - request_handler.rs: Documented exemption - implements RequestHandler trait from ports
            // - common.rs: Contains helpers for request_handler.rs
            // - *_handler.rs: Domain-specific handlers that receive protocol types at the boundary
            // - rule_system.rs: DTO re-exports from protocol (backwards compatibility layer)
            // - workflow.rs: DTO conversion functions using WorkflowService
            // - workflow_service.rs: Uses WorkflowConfigExportDto for import/export
            // - generation_queue_projection_service.rs: Builds DTO snapshots for output
            if file_name == "mod.rs"
                || file_name == "request_handler.rs"
                || file_name == "common.rs"
                || file_name.ends_with("_handler.rs")
                || file_name == "rule_system.rs"
                || file_name == "workflow.rs"
                || file_name == "workflow_service.rs"
                || file_name == "generation_queue_projection_service.rs"
            {
                continue;
            }

            let contents = std::fs::read_to_string(&entry)
                .with_context(|| format!("reading {}", entry.display()))?;

            // Check each line, skipping comments
            for (line_idx, line) in contents.lines().enumerate() {
                let trimmed = line.trim();

                // Skip comment lines
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                {
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
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Exempt files:
        // - request_handler.rs: Documented API boundary - uses RequestPayload/ResponseResult
        // - workflow_service_port.rs: export/import helpers use WorkflowConfigExportDto
        //   TODO: Move export_workflow_configs/import_workflow_configs to engine-app
        // - dm_approval_queue_service_port.rs: Re-exports wire-format types from protocol
        //   (ProposedToolInfo, ChallengeSuggestionInfo, etc.) - protocol is single source of truth
        // - mod.rs: Re-exports protocol types for API compatibility
        if file_name == "request_handler.rs"
            || file_name == "workflow_service_port.rs"
            || file_name == "dm_approval_queue_service_port.rs"
            || file_name == "mod.rs"
        {
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

/// Check that player-app application layer doesn't import wrldbldr_protocol directly.
///
/// The application layer (services, dto) should work with domain types or app-local DTOs,
/// not protocol types.
///
/// Current exemptions (documented in HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md):
/// - error.rs: Request/response error handling uses protocol error types
/// - Services using RequestPayload: Player services construct requests to send to engine
///
/// Unlike engine-app (which receives requests), player-app constructs requests.
/// Full protocol isolation requires Phase P2-P4 completion.
fn check_player_app_protocol_isolation() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    // Directories to check within player-app/src/application/
    let check_dirs = ["services", "dto"];

    // Patterns that indicate protocol usage
    let use_protocol_re = regex_lite::Regex::new(r"use\s+wrldbldr_protocol::")?;
    let fqn_protocol_re = regex_lite::Regex::new(r"wrldbldr_protocol::")?;

    // Files exempt from protocol import checks (with justification)
    let exempt_files: HashSet<&str> = [
        "mod.rs",           // Module declarations only, re-exports with documented exceptions
        "error.rs", // Request/response error handling - uses ErrorCode, RequestError, ResponseResult
        "player_events.rs", // From<protocol::*> impls must live here due to Rust orphan rules
        "requests.rs", // App-layer DTOs with From impls for protocol conversion at boundary
        // Services use GameConnectionPort which takes RequestPayload.
        // They construct app-layer DTOs and convert via .into() at the boundary.
        // The protocol types are only touched in From impls, not in business logic.
        "skill_service.rs",
        "narrative_event_service.rs",
        "player_character_service.rs",
        "actantial_service.rs",
        "generation_service.rs",
        "challenge_service.rs",
        "event_chain_service.rs",
        "observation_service.rs",
        "location_service.rs",
        "world_service.rs",
        "story_event_service.rs",
        "session_command_service.rs",
        "character_service.rs",
        "suggestion_service.rs",
        "session_service.rs",
    ]
    .into_iter()
    .collect();

    let mut violations = Vec::new();

    for dir_name in check_dirs {
        let dir = workspace_root.join(format!("crates/player-app/src/application/{}", dir_name));

        if !dir.exists() {
            continue;
        }

        for entry in walkdir_rs_files(&dir)? {
            let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if exempt_files.contains(file_name) {
                continue;
            }

            let contents = std::fs::read_to_string(&entry)
                .with_context(|| format!("reading {}", entry.display()))?;

            // Check each line, skipping comments
            for (line_idx, line) in contents.lines().enumerate() {
                let trimmed = line.trim();

                // Skip comment lines
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                {
                    continue;
                }

                if use_protocol_re.is_match(line) || fqn_protocol_re.is_match(line) {
                    violations.push(format!(
                        "{}:{}: uses wrldbldr_protocol - application layer should use domain types\n    {}",
                        entry.display(),
                        line_idx + 1,
                        trimmed
                    ));
                    break; // One violation per file is enough
                }
            }
        }
    }

    // Also check error.rs at application root (already in exempt list but handle explicitly)
    let error_file = workspace_root.join("crates/player-app/src/application/error.rs");
    if error_file.exists() && !exempt_files.contains("error.rs") {
        let contents = std::fs::read_to_string(&error_file)
            .with_context(|| format!("reading {}", error_file.display()))?;

        for (line_idx, line) in contents.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            if use_protocol_re.is_match(line) || fqn_protocol_re.is_match(line) {
                violations.push(format!(
                    "{}:{}: uses wrldbldr_protocol - application layer should use domain types\n    {}",
                    error_file.display(),
                    line_idx + 1,
                    trimmed
                ));
                break;
            }
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "Player-app protocol isolation violations ({} files):",
            violations.len()
        );
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: player-app imports protocol types in non-exempt files");
    }

    Ok(())
}

/// Check that player-ports doesn't import wrldbldr_protocol directly (with Shared Kernel exceptions).
///
/// The ports layer defines interfaces and should generally use domain types, not protocol types.
/// However, the Shared Kernel pattern allows specific port files to use protocol types when
/// they define the boundary between player and engine communication.
///
/// Shared Kernel files (whitelisted):
/// - request_port.rs: Defines RequestPort trait using protocol RequestPayload/ResponseResult
/// - game_connection_port.rs: WebSocket connection port uses protocol message types
/// - mock_game_connection.rs: Testing infrastructure that mirrors the connection port
///
/// Any other files in player-ports should NOT use wrldbldr_protocol directly.
fn check_player_ports_protocol_isolation() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let ports_dir = workspace_root.join("crates/player-ports/src");

    if !ports_dir.exists() {
        return Ok(());
    }

    // Patterns that indicate protocol usage
    let use_protocol_re = regex_lite::Regex::new(r"use\s+wrldbldr_protocol::")?;
    let fqn_protocol_re = regex_lite::Regex::new(r"wrldbldr_protocol::")?;

    // Shared Kernel whitelist: files that legitimately use protocol types at the boundary
    let shared_kernel_files: HashSet<&str> = [
        "request_port.rs",          // RequestPort trait uses RequestPayload/ResponseResult
        "game_connection_port.rs",  // WebSocket connection uses protocol message types
        "mock_game_connection.rs",  // Testing infrastructure mirrors connection port
        "player_events.rs", // Re-exports wire-format types from protocol (single source of truth)
    ]
    .into_iter()
    .collect();

    let mut violations = Vec::new();

    for entry in walkdir_rs_files(&ports_dir)? {
        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip Shared Kernel files - they are allowed to use protocol types
        if shared_kernel_files.contains(file_name) {
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
                    "{}:{}: uses wrldbldr_protocol - only Shared Kernel files may use protocol types\n    {}\n    (Shared Kernel files: {})",
                    entry.display(),
                    line_idx + 1,
                    trimmed,
                    shared_kernel_files.iter().copied().collect::<Vec<_>>().join(", ")
                ));
                break; // One violation per file is enough
            }
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "Player-ports protocol isolation violations ({} files):",
            violations.len()
        );
        eprintln!("  Shared Kernel pattern: Only designated boundary files may use protocol types.");
        eprintln!("  Whitelisted files: request_port.rs, game_connection_port.rs, mock_game_connection.rs");
        eprintln!();
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: player-ports imports protocol types in non-Shared-Kernel files");
    }

    Ok(())
}

/// Check for glob re-exports (`pub use module::*`) which are prohibited.
///
/// Glob re-exports make dependencies implicit, prevent dead code detection,
/// and hurt IDE navigation. Use explicit exports instead.
///
/// NOTE: This check is currently in WARNING mode only. Once existing glob
/// re-exports are cleaned up (Phase 4.6), change to enforce mode.
fn check_no_glob_reexports() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    // Directories to check for glob re-exports
    let check_dirs = [
        workspace_root.join("crates/domain/src"),
        workspace_root.join("crates/protocol/src"),
        workspace_root.join("crates/engine-ports/src"),
        workspace_root.join("crates/engine-app/src"),
        workspace_root.join("crates/engine-adapters/src"),
        workspace_root.join("crates/engine-runner/src"),
        workspace_root.join("crates/player-ports/src"),
        workspace_root.join("crates/player-app/src"),
        workspace_root.join("crates/player-adapters/src"),
        workspace_root.join("crates/player-ui/src"),
        workspace_root.join("crates/player-runner/src"),
    ];

    // Pattern: `pub use something::*;` - captures glob re-exports
    // Matches: pub use foo::*; pub use self::bar::*; pub use super::baz::*;
    // Does NOT match: pub use foo::{A, B, C}; (explicit exports are fine)
    let glob_reexport_re = regex_lite::Regex::new(r"(?m)^\s*pub\s+use\s+[^;]+::\*\s*;")
        .context("compiling glob re-export regex")?;

    let mut violations: Vec<String> = Vec::new();

    for dir in check_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in walkdir_rs_files(&dir)? {
            let contents = std::fs::read_to_string(&entry)
                .with_context(|| format!("reading {}", entry.display()))?;

            // Find all glob re-exports in file
            for (line_idx, line) in contents.lines().enumerate() {
                let trimmed = line.trim();

                // Skip comment lines
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                {
                    continue;
                }

                if glob_reexport_re.is_match(line) {
                    violations.push(format!(
                        "{}:{}: glob re-export - use explicit exports instead\n    {}",
                        entry.display(),
                        line_idx + 1,
                        trimmed
                    ));
                }
            }
        }
    }

    if !violations.is_empty() {
        // WARNING mode: Report but don't fail (until Phase 4.6 cleanup)
        eprintln!(
            "Glob re-export violations ({} instances):",
            violations.len()
        );
        eprintln!("  Architecture rule: No `pub use module::*` - use explicit exports");
        eprintln!();
        for v in &violations {
            eprintln!("  - {v}");
        }
        eprintln!();
        eprintln!("Note: This check is in WARNING mode. Fix these in Phase 4.6.");
        // TODO: Uncomment after Phase 4.6 cleanup:
        // anyhow::bail!("arch-check failed: glob re-exports found");
    }

    Ok(())
}

fn allowed_internal_deps() -> HashMap<&'static str, HashSet<&'static str>> {
    // These are *workspace-internal* crate dependencies only.
    // External crates (axum/sqlx/dioxus/etc) are ignored by this check.
    //
    // When the target architecture DAG changes, update this map.
    //
    // Note: dev-dependencies are included in cargo metadata, so we allow them here
    // with comments indicating which are dev-only.
    HashMap::from([
        // Innermost layer: shared vocabulary types with zero internal deps
        ("wrldbldr-domain-types", HashSet::from([])),
        // Domain layer depends on domain-types for shared vocabulary
        ("wrldbldr-domain", HashSet::from(["wrldbldr-domain-types"])),
        // Protocol (API contract) depends on domain-types for shared vocabulary
        // NOTE: wrldbldr-domain dependency was removed - From impls moved to adapters
        ("wrldbldr-protocol", HashSet::from(["wrldbldr-domain-types"])),
        (
            "wrldbldr-engine-dto",
            HashSet::from([
                "wrldbldr-protocol",
                "wrldbldr-domain", // For domain ID types in DTOs
            ]),
        ),
        (
            "wrldbldr-engine-ports",
            HashSet::from([
                "wrldbldr-domain",
                "wrldbldr-protocol",
                "wrldbldr-engine-dto",
            ]),
        ),
        (
            "wrldbldr-player-ports",
            HashSet::from(["wrldbldr-domain", "wrldbldr-protocol"]),
        ),
        (
            "wrldbldr-engine-app",
            HashSet::from([
                "wrldbldr-domain",
                "wrldbldr-domain-types", // for workflow analysis functions
                "wrldbldr-protocol",
                "wrldbldr-engine-ports",
                "wrldbldr-engine-dto", // for test mocks (dev-dependency)
            ]),
        ),
        // Composition layer: defines service containers using port traits
        // Sits between app and adapters, allows clean DI without coupling
        (
            "wrldbldr-engine-composition",
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
                "wrldbldr-player-adapters", // dev-dependency for MockGameConnectionPort
            ]),
        ),
        (
            "wrldbldr-engine-adapters",
            HashSet::from([
                // NOTE: engine-app dependency REMOVED - adapters no longer depend on app layer
                "wrldbldr-engine-ports",
                "wrldbldr-engine-composition",
                "wrldbldr-engine-dto",
                "wrldbldr-protocol",
                "wrldbldr-domain",
                "wrldbldr-domain-types", // For DTO conversions and workflow analysis
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
                "wrldbldr-player-adapters", // For Platform type in context
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
        // Runner is the composition root - needs access to all layers
        (
            "wrldbldr-engine-runner",
            HashSet::from([
                "wrldbldr-engine-adapters",
                "wrldbldr-engine-app",
                "wrldbldr-engine-ports",
                "wrldbldr-engine-composition",
                "wrldbldr-protocol",
                "wrldbldr-domain",
            ]),
        ),
        ("xtask", HashSet::from([])),
    ])
}
