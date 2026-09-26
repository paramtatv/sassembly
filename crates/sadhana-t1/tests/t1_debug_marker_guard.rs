//! A four-octet write in a product `.t1` source must sit under the dump switch.
//!
//! # The failure this exists for
//!
//! `40b9c34a` added fourteen diagnostic markers to `encode.t1` — `९११०`
//! through `९१२३`, one before each top-level statement of `स्थानसङ्केतनम्` —
//! and placed every one of them OUTSIDE the `यदि विन्यासमुद्रणार्हम् समम्
//! सत्यम्` block that every other marker in the file sits inside. Each calls
//! `चतुरष्टकमुद्रणम्`, which reaches `अष्टकॱमुद्रणम्`, which writes four
//! octets into the SAME channel the assembled object goes out on. Nothing in
//! the tree refused them. A day later `cargo test -p sadhana-t1` was red on two
//! targets, and neither red named the cause: the channel guard reported a
//! "SHORT WRITE" for a sink that was twenty-nine times too long, and the
//! module test reported `अष्टकॱमुद्रणम्` "is not a name in scope" — a correct
//! refusal by a loader that does not carry `अष्टक`, read as a defect in
//! `encode.t1`. `W-279` reverted the markers and repaired both instruments.
//!
//! # What this gate asserts, and why it is not a count
//!
//! The 2026-09-13 ruling (item 1) forbids pinning a number that encodes nothing
//! about correctness, and a marker CENSUS would not have caught `40b9c34a`
//! anyway: that commit made the count larger, which is what a commit adding
//! instrumentation is supposed to do. The property that separates a safe marker
//! from a landmine is not how many there are — it is whether the marker can
//! fire in a run nobody asked to instrument.
//!
//! So this REFUSES a shape. Every call to `चतुरष्टकमुद्रणम्` in
//! `crates/sadhana-t1/src/*.t1` must be lexically inside a block opened by
//!
//! ```text
//! यदि विन्यासमुद्रणार्हम् समम् सत्यम् आदि
//! ```
//!
//! `विन्यासमुद्रणार्हम्` is `असत्यम्` at its declaration and is set only by
//! `विन्यासमुद्रणारम्भः`, so a guarded marker costs a comparison and writes
//! nothing until a caller asks for the dump. Add a fifteenth marker anywhere
//! else and this test names the file, the line and the routine.
//!
//! **THE EXEMPTION IS NOT A LOOPHOLE.** Two routines call
//! `चतुरष्टकमुद्रणम्` as a serialisation primitive rather than as a marker —
//! `पाठमुद्रणम्` and `विन्यासमुद्रणम्`, which frame a length then that many
//! items — and their bodies cannot be under the switch, because they ARE the
//! dump. Exempting them would otherwise hand anyone a place to write an
//! unguarded four-octet emit: call the writer instead of the primitive. So the
//! second test requires the other half — every call to an exempt writer is
//! itself under the switch — and the two together say that no four octets reach
//! the channel from this crate's sources unless the switch is on.
//!
//! # Measured
//!
//! On the tree this landed with: 50 call sites across `encode.t1` and
//! `vishlesana.t1`, every one guarded, 0 violations. On
//! `40b9c34a:crates/sadhana-t1/src/encode.t1`, the same scan reports exactly
//! the fourteen — `९११०`…`९१२३` at lines 4440, 4469, 4495, 4502, 4532, 4578,
//! 4580, 4591, 4593, 4614, 4624, 4629, 4631, 4973, all inside
//! `स्थानसङ्केतनम्` (2026-09-19). The git-history half of that control is
//! recorded here rather than run from the test, which has no business shelling
//! out to `git`; the shape itself is reproduced as a control below.

use std::path::{Path, PathBuf};

/// `॰`, ADR-0017's comment mark: the rest of the line, outside a literal.
const COMMENT_MARK: &str = "॰";
/// `उक्तम्`, ADR-0003: opens a string literal.
const STRING_OPEN: &str = "उक्तम्";
/// `इति`: closes a literal, and closes a block.
const STRING_CLOSE: &str = "इति";
/// `आदि`: opens a block.
const BLOCK_OPEN: &str = "आदि";
/// The suffixes `split_trailing_punct` peels, in `crates/sadhana/src/lex.rs`'s
/// order — `ॱॱ` before the single `ॱ` could ever be considered.
const PUNCT: [&str; 4] = ["ॱॱ", "।", "॥", "ऽ"];
/// `ॱ` joins a module to a name in a qualified reference, with no spaces.
const MODULE_SEP: char = 'ॱ';

