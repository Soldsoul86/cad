# cad (working name)

Open-source 2D drafting app in Rust: a standalone DXF library (`crates/cad-dxf`) plus a drafting app built on it. See [docs/SPEC.md](docs/SPEC.md) for the plan and [CHANGELOG.md](CHANGELOG.md) for progress.

```sh
cargo test --workspace
cargo run -p cad-app -- tests/corpus/ac1015_floor_plan.dxf
```

The corpus in `tests/corpus/` is generated: hand-written files by `tools/gen_corpus.py`, the rest by `tools/gen_sources.py` (ezdxf) and re-saved by other CAD tools with `tools/convert_corpus.sh` — see [SOURCES.md](tests/corpus/SOURCES.md) and [COVERAGE.md](tests/corpus/COVERAGE.md). Put DXF files you can't redistribute in `tests/corpus-private/` (git-ignored); the tests load them too.

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
