# Remap

Remap is a small, robust Rust library for efficiently reordering and transforming row-based data,
such as [CSV](https://en.wikipedia.org/wiki/Comma-separated_values) records.
It provides a clean, type-safe API for defining column mappings and applying them to data rows.

The library is built around two core concepts that separate the transformation logic from the data itself,
leading to reusable and predictable operations.

## Core Concepts

`Row`:
A simple data container representing a single record or row of data.
It is essentially a wrapper around a `Vec<String>`.

`Map`:
Defines how to transform a `Row`.
A `Map` is a vector where the index represents the source column and the value (`Some(dest)`) represents the destination column.
A `None` value indicates that the source column should be dropped.

# TODOs

- [x] Add `#[inline]` hints.
- [ ] Add `# Quick Start` to `README.md`.
- [x] Add methods with `fallback` argument.
- [x] Add examples to docstrings.
- [ ] Add `CSV` struct (Wrapper around `Vec<Row>`).
- [x] Refactor `Row` using generics.
- [ ] Add `MapBuilder` struct to increase usability (stores `HasMap`, simply add `(src, dest)` pairings).
- [ ] Add support for `Map` instances from a file.
- [ ] Add file import/export.
- [ ] Name based reordering.
- [x] Add reordering without cloning.
