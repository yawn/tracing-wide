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

// `Alt*`/`Dup*` deliberately collide on `msg`.
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

// An explicit id that collides with another type's fallback id still counts.
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

// One shared `msg` with two distinct ids is not a duplicate.
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

/// "Predecessor" of `Started`, kept to exercise message-level deprecation: the
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
    assert_eq!(d.meta.get("owner"), Some("platform"));
    assert!(d.meta.iter().eq(common::Started::META));
    assert_eq!(d.tags.iter().collect::<Vec<_>>(), ["platform", "startup"]);
    assert_eq!(d.origin.krate, "tracing-wide");
    assert!(d.origin.file.ends_with("common/mod.rs"));

    let names: Vec<&str> = d.fields.iter().map(|f| f.name).collect();

    assert_eq!(names, ["attempt", "legacy_id", "service"]);

    let service = d.fields.iter().find(|f| f.name == "service").unwrap();

    assert_eq!(service.r#type, "& 'static str");
    assert_eq!(service.doc, Some("Logical name of the service."));
    assert_eq!(service.deprecated, None);

    let attempt = d.fields.iter().find(|f| f.name == "attempt").unwrap();

    assert_eq!(attempt.meta.get("unit"), Some("count"));

    let legacy = d.fields.iter().find(|f| f.name == "legacy_id").unwrap();

    assert_eq!(legacy.deprecated, Some("fold the id into `service`"));
    assert_eq!(legacy.r#type, "Option < usize >");

    assert_eq!(Level::from(descriptor("Renamed").level), Level::INFO);

    assert_eq!(
        descriptor("legacy started").deprecated,
        Some("use `service started` instead")
    );
}

/// The descriptor keeps the raw `id` and resolves the join key.
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

/// Reports exactly the planted collisions by resolved id, each with its descriptors.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn catalogue_reports_duplicate_ids_with_their_definition_sites() {
    let dupes = tracing_wide::catalogue::duplicates();

    let counts: Vec<(&str, usize)> = dupes.iter().map(|(id, ds)| (*id, ds.len())).collect();

    assert_eq!(
        counts,
        [
            ("also duplicated", 2),
            ("collided id", 2),
            ("duplicated msg", 3)
        ]
    );

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

    use super::descriptor;

    /// No meta on the message and none on its field.
    #[message(msg = "facet catalogue bare")]
    #[allow(dead_code)]
    struct Bare {
        n: usize,
    }

    /// Exercises every customized descriptor field; the meta keys are written out of sorted order.
    #[message(msg = "facet catalogue probe", id = "facet.probe", level = warn, team = "core", owner = "platform", tags = ["b", "a"])]
    #[deprecated = "demo"]
    #[allow(dead_code)]
    struct Probe {
        /// A documented field.
        #[field(unit = "ms", agg = "sum")]
        duration: usize,
    }

    /// The facet manifest must match the serde manifest byte for byte, so as text: a `serde_json::Value` sorts keys.
    #[test]
    fn descriptor_serializes_identically_under_serde_and_facet() {
        let d = descriptor("facet catalogue probe");
        let via_serde = serde_json::to_string(d).unwrap();

        assert_eq!(via_serde, facet_json::to_string(d).unwrap());

        assert!(!via_serde.contains(r#""id""#), "{via_serde}");
    }

    /// Both frameworks omit `meta` when it is empty, at the message and at the field level.
    #[test]
    fn empty_meta_is_omitted_under_serde_and_facet() {
        let d = descriptor("facet catalogue bare");

        for json in [
            serde_json::to_string(d).unwrap(),
            facet_json::to_string(d).unwrap(),
        ] {
            assert!(!json.contains(r#""meta""#), "{json}");
        }
    }
}
