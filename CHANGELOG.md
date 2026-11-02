# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Pre-1.0, a breaking change bumps the minor version.

## [Unreleased]

### Changed

- **Breaking:** `FieldDescriptor::r#type` is replaced by
  `FieldDescriptor::type_name()`: the field type as `core::any::type_name`
  resolves it (`core::option::Option<usize>`, aliases resolved), not as written
  (`Option < usize >`). The serde and facet manifests show the resolved type.
  The text can change between compiler versions.

## [0.3.0] - 2026-11-01

### Added

- Example `metrics`, deriving OpenTelemetry metrics from messages in a sink and
  printing them with the stdout exporter.
- `Message::meta()`, returning the new `Meta` type, and the `META` inherent
  const: message-level `#[message(key = "value")]` pairs, readable off a live
  `&dyn Message` and looked up with `Meta::get`. A sink that routes on metadata
  no longer needs a `TypeId` → `MessageDescriptor` join, and so no longer needs
  the `catalogue` feature in the application.
- `#[message(id = "...")]` and `Message::id()`: a stable join key, resolved
  `id` → `msg` → type name. Sinks read the event name off a live `&dyn Message`
  with no catalogue join. `MessageDescriptor` keeps the raw `Option` id
  alongside `resolved_id()`, so an app can enforce "every analytics message
  declares an explicit id" in a catalogue test.

### Changed

- **Breaking:** renamed the fan-out trait `subscriber::Subscriber` to `sink::Sink` and
  the set `Subscribers` to `Sinks`, so it no longer shares a name with
  `tracing::Subscriber`. The module, the cargo feature and the examples follow:
  `subscriber` is now `sink`, `subscriber-facet` is `sink-facet`, and
  `subscriber-serde` is `sink-serde`.
- **Breaking:** `Message::tags()`, the `TAGS` inherent const, and `MessageDescriptor::tags`
  are a `Tags` instead of a `&'static [&'static str]`. Check a tag with
  `tags.contains("x")` and list them with `tags.iter()`.
- **Breaking:** `FieldDescriptor::meta` and `MessageDescriptor::meta` are a `Meta`
  instead of a `&'static [(&'static str, &'static str)]`. Read a value with
  `meta.get(key)` and every pair with `meta.iter()`.
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
- The `catalogue` feature's docs no longer say it should never be enabled by an
  application. Applications legitimately enable it for catalogue tests
  (`catalogue::duplicates()`), lookups, and manifest generation; libraries still
  should not.
- **Breaking:** a tag must be free of leading or trailing whitespace, the same
  rule as `msg` and `id`.
- **Breaking:** `msg` must be non-empty and free of leading or trailing
  whitespace, the same two rules as `id`. `msg` is the fallback `id`, so the
  default path could otherwise produce the empty join key `id` rejects.

### Fixed

- Metadata pairs are sorted by key at expansion. The serde manifest wrote them in
  the order written while the facet manifest sorted them, so the two manifests
  differed for any `meta` with two or more keys.

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

[Unreleased]: https://github.com/yawn/tracing-wide/compare/tracing-wide-v0.3.0...HEAD
[0.3.0]: https://github.com/yawn/tracing-wide/compare/tracing-wide-v0.2.0...tracing-wide-v0.3.0
[0.2.0]: https://github.com/yawn/tracing-wide/compare/v0.1.0...tracing-wide-v0.2.0
[0.1.0]: https://github.com/yawn/tracing-wide/releases/tag/v0.1.0
