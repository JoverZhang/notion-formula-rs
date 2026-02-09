# <module name>

Copy into a module/crate as `README.md` and fill in. Keep it short. Point to code and tests.

## Purpose

- What this module owns.
- What it does not own.

## Public API

- Entry points: `<fn/type>` → `<what it returns/does>`

## Contracts / invariants

- Spans/offsets: `<bytes|utf16>`, `[start, end)`, clamping/flooring rules (if any).
- Determinism rules (sorting, deconfliction, formatting).

## Layout

| Path | What lives here |
|---|---|
| `./<path>` | <summary> |

## Flow

```text
<input> -> <step> -> <output>
```

## Tests

- `<command>` (what it covers)

## TODOs

- `<short item>` (file/test pointers)
