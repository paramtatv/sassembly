//! **`W-381` — WHAT AN ENGINE CAME TO, AND WHEN TWO OF THEM AGREE.**
//!
//! ONE statement of the agreement rule, used by BOTH of its readers (owner ruling
//! on `W-381` Part B, 2026-10-06):
//!
//! - `t1_image`'s differential gate, which compares the interpreted predict with
//!   the built image run natively by `yantra-run` and refuses the build when they
//!   disagree;
//! - `crates/yantra/tests/w381_engine_agreement.rs`, the ratchet that records
//!   every known interpreter/native mismatch.
//!
//! Two copies of this rule would be two policies the first time one is edited, so
//! the ratchet imports [`Run`] and [`agree`] from here rather than keeping its own.

/// The finisher carries a status in the 48 bits above its 16-bit code.
pub const STATUS_MASK: u128 = (1 << 48) - 1;

/// The native refusals that are the SAME refusal as an interpreter's: the code a
/// refusal block stores in FAIL form (so `yantra` reports it as the status), and
/// the phrase the interpreter's refusal carries.
///
/// The codes are `ir.t1`'s `रिक्तखण्डपठननिषेधः` (`W-355`), `प्राचलसीमानिषेधः`
/// (`W-359`), `व्यूहदैर्घ्यनिषेधः` (`V-008`, [`crate::t1::nirvahana::VECTOR_LENGTH_REFUSAL`])
/// and `अध्यासप्रतिषेधः` (`V-009` (ii), an aliased matrix result,
/// [`crate::t1::nirvahana::REFUSAL_ADHYASA`]), and `W-381` stage 3's two: a
/// checked built-in's overflow ([`crate::t1::nirvahana::OVERFLOW_REFUSAL`], `0x35c`)
/// and a division or remainder by zero
/// ([`crate::t1::nirvahana::DIVISION_BY_ZERO_REFUSAL`], `0x35e`).
///
/// `W-381` STAGE 4 (O4, 2026-10-05; reads AND writes, index −1 included) WIDENED
/// `0x355` from `W-355`'s nil run ("entry 0 is outside an arena of 0") to every
/// out-of-bounds index, with NO new code: the native bound check branches to the
/// same refusal block, and the interpreter's refusal is the same refusal — an
/// arena read past its length ("entry i is outside an arena of n"), an octet read
/// ("octet i is outside a run of n octets"). One code, two phrasings.
///
/// An out-of-bounds STORE is its OWN refusal, `0x35d` (owner ruling 2026-10-06:
/// "Assign a dedicated refusal. Split from 0x355", `ir.t1`'s
/// `सीमातीतलेखननिषेधः`, [`crate::t1::nirvahana::OUT_OF_BOUNDS_WRITE_REFUSAL`]):
/// natively a negative index, which the interpreter refuses as "arena index −1
/// is not an index (W-381, finisher word 0x35d)" (or "octet index …"), and
/// a store at or past the largest run ("entry i is past the largest run, …
/// (W-381, finisher word 0x35d)").
pub const NATIVE_REFUSALS: [(u64, &str); 8] = [
    (0x355, "is outside an arena of"),
    (0x355, "is outside a run of"),
    (0x35d, "(W-381, finisher word 0x35d)"),
    (0x359, "W-359"),
    (0x35a, "0x35a"),
    (0x35b, "0x35b"),
    (0x35c, "0x35c"),
    (0x35e, "0x35e"),
];

/// One engine's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Run {
    /// Finished with a status and the octets it printed. The interpreter's status
    /// is its `Int` answer; the native one is the finisher's decoded status.
    Ran { status: i128, out: Vec<u8> },
    /// Finished, but left part of an event log undelivered.
    Leftover {
        status: i128,
        delivered: usize,
        out: Vec<u8>,
    },
    /// The interpreter answered something that is not an integer.
    Answered(String),
    /// The interpreter refused while running: the first line of its reason, and
    /// the octets it had printed before refusing.
    Refused { why: String, out: Vec<u8> },
    /// The interpreter refused to load the program.
    NotLoaded(String),
    /// The event interface was refused before the run (either engine).
    NoEvents(String),
    /// A finisher value in neither the PASS nor the FAIL form (no decoded status).
    Code(u64),
    /// Any other native halt, without its `pc` (which moves with the code).
    Fault(String),
    /// The run asked to WAIT and no event source was given: the interpreter's
    /// "no event source" refusal, `yantra-run`'s halt `Wait` (exit 75). Paused,
    /// not finished, with the octets printed so far.
    Paused { out: Vec<u8> },
    /// The image was not built (compile, resolve or link refused).
    NotBuilt(String),
    /// The run did not END: the interpreter's fuel or the machine's step budget
    /// ran out, or the runner gave no verdict. Never an agreement: nothing was
    /// compared.
    Inconclusive(String),
    /// This engine has no part in the comparison.
    Absent,
}

