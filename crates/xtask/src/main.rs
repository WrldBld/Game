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

    let trace = std::env::var("ARCH_CHECK_TRACE").is_ok();
    let step = |name: &str, f: fn() -> anyhow::Result<()>| -> anyhow::Result<()> {
        if trace {
            eprintln!("arch-check: {name}");
        }
        f()
    };

    step("no-cross-crate-shims", check_no_cross_crate_shims)?;
    step("handler-complexity", check_handler_complexity)?;
    step("use-case-layer", check_use_case_layer)?;
    step("engine-protocol-isolation", check_engine_protocol_isolation)?;
    step(
        "engine-ports-protocol-isolation",
        check_engine_ports_protocol_isolation,
    )?;
    step(
        "player-app-protocol-isolation",
        check_player_app_protocol_isolation,
    )?;
    step(
        "player-ports-protocol-isolation",
        check_player_ports_protocol_isolation,
    )?;
    step("no-glob-reexports", check_no_glob_reexports)?;
    step(
        "app-does-not-depend-on-inbound-ports",
        check_app_does_not_depend_on_inbound_ports,
    )?;
    step(
        "no-engine-dto-shadowing-engine-ports-types",
        check_no_engine_dto_shadowing_engine_ports_types,
    )?;
    step(
        "engine-no-internal-service-construction",
        check_engine_app_no_internal_service_construction,
    )?;
    step(
        "engine-runner-composition-no-concrete-arc-fields",
        check_engine_runner_composition_no_concrete_service_fields,
    )?;
    step(
        "engine-runner-composition-no-concrete-pub-fields",
        check_engine_runner_composition_no_concrete_pub_fields,
    )?;
    step(
        "engine-runner-composition-no-shared-world-connection-manager",
        check_engine_runner_composition_no_shared_world_connection_manager,
    )?;
    step(
        "engine-runner-composition-no-world-connection-manager-imports",
        check_engine_runner_composition_no_world_connection_manager_imports,
    )?;

    println!("arch-check OK ({checked} workspace crates checked)");
    Ok(())
}

