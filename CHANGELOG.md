# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Pre-1.0, a breaking change bumps the minor version.

## [Unreleased]

### Added

- `#[message(id = "...")]` and `Message::id()`: a stable join key, resolved
  `id` → `msg` → type name. Sinks read the event name off a live `&dyn Message`
  with no catalogue join. `MessageDescriptor` keeps the raw `Option` id
  alongside `resolved_id()`, so an app can enforce "every analytics message
  declares an explicit id" in a catalogue test.

### Changed

- **Breaking:** `catalogue::duplicates()` returns
  `BTreeMap<&'static str, Vec<&'static MessageDescriptor>>` instead of
  `Vec<&'static str>`. A bare id cannot say *which* definitions collided, which
  matters most across crates. Migration: `assert_eq!(duplicates(), Vec::<&str>::new())`
  becomes `assert!(duplicates().is_empty(), "{:#?}", duplicates())`.
- **Breaking:** the catalogue's unique join key is the resolved id, not `msg`.
  Two messages may now share a `msg` if explicit ids keep them apart, and a
  serialized manifest keys its entries by the resolved id.
- **Breaking:** `id` is now a reserved `#[message]` attribute. It previously fell
  through to arbitrary metadata, so `#[message(id = "x")]` produced a
  `("id", "x")` entry in `descriptor.meta` and in the manifest; that entry is
  gone, and a non-string `id` value is now a compile error.
- **Breaking:** `msg` must be non-empty and free of leading or trailing
  whitespace, the same two rules as `id`. `msg` is the fallback `id`, so the
  default path could otherwise produce the empty join key `id` rejects.

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

[Unreleased]: https://github.com/yawn/tracing-wide/compare/tracing-wide-v0.2.0...HEAD
[0.2.0]: https://github.com/yawn/tracing-wide/compare/v0.1.0...tracing-wide-v0.2.0
[0.1.0]: https://github.com/yawn/tracing-wide/releases/tag/v0.1.0
