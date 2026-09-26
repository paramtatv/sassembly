//! **समावेशः** — the compile-time embed, ADR-0019 (owner-ratified 2026-08-28),
//! task `D-002g`.
//!
//! The module is named `anita` after `आनीतम्`, the word the ADR's DRAFT proposed
//! and the owner refused. The name is left alone deliberately: it is a Rust
//! module path, not a word of Sassembly, and renaming it would cost every
//! `use` site in exchange for nothing a reader of this doc comment lacks.
//!
//! `D-002g` asks one question and refuses to let it be skipped: T1 has no I/O,
//! and `include_str!` is a compile-time facility, so is T1's answer a **runtime
//! reader** or a **compile-time embed**? ADR-0019 answers *embed*, and this
//! module is that answer.
//!
//! Three facts force it. The four blocked readers in
//! `crates/sadhana-t1/src/encode.t1` are called *while the assembler is
//! assembling*, so a runtime read would stop `sadhana` being a cross-assembler.
//! The module being ported already does exactly this thirteen times —
//! `crates/sadhana/src/parse.rs:25` is `include_str!` on the very file this
//! row's acceptance names. And `encode.t1:419` says so in its own margin:
//! *"Rust reaches its four tables through `include_str!` … T1 has no include
//! and no file reader."*
//!
//! # Why this is a pass over tokens, and not a stage of the lexer or parser
//!
//! `include_str!` is a macro: it runs between lexing and parsing. So does this.
//!
//! **Not the lexer.** [`crate::lex::lex_t1`] is a pure function of its argument
//! and is worth keeping one — a lexer that opened files would put the
//! filesystem inside every lexer test and inside `D-003`'s determinism claim.
//!
//! **Not the parser.** `spec/grammar-t1.ebnf` freezes the lexical layer and the
//! three phrase-structure constructs, and says plainly that `expression`,
//! `statement` and `type` are deferred non-terminals that *"are not defined
//! anywhere yet"*, because operators wait on `A-063`. A `Vec<Token> →
//! `Vec<Token>` pass rests only on the layer that **is** frozen.
//!
//! # A file is NAMED, never pathed
//!
//! `spec/mnemonics-riscv64.src.tsv` is Latin letters, `/`, `-` and `.`.
//! `crates/sadhana/src/lex.rs` checks the doc 15 repertoire over every word and
//! over the whole of a string literal — *"a string is not a hole in R-15-1"* —
//! so **a path is not writable in this language at all.**
//!
//! `D-002g` says do not weaken the repertoire, and there were two ways to: admit
//! Latin so a path could be typed, or exempt this construct from the check.
//! Neither is done. The source holds a keyword and an ordinary identifier, both
//! checked exactly as every other word is, and the compiler resolves the name
//! against [`TABLES`]. That is `R-02-1`'s answer — *"every architectural
//! instruction has a Sassembly name … there is no `.insn 0x…` escape hatch"* —
//! applied to tables, and it buys a property validation cannot: **`..` is not
//! refused, it is unspellable**, because there is no path to put it in.
//!
//! The five names are derived rather than coined. Each is `<concept>कोशः`, and
//! each concept word is taken from the blocked reader that wants the file:
//! `सङ्केताः` → `सङ्केतकोशः`, `कोष्ठाङ्कः` → `कोष्ठकोशः`, `क्षेत्रसमूहः` →
//! `क्षेत्रकोशः`, `सङ्कोचः` → `सङ्कोचकोशः`. `नामकोशः` is the mnemonic *name*
//! registry and is this row's acceptance target.
//!
//! # The content is a [`Kind::Str`], and that is the third time
//!
//! ADR-0017 made a string one token because three parsers had each written
//! their own string reader and the three disagreed. ADR-0018 gave its layout
//! words `Kind::Str` for the same stated reason. An embed's content is a
//! `Kind::Str` for the third time, so `super::drishya`'s `string`,
//! `super::parse`'s `StringLiteral` and `super::comptime`'s `Value::String`
//! read an embedded table today with **no edit to any of them**.
//!
//! The content is the file's bytes, exactly. `encode.t1`'s four row parsers all
//! take `अङ्कः अन्तः अ८`, and a `Kind::Str`'s value viewed as UTF-8 is those
//! octets; because TAB and LINE FEED are ASCII, `encode.t1`'s
//! `अष्टकान्वेषणम्` finds a field separator in a UTF-8 Devanagari table with no
//! decoder.
//!
//! # TAB still has no word and does not need one
//!
//! ADR-0018 named two of R-15-1's three layout characters and recorded *"TAB has
//! no word because nothing in the tree wants one"*. A tab-separated file looked
//! like the program that wanted one. It is not: `encode.t1:427` already writes
//! its separators as numerals — *"TAB 9, LF 10, CR 13, SPACE 32"* — because a
//! program that SPLITS octets compares against an octet, where a layout word is
//! for a string a program WRITES. ADR-0018 is not extended here.
//!
//! # What this module does not do
//!
//! Content is **not rescanned**: a `समावेशः` inside an embedded file is data,
//! not a nested embed. That is stated so it is a decision rather than an
//! oversight.
//!
//! # The word does two jobs, and the tier is what separates them
//!
//! T0 is untouched: [`crate::lex::lex`] never produces an embed and `समावेशः`
//! remains `.include`, *"textual inclusion"*, `planned`, in
//! `spec/directives.tsv:36`. So one word now names a T0 DIRECTIVE that splices
//! SOURCE and a T1 EXPRESSION that yields a file's BYTES as a value — `include!`
//! and `include_str!`, in Rust's terms.
//!
//! **The tier decides, and nothing else has to.** A directive is read by
//! `spec/grammar-t0.ebnf` and `lex`; an embed is read by `spec/grammar-t1.ebnf`
//! and [`crate::lex::lex_t1`], which is the only stream [`resolve`] ever sees.
//! No source is both tiers at once, so no occurrence of the word is ambiguous.
//! Two further discriminators back it up — a directive is wrapped in `॥ … ॥`
//! (ADR-0012) where an embed is wrapped in `आरभ्य … समाप्तम्` (ADR-0003), and a
//! directive's argument may be a string (ADR-0013) where an embed's is a bare
//! identifier that cannot be a path.
//!
//! This is not the first word in the tree to do it: five of the six directives
//! at `spec/directives.tsv:32`-`:39` already carry a T1 sense too, and
//! `समावेशः` was the only one that did not. See ADR-0019 and
//! `a_word_that_serves_both_tiers_is_the_rule_and_not_the_exception`.

