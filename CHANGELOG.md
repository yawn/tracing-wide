# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Pre-1.0, a breaking change bumps the minor version.

## [Unreleased]

## [0.2.0] - 2026-06-29

### Added

- `facet` feature: the `Message::as_facet` reflection hook, opt-in per type via
  `#[derive(Facet)]`, returning a `facet::Peek` over the live message body so a
  subscriber can read fields by name. Independent of `serde`: a message may derive
  either, both, or neither, and a single subscriber can react to all of them.
- `facet` re-exported as `tracing_wide::facet`, so subscribers name `Peek`/`Facet`
  through the exact version the crate compiled against. To derive `Facet`, depend on
  `facet` directly at that version or use `#[facet(crate = tracing_wide::facet)]`.
- Catalogue descriptors derive `Facet` under the `facet` feature, so a manifest can
  be produced with any facet serializer: `level`, `origin`, and the `meta` maps
  render identically to the serde output. New example `catalogue-facet`.
- Example `subscriber-facet`, filtering on a message field's value via reflection.
- Thread-local scoped subscribers: `Subscribers::set_default` (returning a
  `DefaultGuard`) and `with_default`, mirroring `tracing`'s own scoped-dispatch
  API. While a guard is held, the calling thread fans out to that set instead of
  the globally `install`ed one - useful for tests and per-thread routing.

### Changed

- **Breaking:** `catalogue::MessageDescriptor::level` is now `catalogue::LevelName`
  (an enum mirroring `tracing::Level`) rather than `tracing::Level`; recover the
  level with `Level::from(..)`.
- Renamed examples: `catalogue` to `catalogue-serde`, `serializable` to `subscriber-serde`.

## [0.1.0] - 2026-06-13

Initial release.

[Unreleased]: https://github.com/yawn/tracing-wide/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/yawn/tracing-wide/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/yawn/tracing-wide/releases/tag/v0.1.0
