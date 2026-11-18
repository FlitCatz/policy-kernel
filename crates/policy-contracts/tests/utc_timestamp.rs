//! `UtcTimestamp` wire behavior.
//!
//! The expected strings were produced by the chrono 0.4.42 implementation this
//! type replaced. They are the canonical serialized bytes and must not change.
use policy_contracts::{IdentityError, UtcTimestamp};
use serde_json::json;
use std::time::{Duration, UNIX_EPOCH};

/// (input, canonical form, canonical form truncated to whole seconds)
const ACCEPTED: &[(&str, &str, &str)] = &[
    (
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03z",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18t01:02:03Z",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18 01:02:03Z",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03+00:00",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03-00:00",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T10:32:03+09:30",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-17T19:17:03-05:45",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03\u{2212}05:00",
        "2026-11-18T06:02:03Z",
        "2026-11-18T06:02:03Z",
    ),
    (
        "2026-11-18T01:02:03+23:59",
        "2026-11-17T01:03:03Z",
        "2026-11-17T01:03:03Z",
    ),
    (
        "2026-11-18T01:02:03-23:59",
        "2026-11-19T01:01:03Z",
        "2026-11-19T01:01:03Z",
    ),
    (
        "2026-12-31T23:30:00-01:00",
        "2027-01-01T00:30:00Z",
        "2027-01-01T00:30:00Z",
    ),
    (
        "2024-02-29T23:59:59.999999999+00:00",
        "2024-02-29T23:59:59.999999999Z",
        "2024-02-29T23:59:59Z",
    ),
    (
        "2026-11-18T01:02:03.5Z",
        "2026-11-18T01:02:03.500Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.50Z",
        "2026-11-18T01:02:03.500Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.500Z",
        "2026-11-18T01:02:03.500Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.5000Z",
        "2026-11-18T01:02:03.500Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.000Z",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.000000000000Z",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.001Z",
        "2026-11-18T01:02:03.001Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.0001Z",
        "2026-11-18T01:02:03.000100Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.123456Z",
        "2026-11-18T01:02:03.123456Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.000001Z",
        "2026-11-18T01:02:03.000001Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.0000001Z",
        "2026-11-18T01:02:03.000000100Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.123456789Z",
        "2026-11-18T01:02:03.123456789Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.000000001Z",
        "2026-11-18T01:02:03.000000001Z",
        "2026-11-18T01:02:03Z",
    ),
    // Digits past nanosecond precision are dropped, never rounded.
    (
        "2026-11-18T01:02:03.1234567899999Z",
        "2026-11-18T01:02:03.123456789Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.9999999999Z",
        "2026-11-18T01:02:03.999999999Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T01:02:03.0000000009Z",
        "2026-11-18T01:02:03Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "2026-11-18T10:32:03.25+09:30",
        "2026-11-18T01:02:03.250Z",
        "2026-11-18T01:02:03Z",
    ),
    (
        "1970-01-01T00:00:00Z",
        "1970-01-01T00:00:00Z",
        "1970-01-01T00:00:00Z",
    ),
    (
        "1969-12-31T23:59:59.999999999Z",
        "1969-12-31T23:59:59.999999999Z",
        "1969-12-31T23:59:59Z",
    ),
    (
        "2000-02-29T00:00:00Z",
        "2000-02-29T00:00:00Z",
        "2000-02-29T00:00:00Z",
    ),
    // Both ends of the supported range, directly and through an offset.
    (
        "0000-01-01T00:00:00Z",
        "0000-01-01T00:00:00Z",
        "0000-01-01T00:00:00Z",
    ),
    (
        "0000-02-29T12:00:00Z",
        "0000-02-29T12:00:00Z",
        "0000-02-29T12:00:00Z",
    ),
    (
        "0000-01-01T23:59:00+23:59",
        "0000-01-01T00:00:00Z",
        "0000-01-01T00:00:00Z",
    ),
    (
        "9999-12-30T22:00:01Z",
        "9999-12-30T22:00:01Z",
        "9999-12-30T22:00:01Z",
    ),
    (
        "9999-12-31T23:59:59Z",
        "9999-12-31T23:59:59Z",
        "9999-12-31T23:59:59Z",
    ),
    (
        "9999-12-31T23:59:59.999999999Z",
        "9999-12-31T23:59:59.999999999Z",
        "9999-12-31T23:59:59Z",
    ),
    (
        "9999-12-31T00:00:59-23:59",
        "9999-12-31T23:59:59Z",
        "9999-12-31T23:59:59Z",
    ),
];

