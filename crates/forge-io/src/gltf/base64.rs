//! Minimal clean-room Base64 encoder and decoder for data URI handling in glTF 2.0.

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes binary slice into Base64 standard string with padding.
pub fn encode_base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(B64_CHARS[((n >> 18) & 63) as usize] as char);
        out.push(B64_CHARS[((n >> 12) & 63) as usize] as char);
        out.push(B64_CHARS[((n >> 6) & 63) as usize] as char);
        out.push(B64_CHARS[(n & 63) as usize] as char);
        i += 3;
    }

    if i + 1 == data.len() {
        let n = (data[i] as u32) << 16;
        out.push(B64_CHARS[((n >> 18) & 63) as usize] as char);
        out.push(B64_CHARS[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if i + 2 == data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(B64_CHARS[((n >> 18) & 63) as usize] as char);
        out.push(B64_CHARS[((n >> 12) & 63) as usize] as char);
        out.push(B64_CHARS[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }

    out
}

fn b64_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decodes Base64 encoded string into binary vector.
pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let clean: Vec<u8> = input
        .bytes()
        .filter(|&b| !b.is_ascii_whitespace())
        .collect();
    if clean.is_empty() {
        return Some(Vec::new());
    }
    if !clean.len().is_multiple_of(4) {
        return None;
    }

    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.as_chunks::<4>().0 {
        let b0 = b64_val(chunk[0])?;
        let b1 = b64_val(chunk[1])?;

        if chunk[2] == b'=' {
            if chunk[3] != b'=' {
                return None;
            }
            let n = ((b0 as u32) << 18) | ((b1 as u32) << 12);
            out.push(((n >> 16) & 0xFF) as u8);
        } else if chunk[3] == b'=' {
            let b2 = b64_val(chunk[2])?;
            let n = ((b0 as u32) << 18) | ((b1 as u32) << 12) | ((b2 as u32) << 6);
            out.push(((n >> 16) & 0xFF) as u8);
            out.push(((n >> 8) & 0xFF) as u8);
        } else {
            let b2 = b64_val(chunk[2])?;
            let b3 = b64_val(chunk[3])?;
            let n = ((b0 as u32) << 18) | ((b1 as u32) << 12) | ((b2 as u32) << 6) | (b3 as u32);
            out.push(((n >> 16) & 0xFF) as u8);
            out.push(((n >> 8) & 0xFF) as u8);
            out.push((n & 0xFF) as u8);
        }
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let cases: &[&[u8]] = &[
            b"",
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0x00, 0xFF, 0x42, 0x7F, 0x80, 0xAA, 0x55],
        ];

        for &case in cases {
            let encoded = encode_base64(case);
            let decoded = decode_base64(&encoded).expect("Decode should succeed");
            assert_eq!(decoded, case);
        }
    }
}