/// Phase 7: composition root should not store concrete types behind `Arc<...>`
/// in composition factories when a port trait object would suffice.
///
/// This is a heuristic check that looks for `pub <field>: Arc<ConcreteType>` fields
/// inside `engine-runner` composition factories.
///
/// Notes:
/// - Only scans `crates/engine-runner/src/composition/factories/**`.
/// - No file excludes (Phase 7 enforcement).
fn check_engine_runner_composition_no_concrete_service_fields() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let factories_dir = workspace_root.join("crates/engine-runner/src/composition/factories");
    if !factories_dir.exists() {
        return Ok(());
    }

    // regex_lite does not support look-around; capture the first token after `Arc<`
    // and filter out `dyn` in code.
    let field_arc_re = regex_lite::Regex::new(r"\bpub\s+[A-Za-z0-9_]+\s*:\s*Arc<\s*([^\s>]+)")
        .context("compiling composition concrete-service regex")?;

    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir_rs_files(&factories_dir)? {
        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let sanitized = sanitize_rust_for_scan(&contents);
        let skip_ranges = collect_cfg_test_item_ranges(&sanitized);

        for cap in field_arc_re.captures_iter(&sanitized) {
            let Some(ty) = cap.get(1).map(|m| m.as_str()) else {
                continue;
            };
            let Some(mat) = cap.get(0) else {
                continue;
            };
            let start = mat.range().start;
            if offset_is_in_ranges(start, &skip_ranges) {
                continue;
            }

            if ty == "dyn" {
                continue;
            }

            // Determine leaf type name (strip module path and generics)
            let leaf = ty.rsplit("::").next().unwrap_or(ty);
            let leaf = leaf.split('<').next().unwrap_or(leaf);

            // Allowlist: concrete infrastructure types that are intentionally stored in
            // factories and are not modeled as ports.
            let is_allowlisted = matches!(leaf, "QueueBackendEnum");
            if is_allowlisted {
                continue;
            }

            // If it's not a trait object and not allowlisted, treat as a violation.

            let (line_no, line) = line_at_offset(&contents, start);
            violations.push(format!(
                "  - {}:{}: {}",
                entry.display(),
                line_no,
                line.trim_end()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!("arch-check failed: engine-runner composition factories store concrete Arc<T> fields (Phase 7)");
        eprintln!("Use `Arc<dyn ...Port>` fields instead when a port exists.\n");
        for v in violations.iter().take(20) {
            eprintln!("{v}");
        }
        if violations.len() > 20 {
            eprintln!("  ... (+{} more)", violations.len() - 20);
        }
        anyhow::bail!("arch-check failed");
    }

    Ok(())
}

/// Phase 7 (tightened): composition factories should not publicly expose known concrete
/// infrastructure types even when not wrapped in `Arc<...>`.
///
/// This intentionally focuses on a small set of known-leaky concretes to avoid
/// false positives on DTOs/containers that are fine to be concrete.
fn check_engine_runner_composition_no_concrete_pub_fields() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let factories_dir = workspace_root.join("crates/engine-runner/src/composition/factories");
    if !factories_dir.exists() {
        return Ok(());
    }

    // Capture the type portion of a `pub field: Type,` struct field.
    let pub_field_re = regex_lite::Regex::new(r"\bpub\s+[A-Za-z0-9_]+\s*:\s*([^,\n]+)")
        .context("compiling composition pub-field regex")?;

    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir_rs_files(&factories_dir)? {
        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let sanitized = sanitize_rust_for_scan(&contents);
        let skip_ranges = collect_cfg_test_item_ranges(&sanitized);

        for cap in pub_field_re.captures_iter(&sanitized) {
            let Some(ty_raw) = cap.get(1).map(|m| m.as_str()) else {
                continue;
            };
            let Some(mat) = cap.get(0) else {
                continue;
            };
            let start = mat.range().start;
            if offset_is_in_ranges(start, &skip_ranges) {
                continue;
            }

            let ty = ty_raw.trim();

            // Let the dedicated Arc<T> check handle these.
            if ty.starts_with("Arc<") {
                continue;
            }

            // Ignore references and dyn trait objects.
            if ty.starts_with('&') || ty.contains("dyn ") {
                continue;
            }

            // Determine leaf type name (strip module path and generics)
            let leaf = ty.rsplit("::").next().unwrap_or(ty);
            let leaf = leaf.split('<').next().unwrap_or(leaf).trim();

            let is_disallowed = matches!(
                leaf,
                "Neo4jRepository"
                    | "QueueFactory"
                    | "OllamaClient"
                    | "ComfyUIClient"
                    | "InProcessEventNotifier"
            );
            if !is_disallowed {
                continue;
            }

            let (line_no, line) = line_at_offset(&contents, start);
            violations.push(format!(
                "  - {}:{}: {}",
                entry.display(),
                line_no,
                line.trim_end()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "arch-check failed: engine-runner composition factories publicly expose concrete infra types (Phase 7)"
        );
        eprintln!("Make these fields non-public, or expose them via ports / accessors instead.\n");
        for v in violations.iter().take(20) {
            eprintln!("{v}");
        }
        if violations.len() > 20 {
            eprintln!("  ... (+{} more)", violations.len() - 20);
        }
        anyhow::bail!("arch-check failed");
    }

    Ok(())
}

/// Phase 7 (tightened): composition factories must not depend on the concrete
/// `SharedWorldConnectionManager` type.
///
/// The intent is to keep factories port-only so that the composition root wires
/// concrete adapters and hands only `Arc<dyn ...Port>` into factories.
///
/// Notes:
/// - Only scans `crates/engine-runner/src/composition/factories/**`.
/// - Excludes `#[cfg(test)]` items to reduce false positives.
fn check_engine_runner_composition_no_shared_world_connection_manager() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let factories_dir = workspace_root.join("crates/engine-runner/src/composition/factories");
    if !factories_dir.exists() {
        return Ok(());
    }

    let needle = "SharedWorldConnectionManager";
    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir_rs_files(&factories_dir)? {
        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let sanitized = sanitize_rust_for_scan(&contents);
        let skip_ranges = collect_cfg_test_item_ranges(&sanitized);

        for (offset, _) in sanitized.match_indices(needle) {
            if offset_is_in_ranges(offset, &skip_ranges) {
                continue;
            }
            let (line_no, line) = line_at_offset(&contents, offset);
            violations.push(format!(
                "  - {}:{}: {}",
                entry.display(),
                line_no,
                line.trim_end()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "arch-check failed: engine-runner composition factories depend on SharedWorldConnectionManager (Phase 7)"
        );
        eprintln!(
            "Use port traits (e.g. ConnectionManagerPort/ConnectionBroadcastPort/ConnectionUnicastPort) in factories instead.\n"
        );
        for v in violations.iter().take(20) {
            eprintln!("{v}");
        }
        if violations.len() > 20 {
            eprintln!("  ... (+{} more)", violations.len() - 20);
        }
        anyhow::bail!("arch-check failed");
    }

    Ok(())
}

/// Phase 7 (tightened): composition factories must not import the concrete
/// connection-manager module.
///
/// Rationale: factories should be port-only; only the composition root should
/// wire concrete adapters.
///
/// Notes:
/// - Only scans `crates/engine-runner/src/composition/factories/**`.
/// - Excludes `#[cfg(test)]` items to reduce false positives.
fn check_engine_runner_composition_no_world_connection_manager_imports() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let factories_dir = workspace_root.join("crates/engine-runner/src/composition/factories");
    if !factories_dir.exists() {
        return Ok(());
    }

    // Keep the check intentionally narrow to avoid false positives: we only
    // care about `use ... world_connection_manager ...` imports.
    let import_re = regex_lite::Regex::new(r"\buse\s+[^;\n]*\bworld_connection_manager\b[^;\n]*;?")
        .context("compiling world_connection_manager import regex")?;

    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir_rs_files(&factories_dir)? {
        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;
        let sanitized = sanitize_rust_for_scan(&contents);
        let skip_ranges = collect_cfg_test_item_ranges(&sanitized);

        for mat in import_re.find_iter(&sanitized) {
            let start = mat.range().start;
            if offset_is_in_ranges(start, &skip_ranges) {
                continue;
            }
            let (line_no, line) = line_at_offset(&contents, start);
            violations.push(format!(
                "  - {}:{}: {}",
                entry.display(),
                line_no,
                line.trim_end()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "arch-check failed: engine-runner composition factories import world_connection_manager (Phase 7)"
        );
        eprintln!(
            "Move these imports to the composition root (e.g. app_state) and pass only port traits into factories.\n"
        );
        for v in violations.iter().take(20) {
            eprintln!("{v}");
        }
        if violations.len() > 20 {
            eprintln!("  ... (+{} more)", violations.len() - 20);
        }
        anyhow::bail!("arch-check failed");
    }

    Ok(())
}

/// Phase 6: legacy IoC check retained for simplified architecture.
///
/// Historically this enforced that application-layer code (engine-app) did not construct
/// other services internally. With the monolithic `engine` crate, we keep the same check
/// as a guardrail: forbid `*Service::new(...)` in `crates/engine/src/**` outside of
/// `#[cfg(test)]` items.
fn check_engine_app_no_internal_service_construction() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let app_dir = workspace_root.join("crates/engine/src");
    if !app_dir.exists() {
        return Ok(());
    }

    let service_new_re = regex_lite::Regex::new(r"\b[A-Za-z0-9_]+Service::new\s*\(")
        .context("compiling service-new regex")?;

    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir_rs_files(&app_dir)? {
        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        let sanitized = sanitize_rust_for_scan(&contents);
        let skip_ranges = collect_cfg_test_item_ranges(&sanitized);

        for cap in service_new_re.captures_iter(&sanitized) {
            let Some(mat) = cap.get(0) else {
                continue;
            };
            let start = mat.range().start;

            if offset_is_in_ranges(start, &skip_ranges) {
                continue;
            }

            let (line_no, line) = line_at_offset(&contents, start);
            violations.push(format!(
                "  - {}:{}: {}",
                entry.display(),
                line_no,
                line.trim_end()
            ));
        }
    }

    if !violations.is_empty() {
        eprintln!(
            "arch-check failed: engine-app application layer constructs services (Phase 6 IoC)"
        );
        eprintln!("Forbidden pattern: `*Service::new(...)` outside `#[cfg(test)]`\n");
        for v in violations.iter().take(20) {
            eprintln!("{v}");
        }
        if violations.len() > 20 {
            eprintln!("  ... (+{} more)", violations.len() - 20);
        }
        anyhow::bail!("arch-check failed");
    }

    Ok(())
}

fn offset_is_in_ranges(offset: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| *start <= offset && offset < *end)
}

fn line_at_offset(contents: &str, offset: usize) -> (usize, &str) {
    let offset = offset.min(contents.len());
    let line_no = contents[..offset].bytes().filter(|b| *b == b'\n').count() + 1;
    let line_start = contents[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = contents[offset..]
        .find('\n')
        .map(|i| offset + i)
        .unwrap_or(contents.len());
    (line_no, &contents[line_start..line_end])
}

/// Produce a scan-friendly version of a Rust source file where comments and
/// string/char literals are replaced with spaces (newlines preserved).
///
/// This avoids false positives when enforcing code-pattern checks.
fn sanitize_rust_for_scan(contents: &str) -> String {
    let bytes = contents.as_bytes();
    let mut out: Vec<u8> = bytes.to_vec();

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State {
        Normal,
        LineComment,
        BlockComment { depth: usize },
        String,
        Char,
        RawString { hashes: usize },
    }

    let mut state = State::Normal;
    let mut i = 0usize;
    while i < out.len() {
        match state {
            State::Normal => {
                if out[i] == b'/' && i + 1 < out.len() && out[i + 1] == b'/' {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                    state = State::LineComment;
                    continue;
                }
                if out[i] == b'/' && i + 1 < out.len() && out[i + 1] == b'*' {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                    state = State::BlockComment { depth: 1 };
                    continue;
                }

                // Raw string literal: r###" ... "###
                if out[i] == b'r' {
                    let mut j = i + 1;
                    while j < out.len() && out[j] == b'#' {
                        j += 1;
                    }
                    if j < out.len() && out[j] == b'"' {
                        let hashes = j - (i + 1);
                        out[i] = b' ';
                        for byte in out[(i + 1)..(j + 1)].iter_mut() {
                            *byte = b' ';
                        }
                        i = j + 1;
                        state = State::RawString { hashes };
                        continue;
                    }
                }

                if out[i] == b'"' {
                    out[i] = b' ';
                    i += 1;
                    state = State::String;
                    continue;
                }

                if out[i] == b'\'' {
                    out[i] = b' ';
                    i += 1;
                    state = State::Char;
                    continue;
                }

                i += 1;
            }
            State::LineComment => {
                if out[i] == b'\n' {
                    i += 1;
                    state = State::Normal;
                } else {
                    out[i] = b' ';
                    i += 1;
                }
            }
            State::BlockComment { mut depth } => {
                if out[i] == b'\n' {
                    i += 1;
                    state = State::BlockComment { depth };
                    continue;
                }

                if out[i] == b'/' && i + 1 < out.len() && out[i + 1] == b'*' {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    depth += 1;
                    i += 2;
                    state = State::BlockComment { depth };
                    continue;
                }

                if out[i] == b'*' && i + 1 < out.len() && out[i + 1] == b'/' {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    depth = depth.saturating_sub(1);
                    i += 2;
                    if depth == 0 {
                        state = State::Normal;
                    } else {
                        state = State::BlockComment { depth };
                    }
                    continue;
                }

                out[i] = b' ';
                i += 1;
                state = State::BlockComment { depth };
            }
            State::String => {
                if out[i] == b'\n' {
                    i += 1;
                    continue;
                }

                if out[i] == b'\\' {
                    out[i] = b' ';
                    if i + 1 < out.len() {
                        out[i + 1] = b' ';
                        i += 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }

                if out[i] == b'"' {
                    out[i] = b' ';
                    i += 1;
                    state = State::Normal;
                    continue;
                }

                out[i] = b' ';
                i += 1;
            }
            State::Char => {
                if out[i] == b'\n' {
                    i += 1;
                    continue;
                }

                if out[i] == b'\\' {
                    out[i] = b' ';
                    if i + 1 < out.len() {
                        out[i + 1] = b' ';
                        i += 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }

                if out[i] == b'\'' {
                    out[i] = b' ';
                    i += 1;
                    state = State::Normal;
                    continue;
                }

                out[i] = b' ';
                i += 1;
            }
            State::RawString { hashes } => {
                if out[i] == b'\n' {
                    i += 1;
                    continue;
                }

                if out[i] == b'"' {
                    // End marker is '"' followed by `hashes` number of '#'
                    let mut ok = true;
                    for h in 0..hashes {
                        let idx = i + 1 + h;
                        if idx >= out.len() || out[idx] != b'#' {
                            ok = false;
                            break;
                        }
                    }

                    if ok {
                        out[i] = b' ';
                        for h in 0..hashes {
                            out[i + 1 + h] = b' ';
                        }
                        i += 1 + hashes;
                        state = State::Normal;
                        continue;
                    }
                }

                out[i] = b' ';
                i += 1;
            }
        }
    }

    // Safety: we only replaced bytes with ASCII spaces, preserving valid UTF-8.
    String::from_utf8(out).unwrap_or_else(|_| contents.to_string())
}

/// Identify `#[cfg(test)]` (and common variants containing `test`) items and
/// return byte ranges to skip when scanning for forbidden production patterns.
fn collect_cfg_test_item_ranges(sanitized: &str) -> Vec<(usize, usize)> {
    let bytes = sanitized.as_bytes();
    let mut ranges = Vec::new();
    let mut i = 0usize;

    while i + 1 < bytes.len() {
        if bytes[i] != b'#' || bytes[i + 1] != b'[' {
            i += 1;
            continue;
        }

        let attr_start = i;
        let Some(rel_end) = sanitized[i + 2..].find(']') else {
            break;
        };
        let attr_end = i + 2 + rel_end; // index of ']'
        let attr = &sanitized[attr_start..=attr_end];

        // Heuristic: treat any cfg attribute containing the token `test` as test-only.
        // Examples:
        // - #[cfg(test)]
        // - #[cfg(any(test, feature = "..."))]
        let is_cfg = attr.contains("cfg") || attr.contains("cfg_attr");
        let is_test = attr.contains("test");

        if !is_cfg || !is_test {
            i = attr_end + 1;
            continue;
        }

        // Move to the start of the following item.
        let mut j = attr_end + 1;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }

        // Skip an item that ends with ';' before any '{'
        let next_semi = sanitized[j..].find(';').map(|o| j + o);
        let next_brace = sanitized[j..].find('{').map(|o| j + o);

        if let Some(semi) = next_semi {
            if next_brace.is_none() || semi < next_brace.unwrap() {
                ranges.push((attr_start, semi + 1));
                i = semi + 1;
                continue;
            }
        }

        let Some(brace) = next_brace else {
            // No obvious item body; skip just the attribute.
            ranges.push((attr_start, attr_end + 1));
            i = attr_end + 1;
            continue;
        };

        // Balance braces to find the end of the item body.
        let mut depth = 0usize;
        let mut k = brace;
        while k < bytes.len() {
            match bytes[k] {
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        ranges.push((attr_start, k + 1));
                        i = k + 1;
                        break;
                    }
                }
                _ => {}
            }
            k += 1;
        }

        if k >= bytes.len() {
            // Unbalanced braces; fall back to skipping only the attribute.
            ranges.push((attr_start, attr_end + 1));
            i = attr_end + 1;
        }
    }

    ranges
}

/// Check for likely DTO shadowing: types with the same name declared in both
/// `engine-dto` and `engine-ports`.
///
/// This is intended to catch cases like `StagingProposal` being duplicated in both crates.
///
/// This is a heuristic check:
/// - It only looks for `pub struct|enum|type` declarations.
/// - It only compares by simple type name (no module path).
///
/// It runs in WARNING mode initially to avoid blocking while the refactor is in-flight.
fn check_no_engine_dto_shadowing_engine_ports_types() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let engine_dto_dir = workspace_root.join("crates/engine-dto/src");
    let engine_ports_dir = workspace_root.join("crates/engine-ports/src");

    if !engine_dto_dir.exists() || !engine_ports_dir.exists() {
        return Ok(());
    }

    // Capture public declarations. Keep it simple and robust.
    // Examples matched:
    //   pub struct Foo
    //   pub enum Bar
    //   pub type Baz = ...;
    let pub_type_re =
        regex_lite::Regex::new(r"(?m)^\s*pub\s+(?:struct|enum|type)\s+([A-Za-z_][A-Za-z0-9_]*)\b")
            .context("compiling pub type capture regex")?;

    let dto_types = collect_public_type_names(&engine_dto_dir, &pub_type_re)?;
    let port_types = collect_public_type_names(&engine_ports_dir, &pub_type_re)?;

    // Explicitly allowed duplicate names (keep list short; document why).
    // Most duplicates should be removed by moving the DTO to the owning boundary.
    let allowed_duplicates: HashSet<&str> = HashSet::from([
        // Example placeholder; keep empty unless we have a justified exception.
        // "SomeSharedName",
    ]);

    let mut collisions: Vec<String> = dto_types
        .keys()
        .filter(|name| port_types.contains_key(*name))
        .filter(|name| !allowed_duplicates.contains(name.as_str()))
        .cloned()
        .collect();
    collisions.sort();

    if !collisions.is_empty() {
        eprintln!(
            "Potential DTO duplication: public types declared in BOTH engine-dto and engine-ports ({} names)",
            collisions.len()
        );
        eprintln!(
            "  Architecture rule: port boundary DTOs must not be shadow-copied in engine-dto"
        );
        eprintln!("  Fix: move the type to the owning port module (or remove the duplicate and use the port DTO)");
        eprintln!();

        for name in &collisions {
            let dto_locs = dto_types.get(name).cloned().unwrap_or_default();
            let port_locs = port_types.get(name).cloned().unwrap_or_default();

            eprintln!("  - {name}");
            for loc in dto_locs.iter().take(3) {
                eprintln!("      engine-dto:  {loc}");
            }
            if dto_locs.len() > 3 {
                eprintln!("      engine-dto:  (+{} more)", dto_locs.len() - 3);
            }
            for loc in port_locs.iter().take(3) {
                eprintln!("      engine-ports:{loc}");
            }
            if port_locs.len() > 3 {
                eprintln!("      engine-ports:(+{} more)", port_locs.len() - 3);
            }
        }

        eprintln!();
        anyhow::bail!("arch-check failed: engine-dto shadows engine-ports types");
    }

    Ok(())
}

fn collect_public_type_names(
    dir: &std::path::Path,
    pub_type_re: &regex_lite::Regex,
) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();

    for entry in walkdir_rs_files(dir)? {
        let contents = std::fs::read_to_string(&entry)
            .with_context(|| format!("reading {}", entry.display()))?;

        for cap in pub_type_re.captures_iter(&contents) {
            let Some(name) = cap.get(1).map(|m| m.as_str()) else {
                continue;
            };

            out.entry(name.to_string())
                .or_default()
                .push(entry.display().to_string());
        }
    }

    Ok(out)
}

/// Check that application layer code does not depend on *inbound* ports.
///
/// Target architecture rule:
/// - Inbound ports are what the application offers; they are implemented by app use cases.
/// - Outbound ports are what the application needs; they are depended on by app code.
///
/// Therefore, application code should not import traits from `*-ports/src/inbound/`.
///
/// This is a **future-architecture** enforcement check. It intentionally focuses on
/// import patterns (cheap + stable) rather than full semantic analysis.
fn check_app_does_not_depend_on_inbound_ports() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    // We only enforce within application-internal code.
    // Handlers are boundary code and are expected to call inbound ports.
    // Use cases implement inbound ports and will naturally reference inbound traits.
    // So for now, we restrict this check to *services* only.
    let enforced_dirs = [
        workspace_root.join("crates/engine-app/src/application/services"),
        workspace_root.join("crates/player-app/src/application/services"),
    ];

    // Import patterns to flag:
    //   use wrldbldr_engine_ports::inbound::...
    //   use wrldbldr_player_ports::inbound::...
    // and the fully-qualified path usage variants.
    let engine_inbound_use_re = regex_lite::Regex::new(r"\buse\s+wrldbldr_engine_ports::inbound::")
        .context("compiling engine inbound port import regex")?;
    let player_inbound_use_re = regex_lite::Regex::new(r"\buse\s+wrldbldr_player_ports::inbound::")
        .context("compiling player inbound port import regex")?;

    let engine_inbound_fqn_re = regex_lite::Regex::new(r"\bwrldbldr_engine_ports::inbound::")
        .context("compiling engine inbound port fqn regex")?;
    let player_inbound_fqn_re = regex_lite::Regex::new(r"\bwrldbldr_player_ports::inbound::")
        .context("compiling player inbound port fqn regex")?;

    let mut violations: Vec<String> = Vec::new();

    for dir in enforced_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in walkdir_rs_files(&dir)? {
            let contents = std::fs::read_to_string(&entry)
                .with_context(|| format!("reading {}", entry.display()))?;

            // Skip internal/ re-export modules - they're allowed to re-export inbound ports
            // for use by app-layer code (maintaining single source of truth while keeping
            // app imports from internal::).
            let path_str = entry.display().to_string();
            if path_str.contains("/internal/") && !path_str.ends_with("mod.rs") {
                // Check if this is a pure re-export module (only pub use statements)
                let non_comment_lines: Vec<&str> = contents
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        !t.is_empty()
                            && !t.starts_with("//")
                            && !t.starts_with("/*")
                            && !t.starts_with("*")
                    })
                    .collect();
                let is_reexport_only = non_comment_lines
                    .iter()
                    .all(|l| l.trim().starts_with("pub use") || l.trim().starts_with("#[cfg"));
                if is_reexport_only {
                    continue;
                }
            }

            // Scan lines so we can ignore comments and report an actionable location.
            for (line_idx, line) in contents.lines().enumerate() {
                let trimmed = line.trim();

                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                {
                    continue;
                }

                if engine_inbound_use_re.is_match(line)
                    || player_inbound_use_re.is_match(line)
                    || engine_inbound_fqn_re.is_match(line)
                    || player_inbound_fqn_re.is_match(line)
                {
                    violations.push(format!(
                        "{}:{}: application code depends on inbound ports (use outbound ports instead)\n    {}",
                        entry.display(),
                        line_idx + 1,
                        trimmed
                    ));
                    break;
                }
            }
        }
    }

    if !violations.is_empty() {
        println!(
            "Inbound-port dependency violations ({} files):",
            violations.len()
        );
        println!("  Architecture rule: application layer must not depend on inbound ports");
        println!("  Fix: move the dependency to an outbound port / DTO, or push it to the boundary layer");
        println!();
        for v in &violations {
            println!("  - {v}");
        }

        println!();
        anyhow::bail!("arch-check failed: application code depends on inbound ports");
    }

    Ok(())
}

