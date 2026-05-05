#!/usr/bin/env python3
"""
Update models.rs field optionality based on the OpenAPI spec.

Reads the OpenAPI spec, resolves required vs optional fields using the same
logic as resolve-field-requirements.py, then rewrites models.rs so that
required non-nullable fields use bare types (T) instead of Option<T>.

Usage:
    python scripts/update-models-optionality.py              # update in-place
    python scripts/update-models-optionality.py --dry-run     # show diff without writing
"""

import argparse
import re
import sys
from pathlib import Path

import importlib.util

# Import the resolution logic from sibling script
_script_dir = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location(
    "resolve_field_requirements", _script_dir / "resolve-field-requirements.py"
)
_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_mod)
build_manifest = _mod.build_manifest
SPEC_PATH = _mod.SPEC_PATH

REPO_ROOT = Path(__file__).resolve().parent.parent
MODELS_RS = REPO_ROOT / "crates" / "clickhouse-cloud-api" / "src" / "models.rs"

# Regex to match a serde rename attribute value
RE_RENAME = re.compile(r'rename\s*=\s*"([^"]+)"')
# Regex to match a field line: `pub field_name: Type,` (supports r#ident raw identifiers)
RE_FIELD = re.compile(r'^(\s*)pub\s+((?:r#)?\w+)\s*:\s*(.+?),\s*$')
# Regex to match an Option<...> type
RE_OPTION = re.compile(r'^Option<(.+)>$')


def pascalize(name: str) -> str:
    """Convert to PascalCase, matching the Rust test convention."""
    result = []
    upper_next = True
    for ch in name:
        if ch.isalnum():
            result.append(ch.upper() if upper_next else ch)
            upper_next = False
        else:
            upper_next = True
    return "".join(result)


def extract_serde_rename(serde_line: str) -> str | None:
    """Extract the rename value from a #[serde(...)] attribute line."""
    m = RE_RENAME.search(serde_line)
    return m.group(1) if m else None


def strip_option_attrs(serde_line: str) -> str | None:
    """Remove skip_serializing_if from a serde attribute line for required fields.

    Keeps `default` so deserialization is tolerant of partial/mock data.
    The T vs Option<T> type communicates the API contract; `default` keeps
    the client robust.

    Returns the modified line, or None if the serde attr becomes empty.
    """
    # Extract content between #[serde(...)]
    m = re.match(r'^(\s*)#\[serde\((.+)\)\]\s*$', serde_line)
    if not m:
        return serde_line

    indent = m.group(1)
    content = m.group(2)

    # Remove only skip_serializing_if = "Option::is_none" — keep rename and default
    parts = []
    for part in split_serde_attrs(content):
        part = part.strip()
        if part.startswith('skip_serializing_if'):
            continue
        if part:
            parts.append(part)

    if not parts:
        return None  # No attrs left, remove the whole line

    return f'{indent}#[serde({", ".join(parts)})]\n'


def split_serde_attrs(content: str) -> list[str]:
    """Split serde attribute content by commas, respecting quotes."""
    parts = []
    current = []
    in_quotes = False
    for ch in content:
        if ch == '"':
            in_quotes = not in_quotes
            current.append(ch)
        elif ch == ',' and not in_quotes:
            parts.append(''.join(current))
            current = []
        else:
            current.append(ch)
    if current:
        parts.append(''.join(current))
    return parts


def unwrap_option_type(type_str: str) -> str:
    """Remove the Option<> wrapper from a type string."""
    m = RE_OPTION.match(type_str.strip())
    if m:
        return m.group(1)
    return type_str


