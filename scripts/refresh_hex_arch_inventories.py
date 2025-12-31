#!/usr/bin/env python3
"""Regenerate inventory blocks in the master hex architecture refactor plan.

This script replaces ONLY the content between these paired markers in
`docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md`:

- %% PORT TAXONOMY %% ... %% /PORT TAXONOMY %%
- %% DTO OWNERSHIP %% ... %% /DTO OWNERSHIP %%

Markers are preserved so the inventories can be regenerated repeatedly.

Usage:
  python3 scripts/refresh_hex_arch_inventories.py
  python3 scripts/refresh_hex_arch_inventories.py --check
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple


WORKSPACE_ROOT = Path(__file__).resolve().parents[1]
MASTER_PLAN = WORKSPACE_ROOT / "docs/plans/HEXAGONAL_ARCHITECTURE_REFACTOR_MASTER_PLAN.md"


@dataclass(frozen=True)
class PubItem:
    name: str
    kind: str  # trait|struct|enum|type
    rel_path: str


PUB_DECL_RE = re.compile(
    r"(?m)^\s*pub\s+(trait|struct|enum|type)\s+([A-Za-z_][A-Za-z0-9_]*)\b"
)


def iter_rs_files(root: Path) -> Iterable[Path]:
    if not root.exists():
        return
    for p in root.rglob("*.rs"):
        if p.is_file():
            yield p


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_text(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")


def rel(p: Path) -> str:
    return p.resolve().relative_to(WORKSPACE_ROOT).as_posix()


def collect_public_items(dir_path: Path) -> List[PubItem]:
    items: List[PubItem] = []
    for file_path in iter_rs_files(dir_path):
        contents = read_text(file_path)
        for m in PUB_DECL_RE.finditer(contents):
            kind = m.group(1)
            name = m.group(2)
            items.append(PubItem(name=name, kind=kind, rel_path=rel(file_path)))
    return items


INBOUND_PATH_USE_RE = re.compile(
    r"wrldbldr_(engine|player)_ports::inbound::([A-Za-z0-9_:{}\s,]+)"
)


def _extract_last_ident(segment: str) -> Optional[str]:
    # Handles:
    # - `foo::bar::Baz` -> Baz
    # - `{A, B, C}` -> A/B/C (handled elsewhere)
    segment = segment.strip()
    if not segment:
        return None
    if segment.startswith("{") and segment.endswith("}"):
        return None
    parts = [p for p in segment.split("::") if p]
    if not parts:
        return None
    last = parts[-1]
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", last):
        return last
    return None


def scan_inbound_usages(search_roots: Sequence[Path]) -> Dict[str, List[str]]:
    """Map item name -> list of file paths where inbound::Item is referenced."""

    out: Dict[str, List[str]] = {}

    for root in search_roots:
        for file_path in iter_rs_files(root):
            contents = read_text(file_path)
            for m in INBOUND_PATH_USE_RE.finditer(contents):
                tail = m.group(2)

                # Handle `inbound::{A, B}` patterns.
                if tail.strip().startswith("{"):
                    brace = tail.strip()
                    # best-effort: take until closing brace on same match
                    brace = brace.split("}", 1)[0].lstrip("{")
                    for name in [n.strip() for n in brace.split(",")]:
                        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
                            out.setdefault(name, []).append(rel(file_path))
                    continue

                # Otherwise it may be a module path like `staging::StagingUseCasePort`.
                # Extract last identifier.
                last = _extract_last_ident(tail)
                if last is not None:
                    out.setdefault(last, []).append(rel(file_path))

    # Stable ordering of locations
    for name in list(out.keys()):
        out[name] = sorted(set(out[name]))
    return out


def is_boundary_path(path_str: str) -> bool:
    # Heuristic only; tuned for current tree.
    return any(
        token in path_str
        for token in (
            "/application/handlers/",
            "/application/request_handlers/",
            "/application/requests/",
            "/presentation/",
            "/routes/",
        )
    )


def suggest_port_target(item_name: str, usage_paths: List[str]) -> str:
    # Simple naming heuristics + usage location.
    if item_name in {"RequestContext", "UseCaseContext"}:
        return "inbound (boundary DTO)"
    if item_name in {"RequestHandler"}:
        return "inbound (boundary trait)"
    if item_name.endswith("UseCasePort"):
        return "inbound"

    if usage_paths:
        boundary_only = all(is_boundary_path(p) for p in usage_paths)
        if not boundary_only:
            return "outbound (misplaced today)"

    return "inbound"


def render_port_taxonomy_block() -> str:
    engine_inbound_dir = WORKSPACE_ROOT / "crates/engine-ports/src/inbound"
    player_inbound_dir = WORKSPACE_ROOT / "crates/player-ports/src/inbound"

    engine_items = collect_public_items(engine_inbound_dir)
    player_items = collect_public_items(player_inbound_dir)

    engine_usage = scan_inbound_usages([WORKSPACE_ROOT / "crates/engine-app/src"])
    player_usage = scan_inbound_usages(
        [
            WORKSPACE_ROOT / "crates/player-app/src",
            WORKSPACE_ROOT / "crates/player-ui/src",
        ]
    )

    # Group by name -> (kind, defining file)
    def index_defs(items: List[PubItem]) -> Dict[str, PubItem]:
        indexed: Dict[str, PubItem] = {}
        for it in sorted(items, key=lambda x: (x.name, x.rel_path)):
            # Keep the first definition location for stable output
            indexed.setdefault(it.name, it)
        return indexed

    engine_defs = index_defs(engine_items)
    player_defs = index_defs(player_items)

    def render_table(defs: Dict[str, PubItem], usage: Dict[str, List[str]]) -> str:
        rows: List[Tuple[str, str, str, str, str]] = []

        for name in sorted(defs.keys()):
            it = defs[name]
            used = usage.get(name, [])
            used_disp = ""
            if used:
                sample = ", ".join(used[:3])
                extra = f" (+{len(used) - 3} more)" if len(used) > 3 else ""
                used_disp = f"{sample}{extra}"

            suggested = suggest_port_target(name, used)

            notes = ""
            if suggested.startswith("outbound"):
                notes = "Move to outbound; update app deps; remove inbound re-export"
            elif "boundary" in suggested:
                notes = "Keep in inbound; ensure it does not leak into services"
            else:
                notes = "Keep in inbound; ensure only adapters/UI import"

            rows.append((name, it.kind, it.rel_path, used_disp, suggested, notes))

        header = "| Item | Kind | Defined in | Used in app/UI (examples) | Suggested target | Notes |\n"
        header += "|---|---|---|---|---|---|\n"
        body = "".join(
            f"| `{r[0]}` | {r[1]} | {r[2]} | {r[3]} | {r[4]} | {r[5]} |\n" for r in rows
        )
        return header + body

    out_lines: List[str] = []

    out_lines.append("#### Engine inbound taxonomy")
    out_lines.append("")
    if engine_defs:
        out_lines.append(render_table(engine_defs, engine_usage).rstrip())
    else:
        out_lines.append("_No engine inbound items found (unexpected)._ ")

    out_lines.append("")
    out_lines.append("#### Player inbound taxonomy")
    out_lines.append("")
    if player_defs:
        out_lines.append(render_table(player_defs, player_usage).rstrip())
    else:
        out_lines.append("_No player inbound items found (unexpected)._ ")

    out_lines.append("")
    out_lines.append("_Regenerate with `task arch:inventories`._")

    return "\n".join(out_lines).strip() + "\n"


def collect_public_type_names(dir_path: Path) -> Dict[str, List[str]]:
    # Note: intentionally mirrors xtask heuristic.
    out: Dict[str, List[str]] = {}
    for file_path in iter_rs_files(dir_path):
        contents = read_text(file_path)
        for m in re.finditer(
            r"(?m)^\s*pub\s+(?:struct|enum|type)\s+([A-Za-z_][A-Za-z0-9_]*)\b",
            contents,
        ):
            name = m.group(1)
            out.setdefault(name, []).append(rel(file_path))

    for name in list(out.keys()):
        out[name] = sorted(set(out[name]))
    return out


def suggest_dto_owner(type_name: str) -> str:
    # Heuristic; final owner is a design decision.
    if "Queue" in type_name or type_name in {"LlmResponse"}:
        return "engine-dto (likely internal glue)"
    if "Proposal" in type_name or type_name in {"ApprovalItem"}:
        return "engine-ports (likely boundary DTO)"
    return "TBD"


def render_dto_ownership_block() -> str:
    engine_dto_src = WORKSPACE_ROOT / "crates/engine-dto/src"
    engine_ports_src = WORKSPACE_ROOT / "crates/engine-ports/src"

    dto_types = collect_public_type_names(engine_dto_src)
    port_types = collect_public_type_names(engine_ports_src)

    collisions = sorted(set(dto_types.keys()) & set(port_types.keys()))

    lines: List[str] = []
    lines.append("| Type name | engine-dto (examples) | engine-ports (examples) | Suggested owner | Notes |")
    lines.append("|---|---|---|---|---|")

    for name in collisions:
        dto_locs = dto_types.get(name, [])
        port_locs = port_types.get(name, [])

        dto_disp = ", ".join(dto_locs[:2]) + (f" (+{len(dto_locs) - 2} more)" if len(dto_locs) > 2 else "")
        port_disp = ", ".join(port_locs[:2]) + (f" (+{len(port_locs) - 2} more)" if len(port_locs) > 2 else "")

        owner = suggest_dto_owner(name)
        notes = "Pick one canonical definition; delete the shadow copy; migrate imports"
        lines.append(f"| `{name}` | {dto_disp} | {port_disp} | {owner} | {notes} |")

    if not collisions:
        lines.append("| _(none)_ |  |  |  |  |")

    lines.append("")
    lines.append("_Regenerate with `task arch:inventories`._")

    return "\n".join(lines).strip() + "\n"


def replace_block(text: str, start_marker: str, end_marker: str, replacement: str) -> str:
    start = f"%% {start_marker} %%"
    end = f"%% /{end_marker} %%"

    pattern = re.compile(
        rf"(?ms)^{re.escape(start)}\s*\n.*?^\s*{re.escape(end)}\s*$"
    )

    def repl(match: re.Match) -> str:
        # Keep markers exactly as-is, replace interior.
        return f"{start}\n\n{replacement.rstrip()}\n\n{end}"

    new_text, n = pattern.subn(repl, text, count=1)
    if n != 1:
        raise RuntimeError(
            f"Expected exactly 1 replace for markers {start}..{end}, got {n}"
        )
    return new_text


def main(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="Do not write files; exit non-zero if changes would be made",
    )
    args = parser.parse_args(list(argv))

    original = read_text(MASTER_PLAN)

    port_block = render_port_taxonomy_block()
    dto_block = render_dto_ownership_block()

    updated = original
    updated = replace_block(updated, "PORT TAXONOMY", "PORT TAXONOMY", port_block)
    updated = replace_block(updated, "DTO OWNERSHIP", "DTO OWNERSHIP", dto_block)

    if updated == original:
        print("Inventories already up to date")
        return 0

    if args.check:
        print("Inventories out of date (run `task arch:inventories`)")
        return 1

    write_text(MASTER_PLAN, updated)
    print("Updated inventories in:", rel(MASTER_PLAN))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