/// The four-octet emit. Reaches `अष्टकॱमुद्रणम्` and the product channel.
const MARKER: &str = "चतुरष्टकमुद्रणम्";
/// The dump switch — `असत्यम्` at its declaration in `encode.t1`.
const FLAG: &str = "विन्यासमुद्रणार्हम्";
/// The two routines that call [`MARKER`] as a serialiser, not as a marker.
const WRITERS: [&str; 2] = ["पाठमुद्रणम्", "विन्यासमुद्रणम्"];

const ROUTINE: &str = "वृत्तिः";
const PUBLIC: &str = "सार्वजनिक";
const IF: &str = "यदि";
const EQUALS: &str = "समम्";
const TRUE: &str = "सत्यम्";

/// The crate's own `src`, from `CARGO_MANIFEST_DIR` and not the working
/// directory: `cargo test -p sadhana-t1` and the workspace run set that
/// differently, and a gate that resolves in one and not the other reports the
/// harness rather than the sources.
fn crate_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.t1` source of this crate, sorted.
fn t1_sources() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(crate_src())
        .expect("crates/sadhana-t1/src exists")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    v
}

/// Peel one trailing punctuation mark, as `crates/sadhana/src/lex.rs` does.
fn split_trailing_punct(word: &str) -> (&str, Option<&str>) {
    for p in PUNCT {
        if word.len() > p.len()
            && let Some(head) = word.strip_suffix(p)
        {
            return (head, Some(p));
        }
    }
    (word, None)
}

/// The bare name of a possibly module-qualified reference.
///
/// `सङ्केतनॱचतुरष्टकमुद्रणम्` and `चतुरष्टकमुद्रणम्` are the same routine seen
/// from two modules, and `vishlesana.t1` reaches it by the first spelling. A
/// gate that matched the bare name only would pass that whole file in silence.
fn bare(name: &str) -> &str {
    name.rsplit(MODULE_SEP).next().unwrap_or(name)
}

/// The code tokens of one line: no literal contents, no comment.
///
/// **THE ORDER OF THE TWO CUTS IS THE WHOLE OF ADR-0017**, and it is copied
/// from `lex.rs::pieces` rather than approximated. Cutting the comment first
/// deletes a `॰` written inside a string; scanning the string first does not.
/// Inside `उक्तम् … इति`, a PAIR of bare `इति` is the literal word and the
/// string continues, and the first unpaired one closes — which is why a naive
/// `इति`-counter puts `ir.t1`, `parse.t1`, `unparse.t1` and `vakyavibhaga.t1`
/// four, twelve, four and two blocks in deficit and silently mis-scopes every
/// line after the first literal that says `आदि` or `इति` out loud. With this
/// tokeniser all twenty-one sources balance to zero and never go negative.
///
/// # Errors
/// A `उक्तम्` that no `इति` closes on the same line — which `lex.rs` also
/// refuses, and which means the scanner must not pretend to have scoped this
/// file.
fn code_tokens(line: &str) -> Result<Vec<&str>, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0usize;
    while i < words.len() {
        let w = words[i];
        if w == STRING_OPEN {
            let mut j = i + 1;
            let close = loop {
                let Some(&wj) = words.get(j) else {
                    return Err(format!("a `{STRING_OPEN}` that no `{STRING_CLOSE}` closes"));
                };
                let (head, tail) = split_trailing_punct(wj);
                if head == STRING_CLOSE {
                    // A pair is the word itself. Both members must be bare:
                    // only the one that CLOSES can carry a daṇḍa.
                    if tail.is_none() && words.get(j + 1) == Some(&STRING_CLOSE) {
                        j += 2;
                        continue;
                    }
                    if let Some(p) = tail {
                        out.push(p);
                    }
                    break j;
                }
                j += 1;
            };
            i = close + 1;
            continue;
        }
        if let Some(mark) = w.find(COMMENT_MARK) {
            if mark > 0 {
                out.push(split_trailing_punct(&w[..mark]).0);
            }
            return Ok(out);
        }
        out.push(split_trailing_punct(w).0);
        i += 1;
    }
    Ok(out)
}

/// The name a routine-head line declares, and the index it sits at.
fn routine_head<'a>(toks: &[&'a str]) -> Option<(&'a str, usize)> {
    if toks.first() == Some(&ROUTINE) {
        return toks.get(1).map(|n| (*n, 1));
    }
    if toks.first() == Some(&PUBLIC) && toks.get(1) == Some(&ROUTINE) {
        return toks.get(2).map(|n| (*n, 2));
    }
    None
}

/// Whether this line is `यदि <flag> समम् सत्यम् आदि` and nothing else.
///
/// Deliberately the exact five tokens. A looser match — "the line mentions the
/// flag somewhere" — would pass a marker sitting under `यदि विन्यासमुद्रणार्हम्
/// समम् असत्यम्`, which is the switch read backwards and fires in exactly the
/// runs the switch exists to keep quiet.
fn is_guard_line(toks: &[&str]) -> bool {
    toks.len() == 5
        && toks[0] == IF
        && bare(toks[1]) == FLAG
        && toks[2] == EQUALS
        && toks[3] == TRUE
        && toks[4] == BLOCK_OPEN
}

