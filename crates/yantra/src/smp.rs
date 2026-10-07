//! **संपुट (Saṃpuṭa) v0 — the self-decoding archive, read by `yantra-run` itself.**
//!
//! The format is NORMATIVE IN THE ARCHIVE-PROJECT REPOSITORY, not here: `research/sampputa-v0-spec.md`
//! and `reference/sampputa.py` at the archive project `8fe6e9f` (the `drsn-v4.0` release, 2026-10-06 — the
//! version the vectors in `tests/data/sampputa/` were copied from; amended since `3e13126`, doc
//! only, at `ea43cb5`, the payload kind is unchecked, and `a23fc47`, the decoder gets no
//! arguments). This module is a reader of that page and nothing more; where this comment and
//! the page differ, the page wins and this is a bug. It exists for the owner's ruling "both" on the spec's §6 (sansos `F-022`): the archive
//! runs through the archive project's `smp-run` AND through `yantra-run <file.smp>` directly.
//!
//! Layout, every integer big-endian and unsigned:
//!
//! ```text
//! 0        4   "SMPT"
//! 4        1   version            0
//! 5        1   flags              0
//! 6        2   header length H    52
//! 8        4   decoder length D
//! 12       32  decoder SHA-256
//! 44       8   step cap           a reader may LOWER it, never raise it
//! 52       D   the decoder ELF
//! 52+D     4   payload kind       ANY four octets, a label for people and tools; a v0 reader
//!                                  refuses nothing on it (spec §2 as amended at `ea43cb5`)
//! 56+D     4   input name length N
//! 60+D     N   input name         UTF-8; becomes YANTRA_INPUT_NAME
//! 60+D+N   8   payload length P
//! 68+D+N   P   the payload        becomes the input channel, unchanged
//! last     32  SHA-256 of every octet before it
//! ```
//!
//! The file is EXACTLY `100 + D + N + P` octets. [`parse`] applies the refusals S1..S8 IN THE
//! SPEC'S ORDER and returns the FIRST one met, so two readers refuse one damaged file with the
//! same cause. Nothing here runs an instruction: a refusal is decided on octets alone, before
//! any [`crate::Machine`] exists.
//!
//! **WHAT `yantra-run` EXPOSES OF THIS (the coordinator's rulings, 2026-10-05):** it calls
//! [`parse`] with the owner's cap [`DECODER_CAP`] and NO allow-list. S8 and a LOWER decoder cap
//! are supported here (the `allow` and `cap` arguments) and have no flag in `yantra-run` yet.
//! The decoder receives exactly what both reference readers give it: the payload as
//! `YANTRA_INPUT`, the name as `YANTRA_INPUT_NAME`, the step ceiling as `YANTRA_STEPS`, and
//! `YANTRA_RAM` if set; no argument (trailing ones are refused, and the argument interface is
//! handed only argv[0], the archive's path, as for any image) and no event log. The spec
//! makes argv[0] the reader's choice (the archive project a23fc47, Saṃpuṭa v0 §4: "The decoder is given no
//! arguments. A reader may pass any argv[0] (the archive's path, a temporary path); a decoder
//! must not depend on it. The input reaches it only through the named input.").

/// The four octets a Saṃpuṭa file starts with. A DRSN file starts "DRSN" and an ELF `7F 45`,
/// so `yantra-run` can tell the three apart on the first word.
pub const MAGIC: &[u8; 4] = b"SMPT";
/// The header length v0 defines: the offset of the decoder image.
pub const HEADER: u16 = 52;
/// The fixed octets around the three variable spans: 52 + 4 + 4 + 8 + 32.
pub const FIXED: u64 = 100;
/// The owner's ONE decoder cap, image and audio, 2026-10-04 (the archive project spec v2 decision 6). A
/// reader may run under a LOWER cap; [`parse`] clamps any higher one down to this.
pub const DECODER_CAP: u64 = 98_304;
/// The longest input name, in octets.
pub const NAME_MAX: u64 = 255;

/// A refusal, by the spec's number. [`Refusal::exit_code`] is the reference reader's
/// `80 + k`, which `yantra-run` matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Refusal {
    /// Shorter than 100 octets, or the magic is not "SMPT".
    S1,
    /// Version, flags or header length not v0's.
    S2,
    /// The declared lengths do not account for the file exactly.
    S3,
    /// The file's SHA-256 does not match.
    S4,
    /// The decoder's SHA-256 does not match its image.
    S5,
    /// The decoder is larger than the cap in force.
    S6,
    /// The input name is empty, longer than 255 octets, not strict UTF-8, or holds U+0000.
    S7,
    /// The decoder is not on the reader's allow-list.
    S8,
}

