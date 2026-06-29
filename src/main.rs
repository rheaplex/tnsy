//! 4-bit two-state Baudot-style codes for `[a-z]` + space.
//!
//! Two variants are provided:
//!
//! * [`two_shift`] — uses two shift codepoints (`0xE` up, `0xF` down). On a
//!   2-state ring both simply flip A<->B, so the second code is redundant for
//!   *this* alphabet but keeps the scheme a clean superset of a multi-state
//!   version. 14 character slots per state.
//!
//! * [`one_shift`] — uses a single shift codepoint (`0xF`). That reclaims `0xE`
//!   as a 15th character slot per state, leaving room for a couple of
//!   punctuation marks while still fitting `[a-z]` + space.
//!
//! In both, space lives in *both* states so word boundaries never force a
//! shift. There is no end-of-message sentinel; the decoder relies on an
//! external nibble-count length specifier.

/// Pack 4-bit nibbles into bytes, high nibble first. A trailing odd nibble
/// parks in the high half of the final byte; its low half is unused padding.
/// The returned count is the authoritative external length specifier.
fn pack_nibbles(nibbles: &[u8]) -> (Vec<u8>, usize) {
    let mut bytes = Vec::with_capacity((nibbles.len() + 1) / 2);
    for pair in nibbles.chunks(2) {
        let hi = pair[0] << 4;
        let lo = if pair.len() == 2 { pair[1] } else { 0 };
        bytes.push(hi | lo);
    }
    (bytes, nibbles.len())
}

fn unpack_nibbles(bytes: &[u8], nibble_count: usize) -> Vec<u8> {
    let mut nibbles = Vec::with_capacity(nibble_count);
    for (i, &b) in bytes.iter().enumerate() {
        nibbles.push(b >> 4);
        if i * 2 + 1 < nibble_count {
            nibbles.push(b & 0x0F);
        }
    }
    nibbles.truncate(nibble_count);
    nibbles
}

#[derive(Debug, PartialEq, Eq)]
pub enum EncodeError {
    Unsupported(char),
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    InvalidCode(u8),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    A,
    B,
}

impl State {
    fn flip(self) -> State {
        match self {
            State::A => State::B,
            State::B => State::A,
        }
    }
}

fn find_in(table: &[char], ch: char) -> Option<u8> {
    table.iter().position(|&c| c == ch).map(|p| p as u8)
}

// ===========================================================================
// Variant 1: two shift codes (0xE up, 0xF down), 14 slots/state.
// ===========================================================================
pub mod two_shift {
    use super::*;

    pub const SHIFT_UP: u8 = 0xE;
    pub const SHIFT_DOWN: u8 = 0xF;

    pub const STATE_A: [char; 14] = [
        ' ', 'e', 't', 'a', 'o', 'i', 'n', 's', 'h', 'r', 'd', 'l', 'c', 'u',
    ];
    pub const STATE_B: [char; 14] = [
        ' ', 'm', 'w', 'f', 'g', 'y', 'p', 'b', 'v', 'k', 'j', 'x', 'q', 'z',
    ];

    fn table(state: State) -> &'static [char; 14] {
        match state {
            State::A => &STATE_A,
            State::B => &STATE_B,
        }
    }

    pub fn encode(input: &str) -> Result<(Vec<u8>, usize), EncodeError> {
        let mut state = State::A;
        let mut nibbles = Vec::with_capacity(input.len());
        for ch in input.chars() {
            if let Some(code) = find_in(table(state), ch) {
                nibbles.push(code);
            } else if let Some(code) = find_in(table(state.flip()), ch) {
                nibbles.push(if state == State::A { SHIFT_UP } else { SHIFT_DOWN });
                state = state.flip();
                nibbles.push(code);
            } else {
                return Err(EncodeError::Unsupported(ch));
            }
        }
        Ok(pack_nibbles(&nibbles))
    }

    pub fn decode(bytes: &[u8], nibble_count: usize) -> Result<String, DecodeError> {
        let nibbles = unpack_nibbles(bytes, nibble_count);
        let mut state = State::A;
        let mut out = String::new();
        for &n in &nibbles {
            match n {
                SHIFT_UP | SHIFT_DOWN => state = state.flip(),
                code if (code as usize) < 14 => out.push(table(state)[code as usize]),
                other => return Err(DecodeError::InvalidCode(other)),
            }
        }
        Ok(out)
    }
}