def update_models(models_text: str, manifest: dict) -> tuple[str, dict]:
    """Update models.rs text based on the manifest.

    Returns (updated_text, stats_dict).
    """
    # Build a lookup: PascalCase schema name -> set of required field spec names
    required_by_schema = {}
    for spec_name, info in manifest.items():
        pascal_name = pascalize(spec_name)
        required_by_schema[pascal_name] = set(info["required"])

    lines = models_text.split('\n')
    output = []
    stats = {"fields_made_required": 0, "fields_unchanged": 0, "structs_processed": 0}

    i = 0
    while i < len(lines):
        line = lines[i]

        # Detect struct start
        struct_match = re.match(r'^pub struct (\w+)\s*\{', line)
        if struct_match:
            struct_name = struct_match.group(1)
            required_fields = required_by_schema.get(struct_name, set())

            if required_fields:
                stats["structs_processed"] += 1

            output.append(line)
            i += 1

            # Process fields within this struct
            while i < len(lines):
                line = lines[i]

                # End of struct
                if line.strip() == '}':
                    output.append(line)
                    i += 1
                    break

                # Look for serde attribute + field pattern
                # A field may have 0 or 1 serde attribute lines before the pub line
                if line.strip().startswith('#[serde('):
                    serde_line = line
                    serde_idx = i
                    i += 1

                    # Next line should be the field
                    if i < len(lines):
                        field_line = lines[i]
                        field_match = RE_FIELD.match(field_line)

                        if field_match:
                            indent = field_match.group(1)
                            field_name = field_match.group(2)
                            field_type = field_match.group(3).strip()

                            # Determine the spec field name (strip r# prefix for Rust keywords)
                            spec_name = extract_serde_rename(serde_line) or field_name.removeprefix("r#")
                            is_option = RE_OPTION.match(field_type) is not None

                            if spec_name in required_fields and is_option:
                                # Make this field required
                                new_type = unwrap_option_type(field_type)
                                new_serde = strip_option_attrs(serde_line)

                                if new_serde is not None:
                                    output.append(new_serde.rstrip('\n'))
                                # else: serde line removed entirely

                                output.append(f'{indent}pub {field_name}: {new_type},')
                                stats["fields_made_required"] += 1
                                i += 1
                                continue
                            else:
                                stats["fields_unchanged"] += 1

                    # No transformation needed, output both lines as-is
                    output.append(serde_line)
                    continue  # Don't increment i, it was already incremented

                elif RE_FIELD.match(line):
                    # Field line without serde attr (already required or simple)
                    field_match = RE_FIELD.match(line)
                    field_name = field_match.group(2)
                    spec_name = field_name.removeprefix("r#")
                    field_type = field_match.group(3).strip()
                    is_option = RE_OPTION.match(field_type) is not None

                    if spec_name in required_fields and is_option:
                        # Bare field name matches spec name, make required
                        indent = field_match.group(1)
                        new_type = unwrap_option_type(field_type)
                        output.append(f'{indent}pub {field_name}: {new_type},')
                        stats["fields_made_required"] += 1
                        i += 1
                        continue

                    output.append(line)
                    i += 1
                else:
                    output.append(line)
                    i += 1
        else:
            output.append(line)
            i += 1

    return '\n'.join(output), stats


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show stats without writing changes",
    )
    parser.add_argument(
        "--spec",
        default=str(SPEC_PATH),
        help=f"Path to OpenAPI spec JSON (default: {SPEC_PATH})",
    )
    args = parser.parse_args()

    import json
    with open(args.spec) as f:
        spec = json.load(f)

    manifest = build_manifest(spec)
    models_text = MODELS_RS.read_text()

    updated_text, stats = update_models(models_text, manifest)

    print(f"Fields made required:  {stats['fields_made_required']}", file=sys.stderr)
    print(f"Fields unchanged:      {stats['fields_unchanged']}", file=sys.stderr)
    print(f"Structs processed:     {stats['structs_processed']}", file=sys.stderr)

    if args.dry_run:
        print("\nDry run — no changes written.", file=sys.stderr)
        return

    MODELS_RS.write_text(updated_text)
    print(f"\nUpdated {MODELS_RS}", file=sys.stderr)


if __name__ == "__main__":
    main()