use crate::lex::{Kind, Token};
use std::path::Path;

/// `समावेशः` — opens an embed (ADR-0019, owner-ratified 2026-08-28).
///
/// It is the word the frozen `keyword` production has carried since v0.1,
/// glossed `include`. **No keyword was added for this construct**, which is the
/// owner's ruling: the draft proposed `आनीतम्` because `समावेशः` was occupied by
/// `crates/sadhana-t1/src/encode.t1`'s `Slot::fits`, and the answer was that the
/// occupancy was a DEFECT — a frozen keyword bound as a name — to be fixed
/// rather than routed around. `B-112` renamed it to `अन्तर्भावः` and added the
/// guard that stops the next one.
pub const EMBED_WORD: &str = "समावेशः";

/// `आरभ्य` — ADR-0003's ratified group open. Nothing is coined here.
const GROUP_OPEN: &str = "आरभ्य";
/// `समाप्तम्` — ADR-0003's ratified group close.
const GROUP_CLOSE: &str = "समाप्तम्";

/// The finite, reviewed set of files a T1 program may bring in: a name, and the
/// file it denotes **relative to the spec root the caller passes in**.
///
/// Every entry is here because a named reader in the tree is blocked on it, not
/// because the file exists. `नामकोशः` is `D-002g`'s acceptance target and
/// `D-002f2`'s `families()`/`directives()`; the other four are the files
/// `encode.t1`'s `सङ्केताः`, `कोष्ठाङ्कः`, `क्षेत्रसमूहः` and `रूपाणि` open in
/// their Rust originals.
///
/// ADR-0019 records that this table is data about the spec tree and belongs in
/// `spec/` beside `directives.tsv`. It is a `const` today only because `D-002g`
/// may not write other `spec/` files; moving it is a successor row.
const TABLES: &[(&str, &str)] = &[
    ("नामकोशः", "mnemonics-riscv64.src.tsv"),
    ("सङ्केतकोशः", "encodings-riscv64.tsv"),
    ("कोष्ठकोशः", "registers-riscv64.tsv"),
    ("क्षेत्रकोशः", "fence-domains-riscv64.tsv"),
    ("सङ्कोचकोशः", "compression-choices.tsv"),
    // ADDED 2026-08-29 for `D-002f7`. `parse.rs:26` reads
    // `include_str!("../../../spec/web-dictionary.tsv")` and its T1 port
    // `जालपाठः` could not be written because the file had no NAME here — the
    // THIRD routine in one day blocked by a missing table name rather than by
    // anything about the language, after `संस्कारकोशः` and `निर्देशकोशः`.
    // `जाल` is the corpus's own word for the web.
    ("जालकोशः", "web-dictionary.tsv"),
    // ADDED 2026-08-29 for `D-002f2`. `parse.rs:179` reads
    // `include_str!("../../../spec/directives.tsv")` and its T1 port
    // `निर्देशकोशपठनम्` could not resolve without a name here. `निर्देशः` is
    // the corpus's own word for a directive — it appears 43 times in
    // `vakyavibhaga.t1`, including in `निर्देशयोजनम्` and `निर्देशसूचकाङ्क`.
    //
    // THIS ENTRY WENT MISSING ONCE ALREADY. It was written, verified, and then
    // lost when a routine-level union took `vakyavibhaga.t1` wholesale from
    // worktrees that never carried it — and a `grep -c` looking for the name
    // found the COMMENT above and reported it present. Two readers were live
    // in the tree with no table to resolve against, and only
    // `every_t1_source_lexes_and_parses` said so.
    ("निर्देशकोशः", "directives.tsv"),
    // ADDED 2026-08-29 for `D-002i2`. `samyojana.rs:65` reads
    // `include_str!("../../../spec/relocations-riscv64.tsv")`, and its T1 port
    // `भेदाङ्कः` could not be written because the file had no NAME here — the
    // embed resolves an identifier against this table, so an unlisted file is
    // unreachable however much it exists on disk. `संस्कारः` is the linker's
    // own word for a relocation in this tree (`samyojana.t1:468`), so the name
    // follows the corpus rather than inventing one.
    ("संस्कारकोशः", "relocations-riscv64.tsv"),
    // ADDED 2026-08-30. THE FIFTH AND SIXTH TIME A ROUTINE WAS BLOCKED BY A
    // MISSING NAME HERE RATHER THAN BY ANYTHING ABOUT THE LANGUAGE, after
    // `जालकोशः`, `निर्देशकोशः` and `संस्कारकोशः` above — and this time TWO
    // separate agents, working in different files and unable to see each
    // other, each traced their own stub to this one table and each wrote down
    // the same one-line remedy. Neither could apply it: `anita.rs` was outside
    // the files they were allowed to touch.
    //
    // That is the finding, and it is about this table rather than about them:
    // an unlisted file is unreachable however much it exists on disk, the
    // failure surfaces in whatever module happened to need it, and so the
    // SAME defect is rediscovered from a new direction each time. The margins
    // above already record it happening three times; a rule that a `spec/*.tsv`
    // gets its name here when the file is created would end it, and that is a
    // successor row rather than this comment.
    //
    // `शिवसूत्रकोशः` unblocks `अक्षरकोश ॱ व्यञ्जन`: `HAL` is
    // `char_set("ह","ल्")` derived from this file, and the 33 consonants are
    // NOT a contiguous range (U+0915–U+0939 also holds ऩ ऱ ळ ऴ), so nothing
    // arithmetic answers it and listing them in the port would be a second
    // transcription of the authority.
    ("शिवसूत्रकोशः", "shiva-sutras.tsv"),
    // `निदानकोशः` unblocks `सङ्केतन ॱ सङ्केतनदोषवचनम्`, whose row walk was
    // already written and had nothing to walk. `निदान` is the corpus's own
    // word for a diagnostic — it names a whole module (`nidana.t1`).
    ("निदानकोशः", "diagnostics.tsv"),
    // ADDED 2026-08-30 for `अक्षरकोश ॱ अक्षराणि`. UNLIKE EVERY ENTRY ABOVE,
    // THIS FILE DID NOT EXIST — the others were on disk and merely unnamed.
    // The UAX #29 grapheme-break classes lived ONLY in
    // `crates/sanskrit-text/src/tables.rs`, generated into RUST by `ucdgen`,
    // and ADR-0019's embed resolves a name against a `spec/*.tsv`, so no T1
    // port could read them at all.
    //
    // `tools/gen-grapheme-break.py` emits it from the PINNED UCD at
    // `research/specs/unicode/ucd/auxiliary/GraphemeBreakProperty.txt`, whose
    // sha256 is in `research/specs/FETCH-LOG.tsv`. It does NOT fetch:
    // `tools/gen-segmenter.py` reaches unicode.org, and `tables.rs`'s own
    // header records that regenerating against a DIFFERENT UCD version changes
    // akṣara boundaries and is a deliberate act rather than a refresh.
    //
    // `the_grapheme_break_table_agrees_with_the_generated_rust_one` checks the
    // two readers of that one source against each other, row for row.
    ("अक्षरभेदकोशः", "grapheme-break.tsv"),
    // ADDED 2026-08-30, and these two complete a set rather than starting one.
    // `अक्षरभेदकोशः` above answered ONE THIRD of `अक्षराणि`'s blocker, not half:
    // the stub names THREE tables and UAX #29 needs all three. GB9c — the rule
    // that makes क्ष ONE akṣara — is undecidable without InCB, and GB11 without
    // Extended_Pictographic.
    //
    // AN AGENT MEASURED THE COST rather than asserting it, against
    // GraphemeBreakTest.txt with a reference first validated at 0 failures of
    // 766 cases: without InCB 16 fail, without Extended_Pictographic 3, without
    // both 19. It also refuted the obvious shortcut — faking GB9c from
    // Devanagari ranges still fails 9 of those 16, because the corpus exercises
    // the conjunct rule in FIVE scripts and only 7 of the cases are Devanagari.
    // So the shortcut would be silently wrong in four scripts while claiming
    // UAX #29, which is exactly what the stub existed to refuse.
    //
    // Both are generated by `tools/gen-incb.py` from the PINNED UCD and
    // asserted identical to it CODE POINT BY CODE POINT — 3148 and 2848 — not
    // merely row for row, so a mis-parsed range boundary cannot hide.
    ("संयोगभेदकोशः", "incb.tsv"),
    ("चित्राक्षरकोशः", "extended-pictographic.tsv"),
];

