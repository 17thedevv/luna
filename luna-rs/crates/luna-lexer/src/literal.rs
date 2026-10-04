//! Exact, target-independent lexical decoding. Type selection belongs to semantic
//! analysis; this module never performs an implicit numeric conversion.
use crate::BuiltinKind;

/// Decode the ordinary string escapes used by both native and comptime
/// execution. Unknown escapes retain their spelling, matching the existing
/// ordinary-string contract; raw/byte literals use their own decoder.
pub fn decode_string(text: &str) -> String {
    let raw = text.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(text);
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' { out.push(c); continue; }
        match chars.next() {
            Some('n') => out.push('\n'), Some('r') => out.push('\r'),
            Some('t') => out.push('\t'), Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'), Some('\'') => out.push('\''), Some('"') => out.push('"'),
            Some(c) => { out.push('\\'); out.push(c); },
            None => out.push('\\'),
        }
    }
    out
}

pub fn quote_string(value: &str) -> String {
    let mut text = String::from("\"");
    for c in value.chars() {
        match c {
            '\n' => text.push_str("\\n"), '\r' => text.push_str("\\r"), '\t' => text.push_str("\\t"),
            '\0' => text.push_str("\\0"), '\\' => text.push_str("\\\\"), '"' => text.push_str("\\\""),
            c => text.push(c),
        }
    }
    text.push('"');
    text
}

pub fn decode_character(text: &str) -> Result<char, String> {
    let body = text.strip_prefix('\'').and_then(|s| s.strip_suffix('\''))
        .ok_or("invalid character literal delimiters")?;
    let value = decode_string(&format!("\"{body}\""));
    let mut chars = value.chars();
    let scalar = chars.next().ok_or("empty character literal")?;
    if chars.next().is_some() { return Err("character literal must contain exactly one Unicode scalar".into()); }
    Ok(scalar)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerLiteral {
    pub magnitude: u128,
    pub suffix: Option<BuiltinKind>,
    pub radix: u32,
}

pub fn decode_integer(text: &str) -> Result<IntegerLiteral, String> {
    let suffixes = [
        ("isize", BuiltinKind::Isize),
        ("usize", BuiltinKind::Usize),
        ("i128", BuiltinKind::I128),
        ("u128", BuiltinKind::U128),
        ("i64", BuiltinKind::I64),
        ("u64", BuiltinKind::U64),
        ("i32", BuiltinKind::I32),
        ("u32", BuiltinKind::U32),
        ("i16", BuiltinKind::I16),
        ("u16", BuiltinKind::U16),
        ("i8", BuiltinKind::I8),
        ("u8", BuiltinKind::U8),
    ];
    let (digits, suffix) = suffixes
        .iter()
        .find_map(|(suffix, kind)| {
            text.strip_suffix(suffix)
                .map(|digits| (digits, Some(*kind)))
        })
        .unwrap_or((text, None));
    let (radix, digits) = match digits.get(..2) {
        Some("0x" | "0X") => (16, &digits[2..]),
        Some("0o" | "0O") => (8, &digits[2..]),
        Some("0b" | "0B") => (2, &digits[2..]),
        _ => (10, digits),
    };
    if digits.is_empty() || !digits.as_bytes()[0].is_ascii_hexdigit() {
        return Err("integer literal requires digits after its radix prefix".into());
    }
    let mut magnitude = 0u128;
    for c in digits.chars().filter(|&c| c != '_') {
        let digit = c
            .to_digit(radix)
            .ok_or_else(|| format!("invalid digit or suffix in integer literal `{text}`"))?;
        magnitude = magnitude
            .checked_mul(radix as u128)
            .and_then(|n| n.checked_add(digit as u128))
            .ok_or_else(|| "integer literal magnitude exceeds 128 bits".to_string())?;
    }
    Ok(IntegerLiteral {
        magnitude,
        suffix,
        radix,
    })
}

pub fn magnitude_fits(magnitude: u128, negative: bool, bits: u32, signed: bool) -> bool {
    if !(1..=128).contains(&bits) || (negative && !signed) {
        return false;
    }
    if signed {
        let edge = 1u128 << (bits - 1);
        if negative {
            magnitude <= edge
        } else {
            magnitude < edge
        }
    } else {
        bits == 128 || magnitude < (1u128 << bits)
    }
}

pub fn decode_bytes(text: &str, character: bool) -> Result<Vec<u8>, String> {
    let quote = if character { '\'' } else { '"' };
    let body = text
        .strip_prefix('b')
        .and_then(|s| s.strip_prefix(quote))
        .and_then(|s| s.strip_suffix(quote))
        .ok_or("invalid byte literal delimiters")?;
    let mut result = Vec::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            if !c.is_ascii() {
                return Err("byte literals require ASCII characters or \\xNN escapes".into());
            }
            result.push(c as u8);
            continue;
        }
        let escaped = match chars.next().ok_or("incomplete byte escape")? {
            'n' => b'\n',
            'r' => b'\r',
            't' => b'\t',
            '0' => 0,
            '\\' => b'\\',
            '\'' => b'\'',
            '"' => b'"',
            'x' => {
                let hi = chars
                    .next()
                    .and_then(|c| c.to_digit(16))
                    .ok_or("byte escape requires two hexadecimal digits")?;
                let lo = chars
                    .next()
                    .and_then(|c| c.to_digit(16))
                    .ok_or("byte escape requires two hexadecimal digits")?;
                (hi * 16 + lo) as u8
            }
            _ => return Err("invalid byte escape; Unicode escapes are not byte literals".into()),
        };
        result.push(escaped);
    }
    if character && result.len() != 1 {
        return Err("byte character literal must contain exactly one byte".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_magnitude_and_suffix() {
        assert_eq!(decode_integer("0xff_u8").unwrap().magnitude, 255);
        assert_eq!(
            decode_integer("0b10usize").unwrap().suffix,
            Some(BuiltinKind::Usize)
        );
        assert_eq!(
            decode_integer("340282366920938463463374607431768211455u128")
                .unwrap()
                .magnitude,
            u128::MAX
        );
        for invalid in [
            "0x",
            "0b2",
            "1wat",
            "340282366920938463463374607431768211456",
        ] {
            assert!(decode_integer(invalid).is_err(), "{invalid}");
        }
    }
    #[test]
    fn ranges_do_not_depend_on_host_width() {
        assert!(magnitude_fits(128, true, 8, true));
        assert!(!magnitude_fits(128, false, 8, true));
        assert!(!magnitude_fits(1, true, 64, false));
        assert!(!magnitude_fits(1u128 << 32, false, 32, false));
        assert!(magnitude_fits(1u128 << 32, false, 64, false));
        assert!(magnitude_fits(u128::MAX, false, 128, false));
    }
    #[test]
    fn byte_arrays_are_not_text_or_c_strings() {
        assert_eq!(
            decode_bytes("b\"A\\0\\xFF\"", false).unwrap(),
            vec![65, 0, 255]
        );
        assert_eq!(
            decode_bytes("b''", true).unwrap_err(),
            "byte character literal must contain exactly one byte"
        );
        assert!(decode_bytes("b\"é\"", false).is_err());
        assert!(decode_bytes("b\"\\u{41}\"", false).is_err());
    }
}