// ===========================================================================
// Variant 2: one shift code (0xF), 15 slots/state — 0xE reclaimed for data.
// Spare slots here are filled with a little punctuation to show the gain;
// space is still duplicated across both states.
// ===========================================================================
pub mod one_shift {
    use super::*;

    pub const SHIFT: u8 = 0xF;

    // 15 slots each (indices 0..=14), 30 total. Space is duplicated into both
    // states (costing 2 slots), the 26 letters fill 26, leaving exactly TWO
    // free data slots — here '.' and ',', both placed in state A. (Dropping
    // the second shift code frees 0xE per state vs the two-shift variant: that
    // is what creates these two slots once all 26 letters are retained.)
    pub const STATE_A: [char; 15] = [
        ' ', 'e', 't', 'a', 'o', 'i', 'n', 's', 'h', 'r', 'd', 'l', 'c', '.', ',',
    ];
    pub const STATE_B: [char; 15] = [
        ' ', 'u', 'm', 'w', 'f', 'g', 'y', 'p', 'b', 'v', 'k', 'j', 'x', 'q', 'z',
    ];

    fn table(state: State) -> &'static [char; 15] {
        match state {
            State::A => &STATE_A,
            State::B => &STATE_B,
        }
    }

    pub fn encode(input: &str) -> Result<(Vec<u8>, usize), EncodeError> {
        let mut state = State::A;
        let mut nibbles = Vec::with_capacity(input.len());
        for ch in input.chars() {
            if let Some(code) = find_in(table(state), ch) {
                nibbles.push(code);
            } else if let Some(code) = find_in(table(state.flip()), ch) {
                nibbles.push(SHIFT);
                state = state.flip();
                nibbles.push(code);
            } else {
                return Err(EncodeError::Unsupported(ch));
            }
        }
        Ok(pack_nibbles(&nibbles))
    }

    pub fn decode(bytes: &[u8], nibble_count: usize) -> Result<String, DecodeError> {
        let nibbles = unpack_nibbles(bytes, nibble_count);
        let mut state = State::A;
        let mut out = String::new();
        for &n in &nibbles {
            match n {
                SHIFT => state = state.flip(),
                code if (code as usize) < 15 => out.push(table(state)[code as usize]),
                other => return Err(DecodeError::InvalidCode(other)),
            }
        }
        Ok(out)
    }
}

// ===========================================================================
// Measurement: achieved bits-per-character, with Baudot (5 bpc) as reference.
// ===========================================================================

#[derive(Debug, Clone, Copy)]
pub struct Efficiency {
    pub chars: usize,
    pub nibbles: usize,
    pub bits_per_char: f64,
    pub baudot_bits_per_char: f64,
    /// Fraction of Baudot's size this scheme achieves (<1.0 means smaller).
    pub ratio_vs_baudot: f64,
}

