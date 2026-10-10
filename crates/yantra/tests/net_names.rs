//! ADR-0048: the owner's network names (ruling 2026-10-08, "names: primaries; ADR-0047
//! accepted") agree everywhere they are written — the lexicon, the Rust refusal messages, the
//! ADR, and the `.t1` library under `spec/net/`. These are cheap and run in every gate; the
//! tests that BUILD and RUN the library are `net_t1.rs`.

use std::path::{Path, PathBuf};
use yantra::netdev::{
    NET_BACKEND_UNAVAILABLE, NET_FRAME_RUNT, NET_FRAME_TOO_LARGE, NET_LOG_TAG_MISMATCH,
    NET_PEER_CLOSED, NET_RX_COUNT_EXCEEDED, NET_SRC_MAC_REFUSED, NET_TIMEOUT,
    NET_TX_COUNT_EXCEEDED, NET_TX_NO_HEADER, NET_WINDOW_REFUSED, RESERVED_REFUSALS, RULED_REFUSALS,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `(word, english, category, source, notes)` for every lexicon row.
fn lexicon() -> Vec<Vec<String>> {
    read("spec/lexicon.src.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t") && !l.is_empty())
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

fn net_sources() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = std::fs::read_dir(root().join("spec/net"))
        .expect("spec/net")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    v.sort();
    assert!(v.len() >= 10, "the library and its demo: {}", v.len());
    v
}

/// Every Devanagari-digit numeral in a source's CODE (comments cut).
fn numerals(src: &str) -> Vec<u64> {
    let mut out = Vec::new();
    for line in src.lines() {
        let code = line.split('\u{970}').next().unwrap_or("");
        let mut cur = String::new();
        let flush = |cur: &mut String, out: &mut Vec<u64>| {
            if !cur.is_empty() {
                if let Ok(n) = cur.parse::<u64>() {
                    out.push(n);
                }
                cur.clear();
            }
        };
        let mut prev_word = false;
        for c in code.chars() {
            if ('\u{966}'..='\u{96f}').contains(&c) && !prev_word {
                cur.push(char::from(b'0' + (c as u32 - 0x966) as u8));
            } else {
                flush(&mut cur, &mut out);
                prev_word = c.is_alphabetic() || ('\u{900}'..='\u{903}').contains(&c);
            }
            if c.is_whitespace() {
                prev_word = false;
            }
        }
        flush(&mut cur, &mut out);
    }
    out
}

#[test]
fn the_nine_refusals_are_in_the_lexicon_the_adr_and_the_device_by_code() {
    let rows = lexicon();
    let adr =
        read("docs/adr/0048-the-network-names-modules-members-and-refusals-0x35f-to-0x367.md");
    assert_eq!(RULED_REFUSALS.len(), 16);
    assert_eq!(RESERVED_REFUSALS.len(), 3);
    for (code, name) in RULED_REFUSALS.into_iter().chain(RESERVED_REFUSALS) {
        assert!((0x35f..=0x371).contains(&code));
        let row = rows
            .iter()
            .find(|r| r[0] == name)
            .unwrap_or_else(|| panic!("{name} is not a lexicon row"));
        assert_eq!(row[2], "error", "{name}");
        assert_eq!(row[3], "owner-2026-10-08", "{name}");
        assert!(
            row[1].contains(&format!("refusal {code:#x}")),
            "{name}: {}",
            row[1]
        );
        let word = (code << 16) | 0x3333;
        assert!(row[4].contains(&format!("{word:#x}")), "{name}: {}", row[4]);
        assert!(adr.contains(name), "ADR-0048 names {name}");
        assert!(
            adr.contains(&format!("{word:#x}")),
            "ADR-0048 lists {word:#x}"
        );
    }
    // the device's own messages lead with the ruled name and code
    for (msg, code) in [
        (NET_WINDOW_REFUSED, 0x35f),
        (NET_FRAME_TOO_LARGE, 0x360),
        (NET_TX_COUNT_EXCEEDED, 0x361),
        (NET_RX_COUNT_EXCEEDED, 0x361),
        (NET_FRAME_RUNT, 0x363),
        (NET_TX_NO_HEADER, 0x363),
        (NET_SRC_MAC_REFUSED, 0x368),
        (NET_TIMEOUT, 0x369),
        (NET_PEER_CLOSED, 0x36a),
        (NET_BACKEND_UNAVAILABLE, 0x36b),
        (NET_LOG_TAG_MISMATCH, 0x36c),
    ] {
        let name = RULED_REFUSALS.iter().find(|(c, _)| *c == code).unwrap().1;
        assert!(
            msg.starts_with(&format!("{name} ({code:#x})")),
            "{msg:?} should lead with {name} ({code:#x})"
        );
    }
}

