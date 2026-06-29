//! Coverage for the auto-collected catalogue (`catalogue` feature): descriptor
//! registration from `#[message]`, and duplicate-`msg` detection. The
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
    assert!(d.origin.file.ends_with("common/mod.rs"));

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

/// Exactly the planted collisions, sorted, one entry per colliding msg; every
/// other fixture's msg is unique, so this also asserts unique msgs aren't flagged.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn catalogue_reports_duplicate_msgs() {
    assert_eq!(
        tracing_wide::catalogue::duplicates(),
        vec!["also duplicated", "duplicated msg"]
    );
}

// Native-only, like the serde catalogue manifest (serde_norway / facet-json are
// both non-wasm dev-deps): the descriptors serialize the same under both frameworks.
#[cfg(all(feature = "facet", feature = "serde", not(target_arch = "wasm32")))]
mod catalogue_facet {
    use tracing_wide::message;

    /// Exercises every customized descriptor field: struct deprecation, field
    /// docs, meta (map), level (name), tags, and origin (compact string).
    #[message(msg = "facet catalogue probe", level = warn, owner = "platform", tags = ["b", "a"])]
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
    }
}
