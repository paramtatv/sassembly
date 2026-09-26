//! Task `A-020` — SLP1 bijection property test.
//!
//! Doc 01 §8 sets the bar at **10^6 generated akṣara strings round-tripping
//! byte-identically**. Bijectivity is what makes SLP1 safe as an input-method
//! and panic-output surface: if `deva → slp1 → deva` ever loses information, a
//! typist's keystrokes silently produce a different identifier than intended.
//!
//! Both directions are checked, because injectivity in one direction is not
//! surjectivity in the other:
//!
//! - `deva → slp1 → deva == deva`  (no Devanagari detail is lost)
//! - `slp1 → deva → slp1 == slp1`  (no SLP1 spelling is unreachable)
//!
//! The generator is a deterministic LCG rather than a random source, so a
//! failure is reproducible from its seed alone.

use sanskrit_text::slp1::{Buf, Slp1Error, decode_into, encode_into};

const CONSONANTS: &[char] = &[
    '\u{0915}', '\u{0916}', '\u{0917}', '\u{0918}', '\u{0919}', '\u{091A}', '\u{091B}', '\u{091C}',
    '\u{091D}', '\u{091E}', '\u{091F}', '\u{0920}', '\u{0921}', '\u{0922}', '\u{0923}', '\u{0924}',
    '\u{0925}', '\u{0926}', '\u{0927}', '\u{0928}', '\u{092A}', '\u{092B}', '\u{092C}', '\u{092D}',
    '\u{092E}', '\u{092F}', '\u{0930}', '\u{0932}', '\u{0933}', '\u{0935}', '\u{0936}', '\u{0937}',
    '\u{0938}', '\u{0939}',
];
const VOWELS: &[char] = &[
    '\u{0905}', '\u{0906}', '\u{0907}', '\u{0908}', '\u{0909}', '\u{090A}', '\u{090B}', '\u{0960}',
    '\u{090C}', '\u{0961}', '\u{090F}', '\u{0910}', '\u{0913}', '\u{0914}',
];
const MATRAS: &[char] = &[
    '\u{093E}', '\u{093F}', '\u{0940}', '\u{0941}', '\u{0942}', '\u{0943}', '\u{0944}', '\u{0962}',
    '\u{0963}', '\u{0947}', '\u{0948}', '\u{094B}', '\u{094C}',
];
const MARKS: &[char] = &['\u{0902}', '\u{0903}', '\u{0901}'];
const SIGNS: &[char] = &[
    '\u{093D}', '\u{0964}', '\u{0965}', '\u{0970}', '\u{0971}', '\u{0950}', '\u{093C}', '\u{0951}',
    '\u{0952}', ' ',
];
const DIGITS: &[char] = &[
    '\u{0966}', '\u{0967}', '\u{0968}', '\u{0969}', '\u{096A}', '\u{096B}', '\u{096C}', '\u{096D}',
    '\u{096E}', '\u{096F}',
];
const VIRAMA: char = '\u{094D}';
const ZWNJ: char = '\u{200C}';
const ZWJ: char = '\u{200D}';