fn check_no_cross_crate_shims() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    // Enforce across the workspace.
    // (Owning crates may legitimately `pub use` their own internals; this check only
    // targets cross-crate re-export/alias patterns like `pub use wrldbldr_*::...`.)
    let enforced_dirs = [
        workspace_root.join("crates/domain/src"),
        workspace_root.join("crates/protocol/src"),
        workspace_root.join("crates/engine/src"),
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

            if metadata.is_file() && entry_path.extension().and_then(|s| s.to_str()) == Some("rs") {
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

/// Check that engine use cases don't import protocol message enums (ServerMessage, ClientMessage)
///
/// Use cases in crates/engine/src/use_cases/ should return domain-centric results,
/// not wire-format protocol messages.
fn check_use_case_layer() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let use_cases_dir = workspace_root.join("crates/engine/src/use_cases");

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
    let exempt_files: HashSet<&str> = ["mod.rs"].into_iter().collect();

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

/// Check that engine internal layers don't import `wrldbldr_protocol` directly.
///
/// The protocol crate is a wire format; only the API boundary should use it.
/// Concretely: forbid `wrldbldr_protocol` usage in `crates/engine/src/{entities,use_cases,infrastructure}`.
fn check_engine_protocol_isolation() -> anyhow::Result<()> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("finding workspace root")?;

    let engine_src = workspace_root.join("crates/engine/src");
    if !engine_src.exists() {
        return Ok(());
    }

    // Directories to check within engine/src/ (API boundary is exempt)
    let check_dirs = ["entities", "use_cases", "infrastructure"];

    // Patterns that indicate protocol usage
    let use_protocol_re = regex_lite::Regex::new(r"use\s+wrldbldr_protocol::")?;
    let fqn_protocol_re = regex_lite::Regex::new(r"wrldbldr_protocol::")?;

    let mut violations = Vec::new();

    for dir_name in check_dirs {
        let dir = engine_src.join(dir_name);

        if !dir.exists() {
            continue;
        }

        for entry in walkdir_rs_files(&dir)? {
            let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Exempt module declarations only.
            if file_name == "mod.rs" {
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
        eprintln!("Engine protocol isolation violations:");
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!("arch-check failed: engine internal layers import protocol types");
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
        let path_str = entry.to_string_lossy();

        // Exempt files and directories:
        // - request_handler.rs: Documented API boundary - uses RequestPayload/ResponseResult
        // - dm_approval_queue_service_port.rs: Re-exports wire-format types from protocol
        //   (ProposedToolInfo, ChallengeSuggestionInfo, etc.) - protocol is single source of truth
        // - queue_types.rs: Queue payload types that use protocol wire-format types
        //   (moved from domain/value_objects/queue_data.rs - Phase 1A.1)
        // - mod.rs: Re-exports protocol types for API compatibility
        // - dto/ directory: Merged from engine-dto crate - contains boundary DTOs that use protocol types
        if file_name == "request_handler.rs"
            || file_name == "dm_approval_queue_service_port.rs"
            || file_name == "queue_types.rs"
            || file_name == "mod.rs"
            || path_str.contains("/dto/")
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
        "request_port.rs",         // RequestPort trait uses RequestPayload/ResponseResult
        "game_connection_port.rs", // WebSocket connection uses protocol message types
        "mock_game_connection.rs", // Testing infrastructure mirrors connection port
        "player_events.rs", // Re-exports wire-format types from protocol (single source of truth)
        "session_types.rs", // Ports-layer types with bidirectional protocol conversions
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
        eprintln!(
            "  Shared Kernel pattern: Only designated boundary files may use protocol types."
        );
        eprintln!("  Whitelisted files: request_port.rs, game_connection_port.rs, mock_game_connection.rs");
        eprintln!();
        for v in &violations {
            eprintln!("  - {v}");
        }
        anyhow::bail!(
            "arch-check failed: player-ports imports protocol types in non-Shared-Kernel files"
        );
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
    // Note: domain-types, common, engine-dto, engine-composition have been merged
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
        // Enforcement mode: fail on any violation
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
        anyhow::bail!("arch-check failed: glob re-exports found");
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
    //
    // Architecture Simplification (2026-01):
    // Engine is now a single crate (`wrldbldr-engine`).
    // Player is still split (ports/app/adapters/ui/runner) for now.
    HashMap::from([
        // Domain layer: entities, value objects, types, common utilities
        // Zero internal dependencies (innermost layer)
        ("wrldbldr-domain", HashSet::from([])),
        // Protocol (API contract) depends on domain for shared vocabulary
        ("wrldbldr-protocol", HashSet::from(["wrldbldr-domain"])),
        // Engine is monolithic (entities/use_cases/infrastructure/api) and only
        // depends on domain + protocol.
        (
            "wrldbldr-engine",
            HashSet::from(["wrldbldr-domain", "wrldbldr-protocol"]),
        ),
        (
            "wrldbldr-player-ports",
            HashSet::from(["wrldbldr-domain", "wrldbldr-protocol"]),
        ),
        // Application layer: services, use cases, handlers (player-side)
        (
            "wrldbldr-player-app",
            HashSet::from([
                "wrldbldr-domain",
                "wrldbldr-protocol",
                "wrldbldr-player-ports",
                "wrldbldr-player-adapters", // dev-dependency for MockGameConnectionPort
            ]),
        ),
        // Adapters layer: infrastructure implementations (player-side)
        (
            "wrldbldr-player-adapters",
            HashSet::from([
                "wrldbldr-player-app",
                "wrldbldr-player-ports",
                "wrldbldr-protocol",
                "wrldbldr-domain",
            ]),
        ),
        // Presentation layer
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
        // Runner layer: composition roots
        (
            "wrldbldr-player-runner",
            HashSet::from([
                "wrldbldr-player-ui",
                "wrldbldr-player-app",
                "wrldbldr-player-ports",
                "wrldbldr-player-adapters",
            ]),
        ),
        ("xtask", HashSet::from([])),
    ])
}