const REJECTED: &[&str] = &[
    "",
    "2026-11-18",
    "2026-11-18T01:02:03",
    "2026-11-18T01:02Z",
    "20261118T010203Z",
    "2026-11-18T01:02:03+0000",
    "2026-11-18T01:02:03+00",
    "2026-11-18T01:02:03+00:0",
    "2026-11-18T01:02:03+0:00",
    "2026-11-18T01:02:03+24:00",
    "2026-11-18T01:02:03+00:60",
    "2026-11-18T01:02:03+05:30:00",
    "2026-11-18T01:02:03+ab:cd",
    "2026-11-18T01:02:03\u{2212}05",
    "2026-11-18T01:02:03\u{2013}05:00",
    "2026-11-18T01:02:03.Z",
    "2026-11-18T01:02:03,5Z",
    "2026-11-18T01:02:03. 5Z",
    "2026-11-18T24:00:00Z",
    "2026-11-18T01:60:03Z",
    "2026-11-18T01:02:61Z",
    "2026-02-29T00:00:00Z",
    "2100-02-29T00:00:00Z",
    "2026-13-01T00:00:00Z",
    "2026-00-10T00:00:00Z",
    "2026-11-31T00:00:00Z",
    "2026-11-00T00:00:00Z",
    "2026-1a-18T01:02:03Z",
    "10000-01-01T00:00:00Z",
    "999-01-01T00:00:00Z",
    "+002026-11-18T01:02:03Z",
    "+2026-11-18T01:02:03Z",
    "-0001-12-31T23:00:00Z",
    "\u{ff12}\u{ff10}\u{ff12}\u{ff16}-11-18T01:02:03Z",
    "2026-11-18T01:02:03Z[UTC]",
    " 2026-11-18T01:02:03Z",
    "2026-11-18T01:02:03Z ",
    "2026-11-18T01:02:03 Z",
    "2026-11-18T01:02:03ZZ",
    "2026-11-18T01:02:03UTC",
    "2026-11-18_01:02:03Z",
    "2026/11/18T01:02:03Z",
    "2026-11-18T01-02-03Z",
];

/// Inputs chrono accepted that have no canonical form here: a leap second,
/// and offsets that move the UTC year outside 0000-9999. Rejecting them is an
/// intentional, fail-closed difference from the previous implementation.
const NO_CANONICAL_FORM: &[&str] = &[
    "2016-12-31T23:59:60Z",
    "2016-12-31T23:59:60.5Z",
    "2026-11-18T12:34:60+00:30",
    "0000-01-01T00:00:00+00:01",
    "0000-01-01T00:00:00+23:59",
    "9999-12-31T23:59:59-00:01",
    "9999-12-31T00:01:00-23:59",
];

fn parse(value: &str) -> UtcTimestamp {
    UtcTimestamp::parse(value).unwrap()
}

#[test]
fn accepted_inputs_serialize_to_the_pinned_canonical_bytes() {
    for (input, canonical, seconds) in ACCEPTED {
        let timestamp = parse(input);
        assert_eq!(timestamp.to_string(), *canonical, "{input}");
        assert_eq!(
            serde_json::to_string(&timestamp).unwrap(),
            format!("\"{canonical}\""),
            "{input}"
        );
        assert_eq!(
            format!("{timestamp:?}"),
            format!("UtcTimestamp({canonical})")
        );
        assert_eq!(
            timestamp.truncated_to_seconds().to_string(),
            *seconds,
            "{input}"
        );
        // Every entry point agrees, and the canonical form is a fixed point.
        assert_eq!(input.parse::<UtcTimestamp>().unwrap(), timestamp);
        assert_eq!(
            serde_json::from_value::<UtcTimestamp>(json!(input)).unwrap(),
            timestamp
        );
        assert_eq!(parse(canonical), timestamp);
        assert_eq!(parse(canonical).to_string(), *canonical);
    }
}

#[test]
fn malformed_and_unrepresentable_inputs_are_rejected() {
    for input in REJECTED.iter().chain(NO_CANONICAL_FORM) {
        assert_eq!(
            UtcTimestamp::parse(input),
            Err(IdentityError::InvalidTimestamp),
            "{input:?}"
        );
        assert!(
            serde_json::from_value::<UtcTimestamp>(json!(input)).is_err(),
            "{input:?}"
        );
    }
    assert!(serde_json::from_value::<UtcTimestamp>(json!(1_763_427_723)).is_err());
}

#[test]
fn ordering_and_durations_are_exact_to_the_nanosecond() {
    let base = parse("2026-11-18T01:02:03Z");
    let next = parse("2026-11-18T01:02:03.000000001Z");
    assert!(base < next);
    assert_eq!(next.duration_since(&base), Some(Duration::from_nanos(1)));
    assert_eq!(base.duration_since(&base), Some(Duration::ZERO));
    assert_eq!(base.duration_since(&next), None);

    // The same instant written with different offsets is one value.
    assert_eq!(parse("2026-11-18T10:32:03+09:30"), base);
    assert_eq!(
        parse("2026-11-18T10:32:04+09:30").duration_since(&base),
        Some(Duration::from_secs(1))
    );

    let first = parse("0000-01-01T00:00:00Z");
    let last = parse("9999-12-31T23:59:59.999999999Z");
    assert_eq!(
        last.duration_since(&first),
        Some(Duration::new(315_569_519_999, 999_999_999))
    );
    assert_eq!(first.duration_since(&last), None);
}

