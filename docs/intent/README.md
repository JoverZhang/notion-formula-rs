---
doc_id: intent.project
title: "Project intent"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Project intent

[简体中文](README.zh-CN.md)

## Why this project exists

Interactive formula experiences need syntax feedback, editor assistance, and evaluation to agree on what a formula means. `notion-formula-rs` brings those capabilities together around one shared definition of the formula language so an integration does not have to reconcile separate parsers, type rules, and runtimes.

The project favors deterministic, inspectable behavior and explicit support boundaries. Notion-style formulas provide the starting vocabulary, but correctness within the documented surface matters more than claiming complete parity with another product.

## Who it serves

The project serves teams that embed formula authoring in an editor or application, including browser integrations that use the JavaScript/WASM boundary. It also serves people who write and evaluate formulas inside those integrations.

## Goals and non-goals

The goals are to provide one coherent formula language, useful analysis while a formula is being written, predictable editor services, and evaluation of supported formulas.

The project is not a standalone end-user product. It does not own formula storage, persistent formula identity, rename transactions, deployment, or operations. Complete Notion compatibility is not a goal, and Rust `pub` items are not automatically stable integration contracts.

## Boundaries and reevaluation

The current boundary treats formula behavior and the JavaScript/WASM integration surface as user-facing commitments, while Rust implementation interfaces may evolve with the workspace.

Reevaluate this boundary if the project must promise a stable Rust API, manage formula identity or persistence, guarantee strict compatibility with an upstream language, or become an independently operated product. Those changes would introduce new users and obligations rather than merely extend the current implementation.
