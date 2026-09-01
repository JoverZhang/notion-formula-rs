---
doc_id: intent.project
title: "Project intent"
language: en
source_language: zh-CN
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Project intent

[简体中文](README.zh-CN.md)

## Why I am building this project

AI can already write code and SQL directly. I do not know how long Formula will remain relevant as a way to interact with software. I still want to try building a Formula engine around static type inference and columnar evaluation. I would be honored if it happens to solve a problem for you.

I believe that as custom tools become cheaper to build, more people will create DSLs for expressing their own business rules and development constraints. That belief does not depend on Formula remaining popular.

`notion-formula-rs` already uses an internal DSL to declare builtin functions, backed by a lightweight compiler written for that DSL. I want to design more DSLs for behavior, specifications, and implementation constraints, and experiment with BDD-like practices. DSL-driven development remains an idea for now.

Even if this Formula engine does not help you, I hope the project's DSL practices still offer something useful: how to design a small language for a specific problem, then use it to declare rules, generate code, and constrain an implementation.

## What I want it to become

`notion-formula-rs` provides a Formula engine for multidimensional table systems such as Notion Database. The goal is to run the same Formula on the server and in the browser. Server-side execution processes batches of data with columnar evaluation. Browser-side execution analyzes and evaluates as the user edits, so errors, types, and results appear promptly. Both environments use the same syntax and evaluation rules.

## Project boundaries

Notion Formula is where this project starts. The project aims to match its syntax and evaluation behavior wherever practical. Static type inference checks types before a Formula runs and produces as precise a result type as possible.

The host system defines and stores non-Formula fields and supplies their values. `notion-formula-rs` accepts their types and values so a Formula can reference them. The project does not implement the other capabilities of Notion Database.

## A personal motivation

The project's IDE- and LSP-related work also comes from a personal wish: to study how these technologies work while there is still some warmth left in them, and to satisfy a curiosity I have carried since my youth.