#[test]
fn arithmetic_is_exact_and_stays_inside_the_supported_range() {
    let base = parse("2026-11-18T01:02:03Z");
    for (start, forward, end) in [
        (
            "2026-11-18T01:02:03Z",
            Duration::from_millis(1_500),
            "2026-11-18T01:02:04.500Z",
        ),
        (
            "2026-11-18T01:02:02.999999999Z",
            Duration::from_nanos(1),
            "2026-11-18T01:02:03Z",
        ),
        (
            "2026-12-31T23:59:59Z",
            Duration::from_secs(1),
            "2027-01-01T00:00:00Z",
        ),
        (
            "2024-02-28T23:59:59Z",
            Duration::from_secs(1),
            "2024-02-29T00:00:00Z",
        ),
        (
            "2026-02-28T23:59:59Z",
            Duration::from_secs(1),
            "2026-03-01T00:00:00Z",
        ),
        (
            "2026-11-18T01:02:03Z",
            Duration::from_secs(86_400),
            "2026-11-19T01:02:03Z",
        ),
    ] {
        assert_eq!(parse(start).checked_add(forward).unwrap().to_string(), end);
        assert_eq!(parse(end).checked_sub(forward).unwrap(), parse(start));
    }

    let first = parse("0000-01-01T00:00:00Z");
    let last = parse("9999-12-31T23:59:59.999999999Z");
    assert_eq!(first.checked_sub(Duration::ZERO), Some(first.clone()));
    assert_eq!(last.checked_add(Duration::ZERO), Some(last.clone()));
    assert_eq!(first.checked_sub(Duration::from_nanos(1)), None);
    assert_eq!(last.checked_add(Duration::from_nanos(1)), None);
    assert_eq!(base.checked_add(Duration::MAX), None);
    assert_eq!(base.checked_sub(Duration::MAX), None);
}

#[test]
fn unix_milliseconds_convert_only_inside_the_supported_range() {
    for (millis, canonical) in [
        (0, Some("1970-01-01T00:00:00Z")),
        (1, Some("1970-01-01T00:00:00.001Z")),
        (-1, Some("1969-12-31T23:59:59.999Z")),
        (1_763_427_723_456, Some("2025-11-18T01:02:03.456Z")),
        (-62_167_219_200_000, Some("0000-01-01T00:00:00Z")),
        (253_402_300_799_999, Some("9999-12-31T23:59:59.999Z")),
        (-62_167_219_200_001, None),
        (253_402_300_800_000, None),
        (i64::MAX, None),
        (i64::MIN, None),
    ] {
        assert_eq!(
            UtcTimestamp::from_unix_millis(millis).map(|timestamp| timestamp.to_string()),
            canonical.map(str::to_owned),
            "{millis}"
        );
    }
}

#[test]
fn system_time_converts_with_full_precision_on_both_sides_of_the_epoch() {
    let convert = |time| UtcTimestamp::try_from(time).map(|timestamp| timestamp.to_string());
    assert_eq!(convert(UNIX_EPOCH).unwrap(), "1970-01-01T00:00:00Z");
    assert_eq!(
        convert(UNIX_EPOCH + Duration::new(1_763_427_723, 456_789_123)).unwrap(),
        "2025-11-18T01:02:03.456789123Z"
    );
    assert_eq!(
        convert(UNIX_EPOCH + Duration::new(1_763_427_723, 456_000_000)).unwrap(),
        "2025-11-18T01:02:03.456Z"
    );
    assert_eq!(
        convert(UNIX_EPOCH - Duration::from_nanos(1)).unwrap(),
        "1969-12-31T23:59:59.999999999Z"
    );
    assert_eq!(
        convert(UNIX_EPOCH + Duration::new(253_402_300_799, 999_999_999)).unwrap(),
        "9999-12-31T23:59:59.999999999Z"
    );
    assert_eq!(
        convert(UNIX_EPOCH + Duration::from_secs(253_402_300_800)),
        Err(IdentityError::InvalidTimestamp)
    );
    // Not every platform clock can express year 0000; check it where it can.
    if let Some(first) = UNIX_EPOCH.checked_sub(Duration::from_secs(62_167_219_200)) {
        assert_eq!(convert(first).unwrap(), "0000-01-01T00:00:00Z");
        assert_eq!(
            convert(first - Duration::from_nanos(1)),
            Err(IdentityError::InvalidTimestamp)
        );
    }
}
