# Tests: `FixedOffsetTz` Parser

**File**: `zbobr-api/src/task.rs`, `mod tests` block

## Rationale
`FixedOffsetTz` is the timezone type now injected into both task backends. Its `from_str` parser has multiple format paths and error cases with no existing coverage. These tests verify the parsing contract and prevent regressions.

## Test cases to add

```rust
#[test]
fn fixed_offset_tz_parses_hhmm() {
    let tz: FixedOffsetTz = "+0300".parse().unwrap();
    assert_eq!(*tz, chrono::FixedOffset::east_opt(3 * 3600).unwrap());
}

#[test]
fn fixed_offset_tz_parses_hh_colon_mm() {
    let tz: FixedOffsetTz = "+03:00".parse().unwrap();
    assert_eq!(*tz, chrono::FixedOffset::east_opt(3 * 3600).unwrap());
}

#[test]
fn fixed_offset_tz_parses_negative() {
    let tz: FixedOffsetTz = "-05:30".parse().unwrap();
    assert_eq!(*tz, chrono::FixedOffset::west_opt(5 * 3600 + 30 * 60).unwrap());
}

#[test]
fn fixed_offset_tz_parses_utc() {
    let tz: FixedOffsetTz = "+0000".parse().unwrap();
    assert_eq!(*tz, chrono::FixedOffset::east_opt(0).unwrap());
}

#[test]
fn fixed_offset_tz_rejects_empty() {
    assert!("".parse::<FixedOffsetTz>().is_err());
}

#[test]
fn fixed_offset_tz_rejects_missing_sign() {
    assert!("0300".parse::<FixedOffsetTz>().is_err());
}

#[test]
fn fixed_offset_tz_rejects_invalid_digits() {
    assert!("+ab:cd".parse::<FixedOffsetTz>().is_err());
}

#[test]
fn fixed_offset_tz_serde_roundtrip() {
    let original: FixedOffsetTz = "+05:30".parse().unwrap();
    let json = serde_json::to_string(&original).unwrap();
    let parsed: FixedOffsetTz = serde_json::from_str(&json).unwrap();
    assert_eq!(*original, *parsed);
}
```

All tests go in the existing `mod tests` block in `zbobr-api/src/task.rs`.