#[test]
fn every_refusal_word_the_library_writes_is_a_ruled_word_and_each_ruled_word_is_written() {
    let ruled: Vec<u64> = RULED_REFUSALS
        .iter()
        .chain(RESERVED_REFUSALS.iter())
        .map(|(c, _)| (c << 16) | 0x3333)
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    for (file, src) in net_sources() {
        for n in numerals(&src) {
            // anything in the neighbourhood of the nine words must BE one of them
            if (0x35e_0000..0x372_0000).contains(&n) {
                assert!(
                    ruled.contains(&n),
                    "{file}: {n:#x} is near the refusal words and is none"
                );
                seen.insert(n);
            }
        }
    }
    for code in (0x35f..=0x367_u64).chain([0x369, 0x36d, 0x371]) {
        let w = &((code << 16) | 0x3333);
        assert!(
            seen.contains(w),
            "no library source writes the refusal word {w:#x}"
        );
    }
}

#[test]
fn the_modules_and_members_are_the_rulings_and_are_lexicon_words() {
    let rows = lexicon();
    let words: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    let sources = net_sources();
    let all: String = sources
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    // the layer modules and the family module (UDP and DNS are milestone 3)
    for m in [
        "सञ्चारपङ्क्तिः",
        "सम्बन्धः",
        "स्थाननिर्देशः",
        "मार्गः",
        "मार्गसन्देशः",
        "निःसन्धिपिण्डः",
        "नामनिर्देशः",
        "सञ्चार",
    ] {
        assert!(words.contains(&m), "{m} is a lexicon word");
        assert!(
            all.contains(&format!("मण्डलम् {m} \u{965}")),
            "{m} is declared as a module in spec/net"
        );
    }
    // the members of the family module, called सञ्चारॱ<member>
    let family = sources
        .iter()
        .find(|(f, _)| f == "sanchara.t1")
        .map(|(_, s)| s.as_str())
        .expect("sanchara.t1");
    for m in [
        "सम्बन्धपिण्डप्रेषणम्",
        "सम्बन्धपिण्डग्रहणम्",
        "सम्बन्धनिर्देशः",
        "सम्बन्धावस्था",
        "मार्गप्रतिध्वनिः",
        "सन्देशप्रेषणम्",
        "सन्देशग्रहणम्",
        "नामनिर्देशनिर्णयः",
    ] {
        let row = rows
            .iter()
            .find(|r| r[0] == m)
            .unwrap_or_else(|| panic!("{m} not a lexicon row"));
        assert!(
            row[4].contains(&format!("सञ्चार\u{971}{m}")),
            "{m}: {}",
            row[4]
        );
        assert!(
            family.contains(&format!("सार्वजनिक वृत्तिः {m} ")),
            "the family module declares the public member {m}"
        );
    }
}

#[test]
fn the_library_is_not_in_the_compilers_corpus() {
    let dir = root().join("crates/sadhana-t1/src");
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "t1") {
            let s = std::fs::read_to_string(&p).unwrap();
            for m in ["सञ्चारपङ्क्तिः", "स्थाननिर्देशः", "मार्गसन्देशः", "निःसन्धिपिण्डः"]
            {
                assert!(!s.contains(m), "{} mentions {m}", p.display());
            }
        }
    }
}
