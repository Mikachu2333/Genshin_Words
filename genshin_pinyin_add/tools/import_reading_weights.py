"""Extract a pinned rime-ice character weight table using only the standard library."""

import argparse
import hashlib
from pathlib import Path
import re


def extract(source: bytes) -> dict[str, dict[str, int]]:
    result: dict[str, dict[str, int]] = {}
    for number, line in enumerate(source.decode("utf-8-sig").splitlines(), 1):
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 3 or len(fields[0]) != 1:
            continue
        character, reading, weight = fields
        if not re.fullmatch(r"[a-z]+", reading) or not weight.isdecimal():
            raise ValueError(f"invalid character reading on source line {number}: {line!r}")
        value = int(weight)
        if value > 0xFFFFFFFF:
            raise ValueError(f"weight exceeds the u32 range on source line {number}")
        if value <= 0:
            continue
        readings = result.setdefault(character, {})
        if reading in readings:
            raise ValueError(f"duplicate reading: {character} {reading}")
        readings[reading] = value
    if not result:
        raise ValueError("no weighted character readings found")
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="cn_dicts/8105.dict.yaml from rime-ice")
    parser.add_argument("--revision", required=True, help="full 40-character source commit")
    parser.add_argument("--output", type=Path,
                        default=Path(__file__).resolve().parents[1] / "data/pinyin_reading_weights.tsv")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision):
        parser.error("--revision must be a full lowercase Git commit hash")
    source = args.source.read_bytes()
    readings = extract(source)
    output = [
        "# Character pinyin weights, sorted by weight within each character.",
        f"# Source: https://github.com/iDvel/rime-ice/blob/{args.revision}/cn_dicts/8105.dict.yaml",
        f"# Source SHA-256: {hashlib.sha256(source).hexdigest()}",
        "# License: GPL-3.0; see LICENSE.rime-ice.",
        "# Fields: character<TAB>toneless pinyin<TAB>weight",
    ]
    rows = 0
    for character, values in sorted(readings.items()):
        for reading, weight in sorted(values.items(), key=lambda value: (-value[1], value[0])):
            output.append(f"{character}\t{reading}\t{weight}")
            rows += 1
    args.output.write_text("\n".join(output) + "\n", encoding="utf-8", newline="\n")
    print(f"Imported {len(readings)} characters and {rows} readings into {args.output}")


if __name__ == "__main__":
    main()
