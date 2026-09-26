//! The boot message in `spec/boot-sbi.sas` is SLP1, and it is not typed.
//!
//! Doc 11 §7.1.1 asks for नमस्ते over the serial line "in SLP1 (pre-font)".
//! A boot payload runs before any shaper exists, so the message cannot be
//! Devanagari on the wire — and Sassembly source cannot be ASCII either
//! (doc 15 R-15-1; the lexer rejects `n` outside the repertoire). What is
//! left is a list of octets, which is the one form where a slip is invisible:
//! `११५` and `११३` look alike and produce `s` and `q`.
//!
//! So the octets are not evidence of anything by themselves. This test makes
//! them evidence, by requiring them to equal what `sanskrit_text::slp1`
//! derives from the Devanagari — the same discipline as the expected bytes in
//! `spec/golden/`, which are never hand-assembled either.

use std::fmt::Write as _;

/// The message the boot program prints, in the script it is written in.
///
/// This is the only authored half. Everything after it is derived.
const MESSAGE: &str = "नमस्ते संसार";

/// The octet list from the `॥ अष्टकाः … ॥` directive in a `.sas` source.
///
/// Devanagari digits are what the file holds, so they are what is read; a
/// parser that accepted ASCII here would accept a file the assembler will not.
fn octets(src: &str) -> Vec<u8> {
    let line = src
        .lines()
        .find(|l| l.starts_with("॥ अष्टकाः"))
        .expect("spec/boot-sbi.sas has an अष्टकाः directive");

    line.split_whitespace()
        .filter(|w| w.chars().all(|c| ('\u{0966}'..='\u{096F}').contains(&c)))
        .map(|w| {
            let ascii: String = w
                .chars()
                .map(|c| char::from(b'0' + (c as u32 - 0x0966) as u8))
                .collect();
            ascii.parse::<u8>().expect("an octet fits in a byte")
        })
        .collect()
}

#[test]
fn the_boot_message_is_the_slp1_of_the_devanagari() {
    let mut slp1 = String::new();
    sanskrit_text::slp1::encode_into(MESSAGE, &mut slp1).expect("the message is in the domain");
    // A line ending so the firmware's own banner does not run into ours, and a
    // terminator because the print loop stops on a zero rather than on a count.
    let mut want = slp1.into_bytes();
    want.push(b'\n');
    want.push(0);

    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/boot-sbi.sas"
    ))
    .expect("spec/boot-sbi.sas");

    assert_eq!(
        octets(&src),
        want,
        "the octets in spec/boot-sbi.sas are not the SLP1 of {MESSAGE}"
    );
}

#[test]
fn the_slp1_round_trips_back_to_the_devanagari() {
    // Bijection is the whole reason SLP1 was chosen (doc 01 §6). Asserting the
    // octets alone would pass just as well against a lossy encoder, so the
    // return journey is what says the message survived being made printable.
    let mut slp1 = String::new();
    sanskrit_text::slp1::encode_into(MESSAGE, &mut slp1).expect("encodes");
    let mut back = String::new();
    sanskrit_text::slp1::decode_into(&slp1, &mut back).expect("decodes");
    assert_eq!(back, MESSAGE);

    // And it is ASCII, which is the property that makes it printable with no
    // font, no shaper and no table at all.
    let mut rendered = String::new();
    write!(rendered, "{slp1}").unwrap();
    assert!(
        rendered.is_ascii(),
        "SLP1 on the wire must be ASCII: {rendered}"
    );
}