/// The file a name denotes, relative to the spec root, or `None`.
#[must_use]
pub fn table_path(name: &str) -> Option<&'static str> {
    TABLES.iter().find(|(n, _)| *n == name).map(|(_, p)| *p)
}

/// Every name a program may bring in, in the order they are declared.
#[must_use]
pub fn table_names() -> Vec<&'static str> {
    TABLES.iter().map(|(n, _)| *n).collect()
}

/// Why an embed could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedError {
    /// 1-based line of the `समावेशः` that opened it.
    pub line: usize,
    /// Akṣara index — what a human counting clusters would say.
    pub aksara: usize,
    /// Byte offset into the source.
    pub byte: usize,
    /// What is wrong, naming the offending word. As [`crate::lex::LexError`],
    /// this is plain English until `A-037` routes diagnostics through the
    /// lexicon, and it already names the word rather than saying only
    /// "bad embed".
    pub reason: String,
}

impl core::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "line {}, akṣara {} (byte {}): {}",
            self.line, self.aksara, self.byte, self.reason
        )
    }
}

/// Build an [`EmbedError`] positioned at the token that opened the embed.
fn err_at(opener: &Token, reason: String) -> EmbedError {
    EmbedError {
        line: opener.line,
        aksara: opener.aksara,
        byte: opener.byte,
        reason,
    }
}