impl Refusal {
    /// `k` in `Sk`.
    #[must_use]
    pub fn number(self) -> u8 {
        match self {
            Self::S1 => 1,
            Self::S2 => 2,
            Self::S3 => 3,
            Self::S4 => 4,
            Self::S5 => 5,
            Self::S6 => 6,
            Self::S7 => 7,
            Self::S8 => 8,
        }
    }

    /// The reference reader's exit status: `80 + k` (81..88).
    ///
    /// THESE ARE `yantra-run`'S OWN EXIT CODES, A SEPARATE NAMESPACE FROM QEMU'S
    /// (`W-381`, 2026-10-06). Under QEMU a guest's FAIL-form finisher word
    /// `0x3333 | n << 16` becomes the QEMU PROCESS's exit status, the low octet of
    /// `n` — so the `W-355` refusal `0x355` exits 85 there, the same number as S5
    /// here. No one tool reports both meanings: `yantra-run` maps every non-zero
    /// finisher status to exit 1, never to `n`, and only an archive refusal exits
    /// 81..88. The stack canary's `0x353B` (QEMU exit 59) was chosen clear of both
    /// anyway (`crates/yantra/tests/v009_stack_bss.rs`).
    #[must_use]
    pub fn exit_code(self) -> u8 {
        80 + self.number()
    }

    /// The reference reader's words for the cause (`WHY` in `sampputa.py`).
    #[must_use]
    pub fn why(self) -> &'static str {
        match self {
            Self::S1 => "shorter than 100 octets, or not SMPT",
            Self::S2 => "version, flags or header length not v0's",
            Self::S3 => "the declared lengths do not account for the file exactly",
            Self::S4 => "the file's SHA-256 does not match: the file has been changed",
            Self::S5 => "the decoder's SHA-256 does not match its image",
            Self::S6 => "the decoder is larger than the cap",
            Self::S7 => "the input name is empty, longer than 255 octets, or not UTF-8",
            Self::S8 => "the decoder is not on this reader's allow-list",
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S{}: {}", self.number(), self.why())
    }
}

/// A verified archive's parts, borrowed from the file. ONLY `decoder`, `name`, `payload` and
/// `step_cap` reach the run; the rest is for saying what is about to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Archive<'a> {
    /// The decoder ELF, D octets.
    pub decoder: &'a [u8],
    /// Its SHA-256, as the header states it (and S5 confirmed).
    pub decoder_sha: [u8; 32],
    /// The writer's step cap. A run may go LOWER, never higher.
    pub step_cap: u64,
    /// The payload kind's four octets, as written.
    pub kind: [u8; 4],
    /// The input name: the decoder's `YANTRA_INPUT_NAME`.
    pub name: &'a str,
    /// The payload: the decoder's input channel, unchanged.
    pub payload: &'a [u8],
}

/// Whether `file` claims to be a Saṃpuṭa archive: its first four octets are [`MAGIC`]. A file
/// that claims it and is short or broken is still an archive to refuse, never an ELF to try.
#[must_use]
pub fn is_archive(file: &[u8]) -> bool {
    file.starts_with(MAGIC)
}

fn be(b: &[u8]) -> u64 {
    b.iter().fold(0u64, |v, &x| (v << 8) | u64::from(x))
}

