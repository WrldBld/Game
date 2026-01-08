#!/usr/bin/env python3

import re
from pathlib import Path


def parse_request_payload_variants(text: str) -> list[str]:
    start = text.find("pub enum RequestPayload")
    if start == -1:
        raise SystemExit("Could not find RequestPayload enum")

    body = text[start:]
    body = body[body.find("{") + 1 :]

    m = re.search(r"\n}\s*\n", body)
    if not m:
        raise SystemExit("Could not find end of RequestPayload enum")
    body = body[: m.start()]

    variants = re.findall(r"^\s{4}([A-Z][A-Za-z0-9_]*)\b", body, re.M)

    seen: set[str] = set()
    ordered: list[str] = []
    for v in variants:
        if v not in seen:
            seen.add(v)
            ordered.append(v)
    return ordered


def main() -> None:
    proto = Path("crates/protocol/src/requests.rs").read_text()
    ordered = parse_request_payload_variants(proto)

    engine_text = ""
    for p in Path("crates/engine/src/api").rglob("websocket*.rs"):
        engine_text += p.read_text() + "\n"

    missing = [v for v in ordered if f"RequestPayload::{v}" not in engine_text]

    print(f"Total variants: {len(ordered)}")
    print(f"Referenced in engine websocket code: {len(ordered) - len(missing)}")
    print(f"Possibly unimplemented (not referenced): {len(missing)}")
    print("---")
    for v in missing:
        print(v)


if __name__ == "__main__":
    main()