/// One call site, with the verdict the scan reached about it.
struct Site {
    line: usize,
    routine: String,
    text: String,
    guarded: bool,
}

/// Every call site of `names` in `src`, each marked guarded or not.
///
/// **THREE OUTCOMES, NOT TWO.** A file with no call sites answers an empty
/// list; a file the block scanner could scope answers its sites; a file it
/// could NOT scope answers `Err` and says why. The third state is the one an
/// instrument usually hides: a scanner whose block stack has drifted reports
/// "no unguarded markers" for exactly the same reason it would report it
/// truthfully, and that is how `40b9c34a` survived a day of green.
fn scan(src: &str, names: &[&str], exempt: &[&str]) -> Result<Vec<Site>, String> {
    let mut stack: Vec<bool> = Vec::new();
    let mut routine = String::new();
    let mut sites: Vec<Site> = Vec::new();

    for (n0, line) in src.lines().enumerate() {
        let n = n0 + 1;
        let toks = code_tokens(line).map_err(|e| format!("line {n}: {e}"))?;
        if toks.is_empty() {
            continue;
        }
        let head = routine_head(&toks);

        for (k, t) in toks.iter().enumerate() {
            if !names.contains(&bare(t)) {
                continue;
            }
            // The routine's own declaration is not a call to it.
            if head.is_some_and(|(_, at)| at == k) {
                continue;
            }
            sites.push(Site {
                line: n,
                routine: routine.clone(),
                text: line.trim().to_string(),
                guarded: stack.iter().any(|g| *g) || exempt.contains(&routine.as_str()),
            });
        }

        let guard = is_guard_line(&toks);
        for t in &toks {
            if *t == BLOCK_OPEN {
                stack.push(guard);
                if let Some((name, _)) = head {
                    routine = name.to_string();
                }
            } else if *t == STRING_CLOSE {
                if stack.pop().is_none() {
                    return Err(format!(
                        "line {n}: a block close with no open — the scanner cannot scope this file"
                    ));
                }
                if stack.is_empty() {
                    routine.clear();
                }
            }
        }
    }

    if stack.is_empty() {
        Ok(sites)
    } else {
        Err(format!(
            "{} block(s) never closed — the scanner cannot scope this file",
            stack.len()
        ))
    }
}

/// Run one scan over the real sources and report every unguarded site.
fn sweep(names: &[&str], exempt: &[&str], what: &str) {
    let mut checked = 0usize;
    let mut total = 0usize;
    let mut bad: Vec<String> = Vec::new();

    for path in t1_sources() {
        let src = std::fs::read_to_string(&path).expect("a .t1 source reads");
        let name = path.file_name().expect("a file name").to_string_lossy();
        if !names.iter().any(|n| src.contains(n)) {
            continue;
        }
        match scan(&src, names, exempt) {
            Err(why) => bad.push(format!("{name}: {why}")),
            Ok(sites) => {
                checked += 1;
                total += sites.len();
                for s in sites.iter().filter(|s| !s.guarded) {
                    let r = if s.routine.is_empty() {
                        "the top level".to_string()
                    } else {
                        s.routine.clone()
                    };
                    bad.push(format!("{name}:{} in {r}\n      {}", s.line, s.text));
                }
            }
        }
    }

    assert!(
        checked > 0,
        "no source mentions {what} at all — this gate is checking nothing, which \
         is how a gate that was silently defeated looks from the outside"
    );
    assert!(
        bad.is_empty(),
        "{what}: {} of {total} call sites across {checked} file(s) are NOT under \
         `{IF} {FLAG} {EQUALS} {TRUE} {BLOCK_OPEN}`.\n\n{}\n\n\
         Each one writes four octets into the product's own output channel on \
         every run, instrumented or not. That is what `40b9c34a` did, and the \
         two tests it reddened a day later both named something else.",
        bad.len(),
        bad.join("\n  - "),
    );

    println!("METRIC {what}: {total} call sites, {checked} file(s), all guarded");
}