/// THE AGREEMENT RULE, interpreter against one native engine.
///
/// Equal finishes agree when the native status equals the interpreter's answer
/// cut to the finisher's 48 bits and the printed octets are equal. Refusals agree
/// only when they are THE SAME refusal: a native FAIL-form status in
/// [`NATIVE_REFUSALS`] against the interpreter refusal carrying its phrase, with
/// the native side having printed exactly what the interpreter printed before it
/// refused (a program that prints and then RETURNS 853 halts with the W-355
/// refusal's own word — the FAIL form's accepted trade — so only the octets tell
/// them apart); a load refusal against a build refusal; the event interface
/// refused on both. A native fault never agrees with an interpreter refusal,
/// because the native side did not refuse: it read or wrote the wrong word. An
/// [`Run::Inconclusive`] side never agrees.
#[must_use]
pub fn agree(interp: &Run, native: &Run) -> bool {
    let cut = |s: i128| (s as u128) & STATUS_MASK;
    match (interp, native) {
        (Run::Ran { status: a, out: x }, Run::Ran { status: b, out: y }) => {
            cut(*a) == *b as u128 && x == y
        }
        (
            Run::Leftover {
                status: a,
                delivered: d,
                out: x,
            },
            Run::Leftover {
                status: b,
                delivered: e,
                out: y,
            },
        ) => cut(*a) == *b as u128 && d == e && x == y,
        (Run::Refused { why, out: x }, Run::Ran { status, out: y }) if x == y => NATIVE_REFUSALS
            .iter()
            .any(|(code, phrase)| *status == i128::from(*code) && why.contains(phrase)),
        (Run::Paused { out: x }, Run::Paused { out: y }) => x == y,
        (Run::NotLoaded(_), Run::NotBuilt(_)) | (Run::NoEvents(_), Run::NoEvents(_)) => true,
        (Run::Inconclusive(_), _) | (_, Run::Inconclusive(_)) => false,
        (_, Run::Absent) => true,
        _ => false,
    }
}

/// Whether two NATIVE outcomes (the `.t1` chain's image and the Rust twin's)
/// agree: equal, or both refused at build whatever their texts (the two builders
/// word their refusals differently), or the second absent.
#[must_use]
pub fn natives_agree(a: &Run, b: &Run) -> bool {
    *b == Run::Absent || a == b || matches!((a, b), (Run::NotBuilt(_), Run::NotBuilt(_)))
}

/// SHA-256 of `data`, FIPS 180-4, for `t1_image`'s provenance sidecar. A copy of
/// `yantra::smp::sha256` (this crate does not depend on `yantra`), held to it by
/// `crates/yantra/tests/w381_engine_agreement.rs`, which hashes the same inputs
/// with both and with the FIPS vectors below.
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let compress = |h: &mut [u32; 8], block: &[u8]| {
        let mut w = [0u32; 64];
        for (i, c) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    };
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut blocks = data.chunks_exact(64);
    for block in &mut blocks {
        compress(&mut h, block);
    }
    let rest = blocks.remainder();
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let end = if rest.len() < 56 { 64 } else { 128 };
    tail[end - 8..end].copy_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    for block in tail[..end].chunks_exact(64) {
        compress(&mut h, block);
    }
    let mut out = [0u8; 32];
    for (o, x) in out.chunks_exact_mut(4).zip(h) {
        o.copy_from_slice(&x.to_be_bytes());
    }
    out
}

/// Lower-case hex of `b`.
#[must_use]
pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FIPS 180-4 / NIST CAVP values, not produced by this implementation.
    #[test]
    fn sha256_matches_the_fips_180_4_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn a_refusal_agrees_only_with_its_own_status_and_the_same_octets() {
        let refused = |out: &[u8]| Run::Refused {
            why: "entry 0 is outside an arena of 0".into(),
            out: out.to_vec(),
        };
        let ran = |s: i128, out: &[u8]| Run::Ran {
            status: s,
            out: out.to_vec(),
        };
        assert!(agree(&refused(b""), &ran(0x355, b"")));
        assert!(!agree(&refused(b""), &ran(0x355, b"X")));
        assert!(!agree(&refused(b""), &ran(0x359, b"")));
        assert!(!agree(&refused(b""), &Run::Code(0x355)));
        // `W-381` stage 4: the bound and the negative store are the same refusal.
        for why in [
            "entry 5 is outside an arena of 2",
            "entry -1 is outside an arena of 2",
            "octet 5 is outside a run of 2 octets",
        ] {
            let r = Run::Refused {
                why: why.into(),
                out: Vec::new(),
            };
            assert!(agree(&r, &ran(0x355, b"")), "{why}");
            assert!(!agree(&r, &ran(0x359, b"")), "{why}");
            assert!(!agree(&r, &ran(0x35d, b"")), "{why}");
        }
        for why in [
            "arena index -1 is not an index (W-381, finisher word 0x35d)",
            "octet index -1 is not an index (W-381, finisher word 0x35d)",
            "entry 1000000000000 is past the largest run, 33554432 entries (W-381, finisher word 0x35d)",
        ] {
            let r = Run::Refused {
                why: why.into(),
                out: Vec::new(),
            };
            assert!(agree(&r, &ran(0x35d, b"")), "{why}");
            assert!(!agree(&r, &ran(0x355, b"")), "{why}");
        }
        assert!(!agree(
            &Run::Refused {
                why: "expected a number, found Nil".into(),
                out: Vec::new()
            },
            &ran(0x355, b"")
        ));
        assert!(agree(&ran(-1, b""), &ran((1 << 48) - 1, b"")));
        assert!(!agree(&ran(1 << 64, b""), &ran(1, b"")));
        assert!(!agree(
            &Run::Inconclusive("fuel".into()),
            &Run::Inconclusive("steps".into())
        ));
    }
}
