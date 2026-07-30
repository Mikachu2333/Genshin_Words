# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

`genshin_pinyin_add` converts lines of Chinese text (a Genshin Impact dictionary) into all possible pinyin combinations, outputting tab-separated entries for the fcitx5 Linux input method framework.

Part of a larger `Genshin_Words` monorepo that processes a `yuanshen.dict.yaml` dictionary file through a multi-step pipeline.

## Build & run

```bash
# Build
cargo build                  # debug build
cargo build --release        # release build (stripped, LTO, panic=abort)

# Run (release mode — requires a file path argument)
./genshin_pinyin_add ./yuanshen.dict.yaml

# Run (debug mode — uses hardcoded path, no args needed)
cargo run
```

There are no tests in this crate.

## Architecture

The program does one thing:

1. **Parse YAML frontmatter** — `delete_useless()` strips everything between the first `---` line and the second `---`/`...` line of the input YAML dictionary, keeping only the content lines (one Chinese word per line).

2. **Generate pinyin combinations** — For each content line, every character is converted to its pinyin readings via the `pinyin` crate's `ToPinyinMulti` trait. Since Chinese characters can have multiple readings (多音字), all combinations are Cartesian-product expanded.

3. **Output fcitx5 format** — Each combination is written as `{word}\t{pinyin}\t100` to `yuanshen_fcitx5.txt`, where pinyin syllables are separated by `'`. Manually annotated Rime entries use spaces between syllables and are converted to apostrophes only in this output.

## Key implementation details

- **Dependencies**: `pinyin = "0.11.0"` provides readings and `tempfile` supports atomic output replacement.
- **CLI behavior**: The input file path is always passed as the sole command-line argument.
- **Multi-character pinyin expansion**: For each character, `to_pinyin_multi()` returns all possible readings; duplicates are removed and sorted. The Cartesian product of per-character readings produces all legal pinyin strings for the word.
- **Zero-copy pinyin strings**: The code borrows `&str` from the pinyin data throughout; no allocation for individual syllable strings.

## Monorepo context

This crate is one step in a processing pipeline under `Genshin_Words/`:

| Step | Tool | Purpose |
|------|------|---------|
| 1 | `delete_repeat.py` | Remove duplicate lines from input |
| 2 | `merge.py` | Append new content to `yuanshen.dict.yaml`, then invoke steps 3 & 4 |
| 3 | `genshin_pinyin_sort/` | Sort the dictionary entries |
| 4 | `genshin_pinyin_add/` | Generate pinyin+fctix5 output |

The sibling crate `genshin_pinyin_sort/` is a separate Rust project that sorts the YAML dictionary. The Python scripts in the repo root orchestrate the full pipeline.

## Release process

Pushing a tag triggers `.github/workflows/release.yml`, which creates a GitHub Release attaching `yuanshen.dict.yaml` and `yuanshen_fcitx5.txt`. Manual re-release is supported via `workflow_dispatch`.