/// Every four-octet emit in the product sources sits under the dump switch.
#[test]
fn every_four_octet_emit_is_under_the_dump_switch() {
    sweep(&[MARKER], &WRITERS, "four-octet emits");
}

/// And the two exempt writers are themselves only reached under the switch.
///
/// Without this, the exemption above is a door: `विन्यासमुद्रणम् स्थानानि`
/// written at the top of a routine emits a length and that many words with no
/// guard anywhere, and the first test would pass it because the emit it can
/// see is inside an exempt body.
#[test]
fn the_dump_writers_are_only_called_under_the_switch() {
    sweep(&WRITERS, &[], "dump-writer calls");
}

// ── controls ───────────────────────────────────────────────────────────────
//
// Each drives `scan` over a source written here, so what the gate answers is
// read off the text rather than off a tree that could change underneath it.

/// The shape `40b9c34a` wrote: a marker one line past the guard's `इति`.
#[test]
fn the_shape_that_reddened_the_crate_is_refused() {
    let src = "\
सार्वजनिक वृत्तिः स्थानसङ्केतनम् आदि
    यदि विन्यासमुद्रणार्हम् समम् सत्यम् आदि
        चरः अवगणन९१०५ ॱॱ न६४ भवति चतुरष्टकमुद्रणम् ९१०५ ।
    इति
    चरः अवगणन९११० ॱॱ न६४ भवति चतुरष्टकमुद्रणम् ९११० ।
इति
";
    let sites = scan(src, &[MARKER], &WRITERS).expect("the control scopes");
    let open: Vec<usize> = sites
        .iter()
        .filter(|s| !s.guarded)
        .map(|s| s.line)
        .collect();
    assert_eq!(
        open,
        vec![5],
        "the marker past the guard's `{STRING_CLOSE}` must be the one refused; \
         sites were {:?}",
        sites
            .iter()
            .map(|s| (s.line, s.guarded))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        sites[0].routine, "स्थानसङ्केतनम्",
        "a refusal that cannot name the routine sends the reader to the wrong file"
    );
}

/// THE CASE THAT MUST STILL BE REFUSED-TO-FIRE: a guarded marker is silent.
///
/// A gate that reds on every marker is a gate that gets deleted the first time
/// someone needs to instrument a run, and then nothing refuses the fifteenth.
#[test]
fn a_guarded_marker_is_not_a_finding() {
    let src = "\
सार्वजनिक वृत्तिः क आदि
    यदि विन्यासमुद्रणार्हम् समम् सत्यम् आदि
        यावत् क्रमः न्यूनम् ४ आदि
            चरः अवगणना ॱॱ न६४ भवति चतुरष्टकमुद्रणम् ९००१ ।
        इति
    इति
इति
";
    let sites = scan(src, &[MARKER], &WRITERS).expect("the control scopes");
    assert_eq!(sites.len(), 1, "one call site");
    assert!(
        sites[0].guarded,
        "a marker nested two blocks deep inside the guard is still guarded"
    );
}

/// The switch read backwards is not a guard.
#[test]
fn the_negated_switch_does_not_guard() {
    let src = "\
सार्वजनिक वृत्तिः क आदि
    यदि विन्यासमुद्रणार्हम् समम् असत्यम् आदि
        चरः अवगणना ॱॱ न६४ भवति चतुरष्टकमुद्रणम् ९००१ ।
    इति
इति
";
    let sites = scan(src, &[MARKER], &WRITERS).expect("the control scopes");
    assert_eq!(sites.len(), 1);
    assert!(
        !sites[0].guarded,
        "`{EQUALS} असत्यम्` fires in exactly the runs the switch keeps quiet, so it \
         cannot be what makes a marker safe"
    );
}

/// A qualified call from another module is the same call.
#[test]
fn a_module_qualified_emit_is_seen() {
    let src = "\
सार्वजनिक वृत्तिः क आदि
    चरः अवगणना ॱॱ न६४ भवति सङ्केतनॱचतुरष्टकमुद्रणम् ९४०१ ।
इति
";
    let sites = scan(src, &[MARKER], &WRITERS).expect("the control scopes");
    assert_eq!(sites.len(), 1, "`सङ्केतनॱ{MARKER}` is a call to {MARKER}");
    assert!(!sites[0].guarded);
}

