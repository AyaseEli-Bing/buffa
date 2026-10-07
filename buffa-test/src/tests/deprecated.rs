//! `[deprecated = true]`: marking a field or enum value deprecated must not
//! change what the codec does — the value still round-trips, prints, and
//! serializes. Reading these fields from consumer code is exactly what the
//! `#[deprecated]` markers exist to warn about, hence the module-level allow.

#![allow(deprecated)]

use crate::deprecated::__buffa::oneof;
use crate::deprecated::__buffa::view::LegacyProfileView;
use crate::deprecated::*;
use buffa::text::encode_to_string;
use buffa::{Enumeration, Message, MessageView};

fn profile() -> LegacyProfile {
    LegacyProfile {
        display_name: "Ada".to_string(),
        nick: "ada-l".to_string(),
        old_tags: vec!["legacy".to_string(), "archived".to_string()],
        old_attrs: [("tier".to_string(), "0".to_string())]
            .into_iter()
            .collect(),
        counter: 7,
        levels: Some(Levels {
            low: 1,
            high: 9,
            ..Default::default()
        })
        .into(),
        channel: Some(oneof::legacy_profile::Channel::Email(
            "ada@example.com".to_string(),
        )),
        ..Default::default()
    }
}

#[test]
fn deprecated_fields_round_trip() {
    let decoded = crate::tests::round_trip(&profile());
    assert_eq!(decoded.display_name, "Ada");
    assert_eq!(decoded.nick, "ada-l");
    assert_eq!(decoded.old_tags, vec!["legacy", "archived"]);
    assert_eq!(decoded.old_attrs.get("tier").map(String::as_str), Some("0"));
    assert_eq!(decoded.counter, 7);
    assert_eq!(
        decoded.levels.as_option().map(|l| (l.low, l.high)),
        Some((1, 9))
    );
}

#[test]
fn deprecated_values_survive_view_round_trip() {
    let bytes = profile().encode_to_vec();
    let view = LegacyProfileView::decode_view(&bytes).expect("view decode");
    let owned = view.to_owned_message().expect("view to_owned");
    assert_eq!(owned, profile());
}

#[test]
fn deprecated_enum_value_keeps_name_and_default_semantics() {
    assert_eq!(Routing::LEGACY.to_i32(), 1);
    assert_eq!(Routing::LEGACY.proto_name(), "LEGACY");
    // The alias names the same value, so it resolves to the same variant.
    assert_eq!(Routing::OLD_LEGACY, Routing::LEGACY);
    assert_eq!(
        Routing::from_proto_name("OLD_LEGACY"),
        Some(Routing::LEGACY)
    );
    assert!(
        Routing::values().contains(&Routing::LEGACY),
        "values() must still list the deprecated value"
    );
    // Deprecation of the first value does not move the declared default.
    assert_eq!(Degenerate::default(), Degenerate::DEAD);
}

#[test]
fn text_and_json_still_carry_deprecated_fields() {
    let text = encode_to_string(&profile());
    assert!(text.contains("display_name"), "text: {text}");
    assert!(text.contains("counter: 7"), "text: {text}");

    let json = serde_json::to_string(&profile()).expect("json encode");
    assert!(json.contains("\"displayName\":\"Ada\""), "json: {json}");
    let decoded: LegacyProfile = serde_json::from_str(&json).expect("json decode");
    assert_eq!(decoded.display_name, "Ada");
}

/// The proto2 half: `[default = OLD]` where `OLD` is deprecated stays the
/// declared default. That default is also why the generated `Default` impl and
/// `clear` need the guard — they name the deprecated variant.
#[test]
fn deprecated_enum_default_is_still_the_declared_default() {
    use crate::deprecated_proto2::{Holder, Size};

    assert_eq!(Holder::default().size, Size::OLD);
    let mut holder = Holder {
        size: Size::NEW,
        note: Some("n".to_string()),
        ..Default::default()
    };
    <Holder as Message>::clear(&mut holder);
    assert_eq!(
        holder.size,
        Size::OLD,
        "clear restores the declared default"
    );
    assert!(holder.note.is_none());
}

/// An alias of a deprecated value as the declared default: `STARTER` is
/// number 1, whose variant is `BASIC`.
#[test]
fn alias_of_deprecated_value_is_still_the_declared_default() {
    use crate::deprecated_proto2::{Plan, Tier};

    assert_eq!(Plan::default().tier, Tier::BASIC);
    assert_eq!(Tier::STARTER, Tier::BASIC);
}

/// An enum with a deprecated value implements `Arbitrary` without the derive,
/// and still produces every declared variant, the deprecated ones included.
#[cfg(feature = "arbitrary")]
#[test]
fn arbitrary_covers_deprecated_enum_values() {
    use arbitrary::{Arbitrary, Unstructured};
    use std::collections::HashSet;

    let seen: HashSet<i32> = (0..=u8::MAX)
        .map(|byte| {
            Routing::arbitrary(&mut Unstructured::new(&[byte; 8]))
                .expect("eight bytes are enough to choose a variant")
                .to_i32()
        })
        .collect();
    let declared: HashSet<i32> = Routing::values().iter().map(|v| v.to_i32()).collect();
    assert_eq!(seen, declared);
    assert!(seen.contains(&Routing::LEGACY.to_i32()));

    // The messages keep the derive.
    LegacyProfile::arbitrary(&mut Unstructured::new(&[7; 64])).expect("arbitrary message");
}