/// Resolve every `समावेशः आरभ्य name समाप्तम्` in a T1 token stream.
///
/// `spec_root` is the directory names resolve against — `spec/` in this tree.
/// It is an argument rather than a constant so that the only I/O in this crate
/// outside `main.rs` is one function whose reach the caller states, and so that
/// a test can point it at a fixture.
///
/// Tokens that are not part of an embed are passed through untouched, and the
/// four tokens of one become a single [`Kind::Str`] whose value is the file's
/// exact content and whose [`Token::text`] is the source phrase, delimiters and
/// all — the split ADR-0017 established, so a diagnostic can quote what was
/// typed while a consumer uses what it means.
///
/// # Errors
/// A malformed embed (a missing `आरभ्य`, a missing name, a missing `समाप्तम्`),
/// a name that is not in [`TABLES`], or a named file that cannot be read.
/// Every embed in the stream is attempted, so one bad name does not hide the
/// next: the whole crop of errors comes back at once, as [`crate::lex`] does.
pub fn resolve(tokens: Vec<Token>, spec_root: &Path) -> Result<Vec<Token>, Vec<EmbedError>> {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut errors: Vec<EmbedError> = Vec::new();
    let mut i = 0usize;

    while i < tokens.len() {
        // A string is already whole — ADR-0017 had the lexer take it before any
        // word was looked at — so `उक्तम् समावेशः इति` never reaches this test
        // as the word `समावेशः`. It arrives as one `Kind::Str` and falls through
        // to the passthrough below, which is what makes the embed a VALUE
        // rather than an escape: there is no position at which one spelling
        // means two things. ADR-0018's rule 2, applied here.
        if !matches!(tokens[i].kind, Kind::Str { .. }) && tokens[i].text == EMBED_WORD {
            match take_embed(&tokens, i, spec_root) {
                Ok((token, next)) => {
                    out.push(token);
                    i = next;
                }
                Err((e, next)) => {
                    errors.push(e);
                    i = next;
                }
            }
            continue;
        }
        out.push(tokens[i].clone());
        i += 1;
    }

    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// Read one embed starting at `at`, where `tokens[at]` is the `समावेशः`.
///
/// Returns the token it becomes and the index to continue from, or the error
/// and the index to continue from — the caller resumes after the phrase either
/// way, so a malformed embed produces one diagnostic and not a cascade.
fn take_embed(
    tokens: &[Token],
    at: usize,
    spec_root: &Path,
) -> Result<(Token, usize), (EmbedError, usize)> {
    let opener = &tokens[at];

    let open = tokens.get(at + 1);
    if open.map(|t| t.text.as_str()) != Some(GROUP_OPEN) {
        return Err((
            err_at(
                opener,
                format!(
                    "`{EMBED_WORD}` must be followed by `{GROUP_OPEN}`, found {}",
                    quoted(open)
                ),
            ),
            at + 1,
        ));
    }

    let Some(name_tok) = tokens.get(at + 2) else {
        return Err((
            err_at(
                opener,
                format!("`{EMBED_WORD} {GROUP_OPEN}` names no table before the source ends"),
            ),
            at + 2,
        ));
    };
    let name = name_tok.text.as_str();
    if name == GROUP_CLOSE {
        return Err((
            err_at(
                opener,
                format!("`{EMBED_WORD} {GROUP_OPEN} {GROUP_CLOSE}` names no table"),
            ),
            at + 3,
        ));
    }

    let close = tokens.get(at + 3);
    if close.map(|t| t.text.as_str()) != Some(GROUP_CLOSE) {
        return Err((
            err_at(
                opener,
                format!(
                    "`{EMBED_WORD} {GROUP_OPEN} {name}` must be closed by `{GROUP_CLOSE}`, found {}",
                    quoted(close)
                ),
            ),
            at + 3,
        ));
    }

    // The refusal names every table that exists. A name that is not in the set
    // is the only way to get a file wrong, so the diagnostic that reports it is
    // the whole of the user's map of the surface.
    let Some(rel) = table_path(name) else {
        return Err((
            err_at(
                opener,
                format!(
                    "`{name}` is not a table this program may bring in; the tables are: {}",
                    table_names().join(", ")
                ),
            ),
            at + 4,
        ));
    };

    let path = spec_root.join(rel);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return Err((
                err_at(
                    opener,
                    format!("`{name}` names `{rel}`, which could not be read: {e}"),
                ),
                at + 4,
            ));
        }
    };

    Ok((
        Token {
            kind: Kind::Str { value: content },
            text: format!("{EMBED_WORD} {GROUP_OPEN} {name} {GROUP_CLOSE}"),
            byte: opener.byte,
            aksara: opener.aksara,
            line: opener.line,
        },
        at + 4,
    ))
}

/// A token's source text for a diagnostic, or `end of source`.
fn quoted(t: Option<&Token>) -> String {
    t.map_or_else(
        || "the end of the source".to_string(),
        |t| format!("`{}`", t.text),
    )
}