/// The name written in a margin or inside a literal is not a call.
///
/// `shrinkhala.t1:2467` names `चतुरष्टकमुद्रणम्` in prose and `encode.t1:2087`
/// names the flag in prose; a grep-shaped gate reds on both. `unparse.t1` goes
/// further and writes `उक्तम् इति इति इति` — the literal word `इति` — which a
/// naive block counter takes as two stray closes.
#[test]
fn prose_and_literals_are_not_call_sites() {
    let src = "\
॰ through `चतुरष्टकमुद्रणम्` when `विन्यासमुद्रणार्हम्` is set — tagged records,
सार्वजनिक वृत्तिः क आदि
    चरः ख ॱॱ न६४ भवति ७ ।   ॰ चतुरष्टकमुद्रणम् ९९९९ in a trailing margin
    चरः ग ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् चतुरष्टकमुद्रणम् आदि इति ।
    चरः घ ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् इति इति इति ।
इति
";
    let sites = scan(src, &[MARKER], &WRITERS).expect(
        "the literal `उक्तम् आदि इति` must not leave the scanner one block deep, \
         and `उक्तम् इति इति इति` must not leave it two closes in deficit",
    );
    assert!(
        sites.is_empty(),
        "no call site here; the scan found {:?}",
        sites.iter().map(|s| s.line).collect::<Vec<_>>()
    );
}

/// The refusal a scan owes, or a panic that says what it scoped instead.
///
/// **NOT `expect_err`.** That wants `Debug` on the `Ok` side, and `Debug` on a
/// `String` escapes every combining mark — the `सङ\u{94d}केतन` fault this
/// ledger logged on 2026-09-19, where a control PASSED, the message fired, the
/// exit code was right, and the one line the reader had to read was unreadable.
fn must_refuse(r: Result<Vec<Site>, String>, what: &str) -> String {
    match r {
        Err(why) => why,
        Ok(sites) => panic!(
            "{what} must be unscopable; the scan instead returned {} site(s), \
             the first at line {}",
            sites.len(),
            sites.first().map_or(0, |s| s.line)
        ),
    }
}

/// An unscopable file says so, instead of reporting an absence of findings.
#[test]
fn a_file_the_scanner_cannot_scope_is_refused_not_passed() {
    let unclosed = "सार्वजनिक वृत्तिः क आदि\n    चरः ख ॱॱ न६४ भवति चतुरष्टकमुद्रणम् १ ।\n";
    let why = must_refuse(scan(unclosed, &[MARKER], &WRITERS), "an unclosed routine");
    assert!(why.contains("never closed"), "said: {why}");

    let extra = "इति\n";
    let why = must_refuse(scan(extra, &[MARKER], &WRITERS), "a stray close");
    assert!(why.contains("no open"), "said: {why}");

    let dangling = "चरः क ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् ख ।\n";
    let why = must_refuse(scan(dangling, &[MARKER], &WRITERS), "an unclosed literal");
    assert!(why.contains(STRING_OPEN), "said: {why}");
}

/// The writer exemption does not travel past the writer's own body.
#[test]
fn a_writer_called_outside_the_switch_is_a_finding() {
    let src = "\
सार्वजनिक वृत्तिः विन्यासमुद्रणम् आदाय स्थानानि ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति चतुरष्टकमुद्रणम् दैर्घ्य ।
इति

सार्वजनिक वृत्तिः क आदि
    चरः अवगणना ॱॱ न६४ भवति विन्यासमुद्रणम् स्थानानि ।
इति
";
    let emits = scan(src, &[MARKER], &WRITERS).expect("the control scopes");
    assert_eq!(emits.len(), 1);
    assert!(
        emits[0].guarded,
        "the emit inside the writer's own body is the exemption, and it holds"
    );

    let calls = scan(src, &WRITERS, &[]).expect("the control scopes");
    let open: Vec<usize> = calls
        .iter()
        .filter(|s| !s.guarded)
        .map(|s| s.line)
        .collect();
    assert_eq!(
        open,
        vec![6],
        "calling the writer from an unguarded routine emits four octets per item \
         with no switch anywhere, and the emit-level scan cannot see it"
    );
}