/// Checks S1..S8 IN ORDER and returns the first refusal met, or the parts.
///
/// `cap` is the reader's decoder cap; anything above [`DECODER_CAP`] is clamped down to it, so a
/// caller can LOWER the cap and never raise it. `allow`, when given, is S8's allow-list of
/// decoder SHA-256s; `None` skips S8.
///
/// # Errors
/// The first of S1..S8 the file fails.
pub fn parse<'a>(
    file: &'a [u8],
    cap: u64,
    allow: Option<&[[u8; 32]]>,
) -> Result<Archive<'a>, Refusal> {
    let n = file.len() as u64;
    // S1
    if n < FIXED || !is_archive(file) {
        return Err(Refusal::S1);
    }
    // S2
    if file[4] != 0 || file[5] != 0 || be(&file[6..8]) != u64::from(HEADER) {
        return Err(Refusal::S2);
    }
    // S3 — THE LENGTH WALK, as the spec states it: each field is read only where the file holds
    // it, and each sum is compared with `n` BEFORE the next term is added. `D` and `N` are at
    // most 2^32 - 1 and `n` is a `usize`, so `FIXED + D + N` cannot overflow a `u64`; `P` is
    // compared by SUBTRACTION from `n`, so a `P` near 2^64 cannot wrap the sum into a match.
    let d = be(&file[8..12]);
    if FIXED + d > n {
        return Err(Refusal::S3);
    }
    let du = d as usize; // ≤ n, which is a usize
    let name_len = be(&file[56 + du..60 + du]);
    if FIXED + d + name_len > n {
        return Err(Refusal::S3);
    }
    let nu = name_len as usize;
    let p = be(&file[60 + du + nu..68 + du + nu]);
    if p != n - (FIXED + d + name_len) {
        return Err(Refusal::S3);
    }
    let nn = file.len();
    // S4
    if sha256(&file[..nn - 32]) != file[nn - 32..] {
        return Err(Refusal::S4);
    }
    // S5
    let decoder = &file[52..52 + du];
    let mut decoder_sha = [0u8; 32];
    decoder_sha.copy_from_slice(&file[12..44]);
    if sha256(decoder) != decoder_sha {
        return Err(Refusal::S5);
    }
    // S6 — a reader may LOWER the cap, never raise it.
    if d > cap.min(DECODER_CAP) {
        return Err(Refusal::S6);
    }
    // S7 — strict UTF-8 is `std::str::from_utf8`'s (RFC 3629: no overlong forms, no
    // surrogates, nothing above U+10FFFF); U+0000 because no host can pass a NUL in an
    // environment value.
    let name_octets = &file[60 + du..60 + du + nu];
    if name_len == 0 || name_len > NAME_MAX || name_octets.contains(&0) {
        return Err(Refusal::S7);
    }
    let Ok(name) = std::str::from_utf8(name_octets) else {
        return Err(Refusal::S7);
    };
    // S8 — the reader's own policy.
    if let Some(list) = allow
        && !list.contains(&decoder_sha)
    {
        return Err(Refusal::S8);
    }
    let mut kind = [0u8; 4];
    kind.copy_from_slice(&file[52 + du..56 + du]);
    Ok(Archive {
        decoder,
        decoder_sha,
        step_cap: be(&file[44..52]),
        kind,
        name,
        payload: &file[68 + du + nu..nn - 32],
    })
}

/// The file's octets for these parts — the reference's `pack`, without its argument checks
/// (a test builds files a reader must refuse with it, as `gen_sampputa.py`'s `raw` does).
#[must_use]
pub fn pack(decoder: &[u8], payload: &[u8], kind: [u8; 4], name: &[u8], steps: u64) -> Vec<u8> {
    let mut b = Vec::with_capacity(100 + decoder.len() + name.len() + payload.len());
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(&[0, 0]);
    b.extend_from_slice(&HEADER.to_be_bytes());
    b.extend_from_slice(&(decoder.len() as u32).to_be_bytes());
    b.extend_from_slice(&sha256(decoder));
    b.extend_from_slice(&steps.to_be_bytes());
    b.extend_from_slice(decoder);
    b.extend_from_slice(&kind);
    b.extend_from_slice(&(name.len() as u32).to_be_bytes());
    b.extend_from_slice(name);
    b.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    b.extend_from_slice(payload);
    let seal = sha256(&b);
    b.extend_from_slice(&seal);
    b
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn compress(h: &mut [u32; 8], block: &[u8]) {
    let mut w = [0u32; 64];
    for (i, wi) in w.iter_mut().enumerate().take(16) {
        *wi = u32::from_be_bytes([
            block[4 * i],
            block[4 * i + 1],
            block[4 * i + 2],
            block[4 * i + 3],
        ]);
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
}

/// SHA-256 of `data`, FIPS 180-4. Pure Rust and no crate, per doc 03 §1 (this crate uses none).
#[must_use]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut blocks = data.chunks_exact(64);
    for block in &mut blocks {
        compress(&mut h, block);
    }
    // The tail: the leftover octets, 0x80, zeros to 56 mod 64, the bit length.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

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
        assert_eq!(
            hex(&sha256(&vec![b'a'; 1_000_000])),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    /// 55, 56 and 64 octets: where the length does or does not fit the last block.
    #[test]
    fn sha256_handles_the_padding_boundaries() {
        assert_eq!(
            hex(&sha256(&[b'a'; 55])),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            hex(&sha256(&[b'a'; 56])),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            hex(&sha256(&[b'a'; 64])),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    #[test]
    fn exit_codes_are_the_references() {
        assert_eq!(Refusal::S1.exit_code(), 81);
        assert_eq!(Refusal::S8.exit_code(), 88);
    }
}
