//! Coverage for the auto-collected catalogue (`catalogue` feature): descriptor
//! registration from `#[message]`, and duplicate-id detection. The
//! richly-attributed `Started`/`Renamed` fixtures whose descriptors this
//! inspects live in `common` (shared with the core suite), so the origin-file
//! assertion below points there.

#![cfg(feature = "catalogue")]

mod common;

use tracing::Level;
use tracing_wide::message;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

// `Alt*`/`Dup*` deliberately collide on `msg` — only ever registered, so they
// poison nothing but the duplicate check they exist to exercise. The `Dup*`
// triple shares one msg (dedup: one report, not one per extra registration);
// the `Alt*` pair shares another (multiple distinct collisions are all reported).
#[message(msg = "also duplicated")]
#[allow(dead_code)]
struct AltA {
    a: usize,
}

#[message(msg = "also duplicated")]
#[allow(dead_code)]
struct AltB {
    b: usize,
}

#[message(msg = "duplicated msg")]
#[allow(dead_code)]
struct DupA {
    a: usize,
}

#[message(msg = "duplicated msg")]
#[allow(dead_code)]
struct DupB {
    b: usize,
}

#[message(msg = "duplicated msg")]
#[allow(dead_code)]
struct DupC {
    c: usize,
}

// An explicit id colliding with another type's *fallback* id: the join key is
// the resolved id, so a collision across the two forms still counts.
#[message(msg = "an unremarkable sentence", id = "collided id")]
#[allow(dead_code)]
struct IdDup {
    a: usize,
}

#[message(msg = "collided id")]
#[allow(dead_code)]
struct MsgDup {
    b: usize,
}

// The inverse: one shared `msg`, two distinct explicit ids. Under the old
// msg-keyed check this pair was a duplicate; under the resolved id it is not.
#[message(msg = "shared sentence", id = "shared.first")]
#[allow(dead_code)]
struct SharedMsgA {
    a: usize,
}

#[message(msg = "shared sentence", id = "shared.second")]
#[allow(dead_code)]
struct SharedMsgB {
    b: usize,
}

/// Predecessor of `Started`, kept to exercise message-level deprecation: the
/// note must land in the descriptor, and the generated impls must not warn (only
/// producer construction should).
#[message(msg = "legacy started")]
#[deprecated = "use `service started` instead"]
#[allow(dead_code)]
struct LegacyStarted {
    n: usize,
}

fn descriptor(msg: &str) -> &'static tracing_wide::catalogue::MessageDescriptor {
    tracing_wide::catalogue::all()
        .find(|d| d.msg == msg)
        .unwrap_or_else(|| panic!("no catalogue entry for {msg:?}"))
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn catalogue_registers_message_descriptors() {
    let d = descriptor("service started");

    assert_eq!(d.level.as_str(), "WARN");
    assert_eq!(Level::from(d.level), Level::WARN);
    assert_eq!(d.doc, Some("Emitted when a service finishes starting up."));
    assert_eq!(d.deprecated, None);
    assert!(d.meta.contains(&("owner", "platform")));
    assert_eq!(d.tags.to_vec(), ["platform", "startup"]);
    assert_eq!(d.origin.krate, "tracing-wide");
    assert!(d.origin.file.ends_with("common.rs"));

    let names: Vec<&str> = d.fields.iter().map(|f| f.name).collect();

    assert_eq!(names, ["attempt", "legacy_id", "service"]);

    let service = d.fields.iter().find(|f| f.name == "service").unwrap();

    assert_eq!(service.r#type, "& 'static str");
    assert_eq!(service.doc, Some("Logical name of the service."));
    assert_eq!(service.deprecated, None);

    let attempt = d.fields.iter().find(|f| f.name == "attempt").unwrap();

    assert!(attempt.meta.contains(&("unit", "count")));

    let legacy = d.fields.iter().find(|f| f.name == "legacy_id").unwrap();

    assert_eq!(legacy.deprecated, Some("fold the id into `service`"));
    assert_eq!(legacy.r#type, "Option < usize >");

    assert_eq!(Level::from(descriptor("Renamed").level), Level::INFO);

    assert_eq!(
        descriptor("legacy started").deprecated,
        Some("use `service started` instead")
    );
}

/// `id` stays raw so an app can tell "declared an id" from "the id happens to
/// equal the msg" — the distinction a catalogue policy test needs.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn descriptor_keeps_the_raw_id_and_resolves_the_join_key() {
    let explicit = descriptor("payment charged");

    assert_eq!(explicit.id, Some("billing.payment_charged"));
    assert_eq!(explicit.resolved_id(), "billing.payment_charged");

    let fallback = descriptor("service started");

    assert_eq!(fallback.id, None);
    assert_eq!(fallback.resolved_id(), "service started");
}

/// Exactly the planted collisions, one entry per colliding id, each carrying the
/// descriptors that claim it. The `SharedMsgA`/`SharedMsgB` pair shares a `msg`
/// but not an id and is absent — the assertion that the check keys on the
/// resolved id, not on `msg`.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn catalogue_reports_duplicate_ids_with_their_definition_sites() {
    let dupes = tracing_wide::catalogue::duplicates();

    // Keys are ordered by the map; `Dup*` is a triple, so a colliding id is
    // reported once with all of its claimants, not once per extra registration.
    let counts: Vec<(&str, usize)> = dupes.iter().map(|(id, ds)| (*id, ds.len())).collect();

    assert_eq!(
        counts,
        [
            ("also duplicated", 2),
            ("collided id", 2),
            ("duplicated msg", 3)
        ]
    );

    // The point of carrying descriptors: the report names each definition site.
    let collided = &dupes["collided id"];

    assert_eq!(collided[0].msg, "an unremarkable sentence");
    assert_eq!(collided[0].id, Some("collided id"));
    assert_eq!(collided[1].msg, "collided id");
    assert_eq!(collided[1].id, None);

    for d in collided {
        assert_eq!(d.origin.krate, "tracing-wide");
        assert!(d.origin.file.ends_with("catalogue.rs"));
    }
}

// Native-only, like the serde catalogue manifest (serde_norway / facet-json are
// both non-wasm dev-deps): the descriptors serialize the same under both frameworks.
#[cfg(all(feature = "facet", feature = "serde", not(target_arch = "wasm32")))]
mod catalogue_facet {
    use tracing_wide::message;

    /// Exercises every customized descriptor field: struct deprecation, field
    /// docs, meta (map), level (name), tags, and origin (compact string).
    #[message(msg = "facet catalogue probe", id = "facet.probe", level = warn, owner = "platform", tags = ["b", "a"])]
    #[deprecated = "demo"]
    #[allow(dead_code)]
    struct Probe {
        /// A documented field.
        #[field(unit = "ms")]
        duration: usize,
    }

    /// The facet manifest must match the serde manifest field-for-field — the
    /// whole point of the proxies (compact origin, map meta, level name).
    #[test]
    fn descriptor_serializes_identically_under_serde_and_facet() {
        let descriptor = tracing_wide::catalogue::all()
            .find(|d| d.msg == "facet catalogue probe")
            .expect("Probe is registered");

        let via_serde = serde_json::to_value(descriptor).unwrap();
        let via_facet: serde_json::Value =
            serde_json::from_str(&facet_json::to_string(descriptor).unwrap()).unwrap();

        assert_eq!(via_serde, via_facet);

        // `id` is skipped under both frameworks — the manifest shape is unchanged
        // by the raw id, even for a message that declares one.
        assert!(via_serde.get("id").is_none());
    }
}
