# Changelog

## v0.2.0 — 2026-10-18 (final release)

This is the final release of Rocket Tracing Fairing Example. The project is discontinued and archived because development of the original Rocket upstream has stalled. Community continuation efforts are discussed [upstream](https://github.com/rwf2/Rocket/discussions/2940).

No further maintenance, dependency updates, bug fixes, or security fixes are planned. The source remains available for historical reference.

Changes included in this final snapshot:

- Migrate logging to the Yansi 1 API and update Yansi to 1.0.1.
- Update checkout, artifact upload, Codecov, and Rust cache actions.
- Replace the obsolete Tarpaulin binary with LLVM coverage and archive LCOV reports.
- Add tests for preserving supplied request IDs and generating missing request IDs.
- Add a prominent discontinuation notice.

Validation: formatting, Clippy with warnings denied, request tracing tests, and hosted CI.