/// Measure bits/char from a nibble count and the source string's char count.
/// Counts the *logical* nibble stream (4 bits each), which is the fair
/// comparison to Baudot's 5 bits/char; byte padding of a trailing nibble is a
/// storage detail, not part of the code's information cost.
pub fn efficiency(input: &str, nibble_count: usize) -> Efficiency {
    let chars = input.chars().count();
    let bits = nibble_count * 4;
    let bpc = if chars == 0 { 0.0 } else { bits as f64 / chars as f64 };
    const BAUDOT: f64 = 5.0;
    Efficiency {
        chars,
        nibbles: nibble_count,
        bits_per_char: bpc,
        baudot_bits_per_char: BAUDOT,
        ratio_vs_baudot: if chars == 0 { 0.0 } else { bpc / BAUDOT },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLES: &[&str] = &[
        /*"the quick brown fox jumps over the lazy dog",
        "hello world",
        "e",
        "",
        "zz top",
        "a man a plan a canal panama",
        "the cat sat on the mat",*/
        "non",
        "is art",
        "ten k drop",
        "self identifying",
        "tokens equal text",
        "type oposite images",
        "the ego and its owned",
        "the fractionalized phallus",
        "certificate of inauthenticity",
    ];

    #[test]
    fn two_shift_roundtrip() {
        for s in SAMPLES {
            let (bytes, len) = two_shift::encode(s).unwrap();
            assert_eq!(two_shift::decode(&bytes, len).unwrap(), *s, "two_shift {s:?}");
        }
    }

    #[test]
    fn one_shift_roundtrip() {
        for s in SAMPLES {
            let (bytes, len) = one_shift::encode(s).unwrap();
            assert_eq!(one_shift::decode(&bytes, len).unwrap(), *s, "one_shift {s:?}");
        }
    }

    #[test]
    fn one_shift_handles_extra_punctuation() {
        // One-shift adds exactly two marks ('.' and ',') over the letters+space set.
        let s = "hello, world. one more test, please.";
        let (bytes, len) = one_shift::encode(s).unwrap();
        assert_eq!(one_shift::decode(&bytes, len).unwrap(), s);
    }

    #[test]
    fn two_shift_rejects_unsupported() {
        assert_eq!(two_shift::encode("hi!"), Err(EncodeError::Unsupported('!')));
        // '.' is not in the two-shift alphabet either:
        assert_eq!(two_shift::encode("end."), Err(EncodeError::Unsupported('.')));
    }

    #[test]
    fn one_shift_rejects_unsupported() {
        assert_eq!(one_shift::encode("hi!"), Err(EncodeError::Unsupported('!')));
    }

    #[test]
    fn space_never_shifts() {
        // All-state-A words separated by spaces must produce zero shift codes.
        let (_, len) = two_shift::encode("the red horse").unwrap();
        // "the red horse" = 13 chars, all in state A, no shifts -> 13 nibbles.
        assert_eq!(len, 13);
    }

    #[test]
    fn odd_nibble_padding_roundtrips() {
        // Odd nibble count exercises the trailing-padding path.
        let (bytes, len) = two_shift::encode("ate").unwrap(); // 3 nibbles, all state A
        assert_eq!(len, 3);
        assert_eq!(bytes.len(), 2); // 3 nibbles -> 2 bytes, last half padded
        assert_eq!(two_shift::decode(&bytes, len).unwrap(), "ate");
    }

    #[test]
    fn efficiency_beats_baudot_on_prose() {
        // Ordinary prose (state-A dominant), NOT a pangram — pangrams are an
        // adversarial worst case that exceed 5 bpc because they force shifts to
        // every rare state-B letter.
        let s = "the cat sat on the mat as the red hen ran to the den";
        let (_, len2) = two_shift::encode(s).unwrap();
        let e2 = efficiency(s, len2);
        let (_, len1) = one_shift::encode(s).unwrap();
        let e1 = efficiency(s, len1);
        assert!(e2.bits_per_char < 5.0, "two_shift bpc = {}", e2.bits_per_char);
        assert!(e1.bits_per_char < 5.0, "one_shift bpc = {}", e1.bits_per_char);
    }

    #[test]
    fn pangram_is_worst_case_and_loses() {
        // Documents the tradeoff explicitly: rare-letter-dense text does worse
        // than Baudot. This is expected, not a bug.
        let s = "the quick brown fox jumps over the lazy dog";
        let (_, len) = two_shift::encode(s).unwrap();
        assert!(efficiency(s, len).bits_per_char > 5.0);
    }
}
