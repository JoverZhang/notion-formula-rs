# notion-formula-rs

A Rust Formula engine for analyzing, editing, and evaluating Notion-style formulas.

- [Live demo](https://joverzhang.github.io/notion-formula-rs/)
- [Documentation](docs/README.md)
- [简体中文](README.zh-CN.md)

![The browser demo showing completion suggestions, signature help, and inferred result types](docs/assets/browser-demo.webp)

## What it does

- Analyzes incomplete formula source and returns diagnostics and inferred result types.
- Provides completion, signature help, deterministic formatting, and quick fixes for formula
  editors.
- Evaluates saved formulas and their shared dependencies over batches of rows.
- Exposes Engine and Draft operations to browser integrations through WebAssembly and a Worker.

## Current scope

The project starts from Notion-style formula syntax but does not promise complete compatibility.
The browser demo analyzes editable Drafts and displays batch results from saved definitions.
Save applies a Draft to the Engine; Discard restores the saved expression. The
[current specifications](docs/specs/README.md) define the supported syntax and behavior.

## Explore the project

- [Project intent](docs/intent/README.md)
- [Current specifications](docs/specs/README.md)

## License

Licensed under the [Apache License 2.0](LICENSE).
