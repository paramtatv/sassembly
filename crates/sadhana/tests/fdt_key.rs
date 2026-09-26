//! The name `spec/fdt-header.sas` looks for in the device tree is not typed.
//!
//! `C-001b2b2` matches a node name during the walk, and the name it matches
//! against belongs to the device tree specification rather than to us: the
//! node is `/memory`, spelled in ASCII, because that is what a flattened tree
//! holds. Sassembly source cannot be ASCII (doc 15 R-15-1; the lexer rejects
//! `m` outside the repertoire), so the key is a list of octets — and that is
//! the one form where a slip is invisible: `१०९` and `१०८` look alike and
//! produce `m` and `l`, and a key of `lemory` would simply never match, on
//! every machine, silently and for ever.
//!
//! So the octets are made evidence here, the same discipline `boot_slp1.rs`
//! applies to the boot message and `spec/golden/` applies to expected bytes.

/// The node name the walk looks for, and one of the two authored halves here.
const KEY: &str = "memory";

/// The property name it then looks for inside that node (`C-001b2b3`), and the
/// other. This one is spelled in a second block: the structure block holds only
/// its offset, so an octet wrong here fails no differently — silently, for ever.
const PROP: &str = "reg";

/// Every octet list from the `॥ अष्टकाः … ॥` directives in a `.sas` source, in
/// the order the file writes them.
///
/// Devanagari digits are what the file holds, so they are what is read; a
/// parser that accepted ASCII here would accept a file the assembler will not.
fn octet_lists(src: &str) -> Vec<Vec<u8>> {
    let lists: Vec<Vec<u8>> = src
        .lines()
        .filter(|l| l.starts_with("॥ अष्टकाः"))
        .map(|line| {
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
        })
        .collect();

    assert!(
        !lists.is_empty(),
        "spec/fdt-header.sas has no अष्टकाः directive"
    );
    lists
}

fn source() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/fdt-header.sas"
    ))
    .expect("spec/fdt-header.sas")
}

/// A key as the source must spell it: the name, then the terminator the match
/// loops stop on — which is part of the key and not a decoration.
fn terminated(name: &str) -> Vec<u8> {
    let mut want = name.as_bytes().to_vec();
    want.push(0);
    want
}

#[test]
fn the_search_keys_are_the_octets_of_memory_and_reg() {
    assert_eq!(
        octet_lists(&source()),
        vec![terminated(KEY), terminated(PROP)],
        "the keys in spec/fdt-header.sas do not spell {KEY} and {PROP}"
    );
}

#[test]
fn the_key_is_a_prefix_of_the_name_it_must_match() {
    // `/memory` is written `memory@80000000` in a flattened tree: the unit
    // address is part of the node name. The walk therefore matches the key as
    // a prefix and then requires `@` or the terminator, which is the property
    // this asserts is needed at all — a whole-string compare would find
    // nothing, and a bare prefix compare would also accept `memory-controller`.
    let real = "memory@80000000";
    assert!(real.starts_with(KEY));
    assert_ne!(real, KEY);
    assert_eq!(real.as_bytes()[KEY.len()], b'@');
    assert!("memory-controller".starts_with(KEY));
}

#[test]
fn the_property_name_is_matched_whole_and_the_node_name_is_not() {
    // Two names, two rules, and the difference is in the format rather than in
    // us. A node name carries a unit address after `@`, so `memory` can only be
    // a prefix of it; a property name carries nothing, so `reg` is the whole
    // string — and `reg-names` is an ordinary device-tree property that a prefix
    // compare would accept, yielding a list of strings read as an address.
    assert!(!"memory@80000000".starts_with(PROP));
    assert!("reg-names".starts_with(PROP));
    assert_ne!("reg-names", PROP);

    // A whole-string compare is what the source does: both sides are read until
    // a zero and every octet must agree, so the shorter key ends first and the
    // mismatch is against `-`, not against a terminator.
    assert_eq!("reg-names".as_bytes()[PROP.len()], b'-');
}