/// Deterministic LCG — a failing case is reproducible from its seed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() as usize) % xs.len()]
    }
    fn upto(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Build one well-formed akṣara, staying inside the domain the bijection claims
/// (module docs in `slp1.rs`): no virāma before an independent vowel, no
/// dangling marks.
///
/// Returns whether this akṣara ended in a virāma. The caller must not follow a
/// virāma-final akṣara with an independent vowel — that is the one genuinely
/// ambiguous sequence, and the encoder rejects it by design.
fn push_akshara(rng: &mut Rng, out: &mut String, prev_dead: bool) -> bool {
    let choice = if prev_dead {
        // after a dead consonant, anything except a vowel-initial akṣara
        2 + rng.upto(8)
    } else {
        rng.upto(10)
    };
    match choice {
        0..=1 => {
            out.push(*rng.pick(VOWELS));
            if rng.upto(4) == 0 {
                out.push(*rng.pick(MARKS));
            }
            false
        }
        2 => {
            out.push(*rng.pick(SIGNS));
            false
        }
        3 => {
            out.push(*rng.pick(DIGITS));
            false
        }
        _ => {
            // consonant cluster: C (virāma [joiner] C)* then mātrā | virāma | nothing
            out.push(*rng.pick(CONSONANTS));
            for _ in 0..rng.upto(3) {
                out.push(VIRAMA);
                match rng.upto(6) {
                    0 => out.push(ZWNJ),
                    1 => out.push(ZWJ),
                    _ => {}
                }
                out.push(*rng.pick(CONSONANTS));
            }
            let mut dead = false;
            match rng.upto(6) {
                0 => {
                    out.push(VIRAMA); // dead consonant
                    dead = true;
                }
                1..=3 => out.push(*rng.pick(MATRAS)),
                _ => {}
            }
            if !dead && rng.upto(5) == 0 {
                out.push(*rng.pick(MARKS));
            }
            dead
        }
    }
}

fn round_trip(deva: &str) -> Result<String, Slp1Error> {
    let mut mid = String::new();
    encode_into(deva, &mut mid)?;
    let mut back = String::new();
    decode_into(&mid, &mut back)?;
    Ok(back)
}

#[test]
fn deva_slp1_deva_is_identity_over_a_million_strings() {
    const N: usize = 1_000_000;
    let mut rng = Rng(0x5A_4E_53_4F_53); // "SNSOS"
    let mut checked = 0usize;
    let mut deva = String::new();

    for i in 0..N {
        deva.clear();
        let mut dead = false;
        for _ in 0..=rng.upto(6) {
            dead = push_akshara(&mut rng, &mut deva, dead);
        }
        match round_trip(&deva) {
            Ok(back) => {
                assert_eq!(
                    back, deva,
                    "round trip lost information at iteration {i}\n  in : {:?}\n  out: {:?}",
                    deva, back
                );
                checked += 1;
            }
            Err(e) => panic!("iteration {i}: generator produced out-of-domain {deva:?}: {e:?}"),
        }
    }
    assert_eq!(checked, N);
}

#[test]
fn slp1_deva_slp1_is_identity() {
    // The other direction: every SLP1 spelling this profile emits must be
    // reachable, or some keystroke sequence would be unreachable by the IME.
    const N: usize = 200_000;
    let mut rng = Rng(0xC0FFEE);
    let mut deva = String::new();

    for i in 0..N {
        deva.clear();
        let mut dead = false;
        for _ in 0..=rng.upto(5) {
            dead = push_akshara(&mut rng, &mut deva, dead);
        }
        let mut slp = String::new();
        encode_into(&deva, &mut slp).unwrap();

        let mut back_deva = String::new();
        decode_into(&slp, &mut back_deva).unwrap();
        let mut back_slp = String::new();
        encode_into(&back_deva, &mut back_slp).unwrap();

        assert_eq!(
            back_slp, slp,
            "SLP1 spelling not stable at iteration {i}: {slp:?}"
        );
    }
}

/// Distinct Devanagari must never collapse to the same SLP1 — injectivity, the
/// property that stops two identifiers becoming one symbol.
#[test]
fn encoding_is_injective_on_a_large_sample() {
    use std::collections::HashMap;
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut rng = Rng(0xBEEF);
    let mut deva = String::new();

    for _ in 0..200_000 {
        deva.clear();
        let mut dead = false;
        for _ in 0..=rng.upto(3) {
            dead = push_akshara(&mut rng, &mut deva, dead);
        }
        let mut slp = String::new();
        encode_into(&deva, &mut slp).unwrap();
        if let Some(prev) = seen.get(&slp) {
            assert_eq!(
                *prev, deva,
                "COLLISION: {prev:?} and {deva:?} both encode to {slp:?}"
            );
        } else {
            seen.insert(slp, deva.clone());
        }
    }
}

#[test]
fn fixed_buffer_path_matches_the_allocating_path() {
    // Panic output uses Buf; it must agree with the String path exactly.
    let mut rng = Rng(0xD00D);
    let mut deva = String::new();
    let mut raw = [0u8; 512];

    for _ in 0..10_000 {
        deva.clear();
        let mut dead = false;
        for _ in 0..=rng.upto(4) {
            dead = push_akshara(&mut rng, &mut deva, dead);
        }
        let mut s = String::new();
        encode_into(&deva, &mut s).unwrap();

        let mut b = Buf::new(&mut raw);
        encode_into(&deva, &mut b).unwrap();
        assert_eq!(b.as_str(), s);
    }
}
