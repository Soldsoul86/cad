# cad (working name)

Open-source 2D drafting app in Rust: a standalone DXF library (`crates/cad-dxf`) plus a drafting app built on it. See [docs/SPEC.md](docs/SPEC.md) for the plan and [CHANGELOG.md](CHANGELOG.md) for progress.

```sh
cargo test --workspace
cargo run -p cad-app -- tests/corpus/ac1015_floor_plan.dxf
```
