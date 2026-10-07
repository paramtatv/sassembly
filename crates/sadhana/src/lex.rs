//! T0 lexer — Devanagari source to tokens, per the frozen `spec/grammar-t0.ebnf`.
//!
//! Part of **साधनम्** (Sadhana), doc 17's name for the toolchain: assembler,
//! linker and compiler as one instrument.
//!
//! # The atom is the akṣara, not the code point
//!
//! `क्ष` is *one* token character. Splitting it would let two spellings of one
//! identifier exist, which is the confusable class doc 15 exists to remove — so
//! every offset this lexer reports is an akṣara index as well as a byte offset,
//! and every slice it takes is on a cluster boundary.
//!
//! # The collision that shapes the design
//!
//! The destination sigil is `म्`. Most mnemonics are neuter nominatives and end
//! in `-म्` too: `गुणनम्`, `निधानम्`, `युक्तम्`, `वामसरणम्`, `लङ्घनम्`. So
//! `गुणनम्` read as an operand is "गुणन, destination", and read as a verb it is
//! "multiply". The two are not distinguishable by shape at all.
//!
//! **Position disambiguates, and only position** (ADR-0004): the first word of a
//! statement is the verb. But *the lexer does not decide that*, and cannot —
//! statements end at the daṇḍa and may span lines, so "first word of a
//! statement" is not a lexical property. A first version classified the first
//! word of each **line** as the verb, which broke every multi-line statement and
//! made a verbless statement report "unknown mnemonic" instead of naming the
//! rule.
//!
//! So this module emits [`Kind::Word`] and [`Kind::Operand`] and stops there.
//! The parser applies position, and resolves against the registry — which is
//! also the right authority, since R-02-1 makes that file the ISA surface.
//!
//! # Numerals are matched whole
//!
//! `०षोड्८` segments as `[०][षो][ड्][८]`: the radix prefix is not one cluster
//! and cannot be recognised cluster-by-cluster. Numerals are therefore tested
//! against `sanskrit_text::is_numeral` on the whole word, before any other
//! classification.

use sanskrit_text::{aksharas, is_numeral, repertoire};

/// A kāraka role, from doc 02 §2.2 decision D-02-C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Karaka {
    /// कर्म `म्` — write here.
    Destination,
    /// करण `न` — read from.
    Source,
    /// अपादान `त्` — load from this address.
    SourceAddress,
    /// सम्प्रदान `य्` — store to this address.
    DestAddress,
    /// अधिकरण `ए` — at / in.
    Locus,
}

impl Karaka {
    /// The sigil that marks this role.
    #[must_use]
    pub fn sigil(self) -> &'static str {
        match self {
            Karaka::Destination => "म्",
            Karaka::Source => "न",
            Karaka::SourceAddress => "त्",
            Karaka::DestAddress => "य्",
            Karaka::Locus => "ए",
        }
    }

    /// The role's own name, for a diagnostic to say out loud.
    ///
    /// `{:?}` on this enum prints `Destination`, which is an English word in
    /// the middle of a Sanskrit sentence — the fault `B-078b` found and split
    /// two codes to avoid. The kāraka has a name of its own; it is the term the
    /// documents use and the one D-02-C ratified.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Karaka::Destination => "कर्म",
            Karaka::Source => "करण",
            Karaka::SourceAddress => "अपादान",
            Karaka::DestAddress => "सम्प्रदान",
            Karaka::Locus => "अधिकरण",
        }
    }

    /// Every kāraka. One list, so a sixth cannot be added to some places only.
    const ALL: [Karaka; 5] = [
        Karaka::Destination,
        Karaka::Source,
        Karaka::SourceAddress,
        Karaka::DestAddress,
        Karaka::Locus,
    ];

    /// The role a sigil marks, if it is one.
    #[must_use]
    fn from_sigil(s: &str) -> Option<Self> {
        Karaka::ALL.into_iter().find(|k| k.sigil() == s)
    }

    /// Whether this sigil fuses with a preceding virama into one akṣara.
    ///
    /// The answer is membership of `हल्`, derived from the Māheśvara sūtras
    /// rather than listed here (`B-046`). Listing it is what went wrong in
    /// ADR-0005: the four consonant sigils were enumerated by hand, `न` was
    /// left out because the reasoning behind the list was wrong, and 439
    /// conformance cases failed. A sixth sigil now classifies itself.
    #[must_use]
    fn conjoins_after_virama(self) -> bool {
        sanskrit_text::phonology::is_consonant(self.sigil())
    }
}

/// What a token is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// An operand, carrying its role and the text before the sigil.
    Operand {
        /// The operand text with the sigil removed — the register or label.
        base: String,
        /// The role the sigil marked.
        karaka: Karaka,
    },
    /// A numeral without a sigil, e.g. an immediate in a directive.
    Numeral,
    /// A bare word: label name, directive name, or an operand missing a sigil.
    Word,
    /// `।` statement end.
    Danda,
    /// `॥` directive end.
    DoubleDanda,
    /// `ॱॱ` the colon role — a label mark.
    LabelMark,
    /// `ॱ` member access.
    MemberMark,
    /// `ऽ` argument separator.
    Separator,
    /// A string literal, `उक्तम् … इति`, or one of ADR-0018's layout words.
    /// **T1 only** — see [`lex_t1`].
    Str {
        /// The text between the delimiters: `इति इति` read as a literal `इति`
        /// (ADR-0011), and interior whitespace kept exactly as written.
        ///
        /// For a layout word it is the one character the word names — `यतिः` is
        /// `"\n"` and `विवरम्` is `" "`. They carry this kind rather than one of
        /// their own so that every reader of a string reads them too; see
        /// [`layout_value`].
        ///
        /// [`Token::text`] carries the source instead, delimiters and all, so
        /// a diagnostic can quote what was typed and a consumer can use what
        /// it means without either having to reconstruct the other.
        value: String,
    },
}

/// One token, with enough position to point at in a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What it is.
    pub kind: Kind,
    /// The source text, exactly.
    pub text: String,
    /// Byte offset of the first character.
    pub byte: usize,
    /// Akṣara index — what a human counting clusters would say.
    pub aksara: usize,
    /// 1-based line.
    pub line: usize,
}

/// Why lexing stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    /// The offending akṣara, not the offending byte: a diagnostic that named a
    /// code point would point at half a letter.
    pub aksara: String,
    /// Byte offset.
    pub byte: usize,
    /// Akṣara index.
    pub index: usize,
    /// 1-based line.
    pub line: usize,
    /// What is wrong, in English, naming the offending akṣara. Doc 01 §1.2.4
    /// will route this through the lexicon; until `A-037` lands it is plain
    /// text, but it already names the character rather than saying only
    /// "invalid input" — in a script the reader may be learning, that
    /// difference is the whole value of the message.
    pub reason: String,
}

impl core::fmt::Display for LexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "line {}, akṣara {} (byte {}): {}",
            self.line, self.index, self.byte, self.reason
        )
    }
}

/// `उक्तम्`, "spoken" — opens a string literal (ADR-0003).
const STRING_OPEN: &str = "उक्तम्";
/// `इति`, "thus" — closes one, and doubled is the word itself (ADR-0011).
const STRING_CLOSE: &str = "इति";
/// `॰` U+0970. Outside a string it is the comment mark; inside one, ADR-0017
/// makes it ordinary text.
const COMMENT_MARK: char = '॰';

/// `यतिः`, *"the caesura, the pause that divides a metrical line"* — LINE FEED
/// (U+000A) as a value (ADR-0018). Doc 17a §3.3 already names `renderer`'s line
/// breaker with this word, for the same concept.
const NEWLINE_WORD: &str = "यतिः";
/// `विवरम्`, *"an opening, an interstice"* — SPACE (U+0020) as a value
/// (ADR-0018).
const SPACE_WORD: &str = "विवरम्";

/// The layout character a word names, if the word names one.
///
/// ADR-0018. R-15-1 (doc 15 §1) admits *"exactly three layout characters"* and
/// this names two of them; TAB has no word because nothing in the tree wants
/// one, which is ADR-0017's rule for an escape applied to a value.
///
/// **These are values and not escapes**, and the difference is where they are
/// recognised. [`pieces`] takes a `उक्तम् … इति` literal WHOLE before any word
/// is looked at, so inside a string `यतिः` is four akṣaras of text; outside one
/// it is a newline. There is no lead-in character, no lexer state, and no
/// position at which one spelling means two things — which is what ADR-0011
/// refused when it killed `॰` as an escape introducer.
///
/// The value is handed back as a [`Kind::Str`], so every reader of a string
/// literal reads a layout word with no second rule. ADR-0017 made one
/// implementation of what a string is out of three that disagreed; a layout
/// word each parser had to recognise for itself would have started that count
/// over.
fn layout_value(word: &str) -> Option<&'static str> {
    match word {
        NEWLINE_WORD => Some("\n"),
        SPACE_WORD => Some(" "),
        _ => None,
    }
}

/// Whether the lexer takes `उक्तम् … इति` as one token.
///
/// The two tiers **spell** a string identically — ADR-0003's words with
/// ADR-0011's doubled close — and differ only in which stage reads it. T0's
/// parser reassembles the literal from words ([`crate::parse`]'s
/// `string_body`); T1's lexer takes it whole, which is what lets it hold a
/// space and a `॰`. ADR-0017 records why the difference exists and that it is
/// meant to end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Strings {
    /// `उक्तम्` is an ordinary word; the parser finds the close.
    Words,
    /// The lexer consumes the literal and emits one [`Kind::Str`].
    Token,
}

/// One unit of a line as the token loop wants it.
enum Piece<'a> {
    /// A whitespace-delimited word.
    Word {
        /// Byte offset within the line.
        at: usize,
        /// The word.
        text: &'a str,
    },
    /// A whole string literal.
    Str {
        /// Byte offset of the opening `उक्तम्` within the line.
        at: usize,
        /// The source exactly, `उक्तम्` through the closing `इति`.
        raw: &'a str,
        /// The text between them, doubling resolved.
        value: String,
        /// ADR-0044: the source between the delimiters, unresolved — from the
        /// first body word to the end of the last — and its offset within the
        /// line, so a `वर्णाष्टकम्` refusal can name a column. Empty for an
        /// empty literal.
        body: (usize, &'a str),
    },
}

/// The text of a `उक्तम् … इति` literal, and the index in `words` of the word
/// that closed it.
///
/// `open` indexes the `उक्तम्`. ADR-0011: a pair of `इति` is a literal `इति`
/// and the string continues; the first unpaired one closes, which scanning left
/// to right makes unambiguous.
///
/// **The text is copied out of the source rather than rebuilt from the words**,
/// which is the whole difference this token makes. `split_whitespace` had
/// already thrown the spacing away by the time any parser could ask for it, so
/// three separate parsers joined the words back with exactly one space and
/// guessed. Here a run of three spaces survives as three.
///
/// The whitespace separating a delimiter from the body belongs to the
/// delimiter — the delimiters are words, and words are whitespace-separated —
/// so a string neither begins nor ends with a space.
fn string_value(line: &str, words: &[(usize, &str)], open: usize) -> Option<(String, usize)> {
    let mut value = String::new();
    // The run of source not yet copied out: from the first character after the
    // opener's whitespace, to the end of the last word taken as text.
    let mut start = words.get(open + 1)?.0;
    let mut end = start;
    let mut i = open + 1;

    while i < words.len() {
        let (at, w) = words[i];
        let (head, tail) = split_trailing_punct(w);
        if head == STRING_CLOSE {
            // A pair is the word itself. Both members must be bare: only the
            // one that CLOSES can carry a daṇḍa written against it.
            if tail.is_none() && words.get(i + 1).map(|(_, n)| *n) == Some(STRING_CLOSE) {
                // Copy through the first `इति` and resume after the second, so
                // the space between them goes with the pair rather than the
                // text.
                value.push_str(&line[start..at + w.len()]);
                start = words[i + 1].0 + STRING_CLOSE.len();
                end = start;
                i += 2;
                continue;
            }
            value.push_str(&line[start..end]);
            return Some((value, i));
        }
        end = at + w.len();
        i += 1;
    }
    None
}

/// Cut one line into the pieces the token loop consumes.
///
/// **The order of the two cuts is the whole of ADR-0017.** This was
/// `let code = line.split('॰').next()` followed by `split_whitespace`, so the
/// comment was stripped *before* anything else and a `॰` inside a string was
/// deleted before a string could exist. Here the string is recognised first and
/// the comment mark is looked for only outside one.
///
/// In [`Strings::Words`] the result is what it always was, to the byte: no
/// string is recognised, so the first `॰` anywhere on the line still ends it.
///
/// # Errors
/// The byte offset within the line of a `उक्तम्` that no `इति` closes.
fn pieces(line: &str, strings: Strings) -> Result<Vec<Piece<'_>>, usize> {
    // Word starts, by `find` from a moving cursor rather than pointer
    // arithmetic, because that is what this lexer has always done and the
    // offsets it produces are load-bearing for every diagnostic.
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut cursor = 0usize;
    // ADR-0044 review F1/F2: words are split on R-15-1's LAYOUT characters only
    // (SPACE, TAB, and the CR a CRLF line leaves) — the `.t1` lexer's own set
    // (`lex.t1`'s `अवकाशः`). `split_whitespace` also split on NBSP, NEL, the
    // Unicode spaces and VT, so such a character was silently a separator here
    // and never met the repertoire check, while the `.t1` lexer kept it inside
    // a word: `॥ अष्टकाः वर्णाष्टकम् <NBSP>क इति ॥` assembled as [0x95] here and
    // was P19 there. Now it is inside the word on both sides and refused.
    for w in line.split(|c: char| is_layout(c)).filter(|w| !w.is_empty()) {
        let at = line[cursor..].find(w).map_or(cursor, |o| cursor + o);
        cursor = at + w.len();
        words.push((at, w));
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    while i < words.len() {
        let (at, w) = words[i];

        // ADR-0044 D2: `वर्णाष्टकम्` is a second opener and mirrors `उक्तम्`
        // EXACTLY — the same close, the same doubling, the same one-line rule —
        // so it takes the same path. What differs is its value, which
        // `lex_with` checks and `devanagari8::literal_octets` packs.
        if strings == Strings::Token && (w == STRING_OPEN || w == crate::devanagari8::OPEN) {
            let (value, close) = string_value(line, &words, i).ok_or(at)?;
            let (cat, cw) = words[close];
            // `इति ।` is how every source in the tree writes it, but `इति।`
            // must not silently fail to close.
            let (chead, ctail) = split_trailing_punct(cw);
            let body = if close > i + 1 {
                let (b0, _) = words[i + 1];
                let (bl, blw) = words[close - 1];
                (b0, &line[b0..bl + blw.len()])
            } else {
                (cat, "")
            };
            out.push(Piece::Str {
                at,
                raw: &line[at..cat + chead.len()],
                value,
                body,
            });
            if let Some(p) = ctail {
                out.push(Piece::Word {
                    at: cat + chead.len(),
                    text: p,
                });
            }
            i = close + 1;
            continue;
        }

        if let Some(mark) = w.find(COMMENT_MARK) {
            // The comment runs to end of line. A mark glued to the end of a
            // word still ends the line there, which is what splitting the whole
            // line on `॰` used to do.
            if mark > 0 {
                out.push(Piece::Word {
                    at,
                    text: &w[..mark],
                });
            }
            return Ok(out);
        }

        out.push(Piece::Word { at, text: w });
        i += 1;
    }
    Ok(out)
}

/// R-15-1's layout characters as a word separator: SPACE and TAB, and the CR a
/// CRLF line ends with (`str::lines` leaves a lone `\r` mid-line). The `.t1`
/// lexer's `अवकाशः` is the same set (it also lists LF, which `lines` consumes).
fn is_layout(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r')
}

fn punctuation(word: &str) -> Option<Kind> {
    match word {
        "।" => Some(Kind::Danda),
        "॥" => Some(Kind::DoubleDanda),
        "ॱॱ" => Some(Kind::LabelMark),
        "ॱ" => Some(Kind::MemberMark),
        "ऽ" => Some(Kind::Separator),
        _ => None,
    }
}

/// Split a written operand into its base and its kāraka.
///
/// Two cases, and the second is ADR-0005.
///
/// Normally the sigil is its own akṣara: `कम्` is `[क][म्]`, so the last cluster
/// is the role and the rest is the base.
///
/// But a virama conjoins with **whatever consonant follows it**, and that is the
/// actual mechanism — not, as ADR-0005 first said, that three of the sigils end
/// in a virama themselves. A base ending in a virama fuses with any consonant
/// sigil into ONE cluster:
///
/// | | last cluster | |
/// |---|---|---|
/// | `पुनःस्थानम्` + `म्` | `म्म्` | conjoins |
/// | `पुनःस्थानम्` + `न` | `म्न` | conjoins — and `न` has no virama at all |
/// | `पुनःस्थानम्` + `ए` | `ए` | separate; an independent vowel does not conjoin |
///
/// The first version of this function handled only `म्`, `त्` and `य्`, because
/// those were the three the (wrong) theory predicted. The conformance suite
/// found the rest: 439 of 2862 cases, every one of them `…म्न`.
/// Sanskrit's neuter nominative singular is `-अम्`, so this is not rare — 25 of
/// the 49 mnemonics end that way — and forbidding such names would ban the
/// neuter from the language to suit this function.
///
/// So when no cluster is a sigil, look inside the last one: if it ends with a
/// sigil and something precedes it, that is the role. Decidable rather than
/// heuristic, because the grammar requires every operand to carry a sigil.
fn split_sigil(word: &str) -> Option<(&str, Karaka)> {
    let clusters: Vec<&str> = aksharas(word).collect();
    if clusters.len() > 1
        && let Some(k) = Karaka::from_sigil(clusters[clusters.len() - 1])
    {
        return Some((&word[..word.len() - clusters[clusters.len() - 1].len()], k));
    }
    // ADR-0005: the sigil was absorbed into the final conjunct.
    let last = clusters.last()?;
    // Every CONSONANT sigil is affected; only ए escapes, being an independent
    // vowel. Omitting न here left 439 of 2862 conformance cases failing.
    for k in Karaka::ALL
        .into_iter()
        .filter(|k| k.conjoins_after_virama())
    {
        let s = k.sigil();
        if last.len() > s.len()
            && last.ends_with(s)
            && let Some(base) = word.strip_suffix(s)
            && !base.is_empty()
        {
            return Some((base, k));
        }
    }
    None
}

/// Split trailing punctuation off a word: `कम्।` is an operand and a daṇḍa,
/// written without a space because nothing requires one.
fn split_trailing_punct(word: &str) -> (&str, Option<&str>) {
    // `ॱॱ` first, and before the single `ॱ` could ever be considered: a label
    // is written against its name, `प्रारम्भःॱॱ`, exactly as `foo:` is. Peeling
    // it here is what makes the label mark a token instead of the last two
    // characters of an identifier — `प्रारम्भःॱॱ` lexed as one Word before, so
    // no label could be defined at all (`B-007`).
    //
    // The width suffix uses the same character singly (`आहारःॱअ८`) and is not
    // affected: it never ENDS in `ॱ`, because a member access has a member.
    for p in ["ॱॱ", "।", "॥", "ऽ"] {
        if word.len() > p.len()
            && word.ends_with(p)
            && let Some(head) = word.strip_suffix(p)
        {
            return (head, Some(p));
        }
    }
    (word, None)
}

/// Tokenise T0 source.
///
/// Every akṣara is checked against the doc 15 repertoire first, so a character
/// that could not be written in Sassembly is rejected before anything tries to
/// interpret it.
///
/// # Errors
/// Returns every violation found rather than only the first: a source file with
/// four stray Latin letters should report four, not four compile runs.
pub fn lex(source: &str) -> Result<Vec<Token>, Vec<LexError>> {
    lex_with(source, Strings::Words)
}

/// Tokenise T1 source.
///
/// Identical to [`lex`] in two respects only, and both are about what a string
/// is. `उक्तम् … इति` is a single [`Kind::Str`] token, taken **before** the
/// comment mark is stripped, so a string may hold a space and a `॰` (ADR-0017);
/// and `यतिः` and `विवरम्` are [`Kind::Str`] values of one layout character each
/// (ADR-0018), so text may cross a line and begin with a space without the
/// string doing either.
///
/// # Why there are two entry points and not one
///
/// One `lex` that always took the string would be the better end state and
/// breaks T0 today: [`crate::parse`] tests `args.first() == Some(&"उक्तम्")`
/// against a word stream that would no longer contain the word, so every T0
/// string datum would be read as a symbol address. `spec/grammar-t0.ebnf` is
/// frozen and spells `string` as a phrase. Moving T0 onto the token is a
/// successor row; until it lands the two tiers agree on the *spelling* of a
/// string and differ on which stage reads it.
///
/// # Errors
/// As [`lex`], plus a `उक्तम्` that no `इति` closes on the same line.
pub fn lex_t1(source: &str) -> Result<Vec<Token>, Vec<LexError>> {
    lex_with(source, Strings::Token)
}

fn lex_with(source: &str, strings: Strings) -> Result<Vec<Token>, Vec<LexError>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut in_ascii = false;
    let mut in_jal = false;

    let mut current_akshara_count = 0;
    let mut prev_byte = 0;

    for (line_no, line) in source.lines().enumerate() {
        let line_no = line_no + 1;
        let line_start = line.as_ptr() as usize - source.as_ptr() as usize;

        // The comment mark runs to end of line — a comment may legitimately
        // contain anything the repertoire allows — but in `Strings::Token` a
        // string is recognised FIRST, so a `॰` inside one is text. ADR-0017.
        let line_pieces = match pieces(line, strings) {
            Ok(p) => p,
            Err(at) => {
                let byte = line_start + at;
                current_akshara_count += aksharas(&source[prev_byte..byte]).count();
                prev_byte = byte;
                // ADR-0044: the opener that was left open, by name.
                let opener = if line[at..].starts_with(crate::devanagari8::OPEN) {
                    crate::devanagari8::OPEN
                } else {
                    STRING_OPEN
                };
                errors.push(LexError {
                    aksara: opener.to_string(),
                    byte,
                    index: current_akshara_count,
                    line: line_no,
                    // Refusing is the decision. A string that ran to end of
                    // file would turn one missing word into a diagnostic about
                    // the last line of the program.
                    reason: format!(
                        "`{opener}` opens a string that no `{STRING_CLOSE}` closes on this line"
                    ),
                });
                continue;
            }
        };

        for piece in line_pieces {
            let (at, word) = match &piece {
                Piece::Word { at, text } => (*at, *text),
                // The repertoire is checked over the WHOLE literal, delimiters
                // and interior whitespace included: a string is not a hole in
                // R-15-1, which is the one way this change could have been
                // worse than the gap it filled.
                Piece::Str { at, raw, .. } => (*at, *raw),
            };
            let opener_is_d8 = matches!(&piece, Piece::Str { raw, .. }
                if raw.strip_prefix(crate::devanagari8::OPEN)
                    .is_some_and(|r| r.is_empty() || r.starts_with(is_layout)));
            let byte = line_start + at;
            if prev_byte > byte {
                panic!(
                    "DEBUG: prev_byte {} > byte {}, line_start {}, at {}, line_no {}",
                    prev_byte, byte, line_start, at, line_no
                );
            }

            current_akshara_count += aksharas(&source[prev_byte..byte]).count();
            prev_byte = byte;
            let index = current_akshara_count;

            // ADR-0044 D2: a `वर्णाष्टकम्` literal holds Devanagari letters and
            // nothing else, and the refusal is the owner's, named — checked
            // BEFORE the repertoire so a Latin letter or an ASCII digit is
            // refused by this rule and not by doc 15's general one. The body is
            // the source between the delimiters; the one body that is not its
            // own letters is a lone `इति इति` pair, whose value is `इति`.
            if opener_is_d8
                && let Piece::Str {
                    body: (body_at, body),
                    ..
                } = &piece
            {
                let pair = body
                    .split(|c: char| is_layout(c))
                    .filter(|w| !w.is_empty())
                    .collect::<Vec<_>>()
                    == ["इति", "इति"];
                if !pair && let Some((off, ch)) = crate::devanagari8::first_outside(body) {
                    let in_line = body_at + off;
                    let column = line[..in_line].chars().count() + 1;
                    errors.push(LexError {
                        aksara: ch.to_string(),
                        byte: line_start + in_line,
                        index: index + aksharas(&line[at..in_line]).count(),
                        line: line_no,
                        reason: format!(
                            "{}: `{}` (U+{:04X}) at column {column} is not a Devanagari \
                             letter — a `{}` literal holds only U+0900–U+097F, one octet \
                             per letter (ADR-0044)",
                            crate::devanagari8::REFUSAL,
                            ch.escape_debug(),
                            u32::from(ch),
                            crate::devanagari8::OPEN,
                        ),
                    });
                    continue;
                }
            }

            if word == "आस्की" {
                in_ascii = true;
            } else if word == "जाल" {
                in_jal = true;
            }

            if !in_ascii {
                // If in_jal, we allow '<' and '>' specifically, but check everything else.
                // Repertoire first: a character outside doc 15 is not a token of
                // any kind, and interpreting it would be inventing meaning.
                // One diagnostic per offending WORD, not per offending character.
                let violations = repertoire::check(word);
                let violations: Vec<_> = violations
                    .into_iter()
                    .filter(|v| {
                        if in_jal {
                            let c = v.ch;
                            if c == '<'
                                || c == '>'
                                || c == '='
                                || c == '"'
                                || c == '/'
                                || c == '-'
                                || c == '!'
                                || c == ' '
                                || c == '{'
                                || c == '}'
                                || c == ';'
                                || c == ':'
                                || c.is_ascii_lowercase()
                                || c.is_ascii_uppercase()
                            {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                if let Some(first) = violations.first() {
                    let more = violations.len() - 1;
                    // W-349. The repertoire sentence is TRUE of a brace and is not
                    // the diagnosis: an ASCII brace or bracket in a source is, in
                    // practice, a template placeholder some generator did not
                    // substitute (the report was an f-string split by an edit, and
                    // a 268 s build to find it). So the word is refused exactly as
                    // before and the refusal says where to look.
                    //
                    // THE HINT SITS BETWEEN THE SENTENCE AND THE COUNT, WITH NO
                    // ` (` OF ITS OWN. Six test files filter on the sentence, and
                    // `sas_reachability.rs` recovers the count by
                    // `split_once(" (")` then `strip_suffix(" more in this word)")`
                    // — a parenthesis here, or a hint after the count, would turn
                    // every braced word's count into 1 without reddening anything.
                    //
                    // ANY violation in the word, not only the first: `x{0}` names
                    // `x`, and the brace is still why the word exists.
                    let placeholder = violations
                        .iter()
                        .any(|v| matches!(v.ch, '{' | '}' | '[' | ']'));
                    let hint = if placeholder {
                        "; an ASCII brace or bracket in a source is almost always a template \
                         placeholder its generator did not substitute, so look at whatever \
                         wrote this file"
                    } else {
                        ""
                    };
                    errors.push(LexError {
                        aksara: first.ch.to_string(),
                        byte: byte + first.at,
                        index,
                        line: line_no,
                        reason: if more == 0 {
                            format!("`{}` is outside the doc 15 repertoire{hint}", first.ch)
                        } else {
                            format!(
                                "`{}` is outside the doc 15 repertoire{hint} ({more} more in this word)",
                                first.ch
                            )
                        },
                    });
                    continue;
                }
            }

            // A string is already whole: no sigil to split, no trailing
            // punctuation to peel — `pieces` peeled a glued daṇḍa off the close
            // and handed it over as its own piece.
            if let Piece::Str { raw, value, .. } = piece {
                tokens.push(Token {
                    kind: Kind::Str { value },
                    text: raw.into(),
                    byte,
                    aksara: index,
                    line: line_no,
                });
                continue;
            }

            let (head, trailing) = split_trailing_punct(word);

            // ADR-0018: a layout character is a word, and the word is a VALUE.
            // T1 only, for ADR-0017's reason — `lex` keeps its token stream to
            // the byte, so no T0 `ॱदत्त` holding text changes meaning. The test
            // is made before `punctuation`, `is_numeral` and `split_sigil`,
            // because a keyword is not any of those and `विवरम्` would otherwise
            // read as an operand: it ends in `म्`, the कर्म sigil.
            if strings == Strings::Token
                && let Some(value) = layout_value(head)
            {
                tokens.push(Token {
                    kind: Kind::Str {
                        value: value.into(),
                    },
                    text: head.into(),
                    byte,
                    aksara: index,
                    line: line_no,
                });
            } else if let Some(k) = punctuation(head) {
                if k == Kind::DoubleDanda {
                    in_ascii = false;
                    in_jal = false;
                }
                tokens.push(Token {
                    kind: k,
                    text: head.into(),
                    byte,
                    aksara: index,
                    line: line_no,
                });
            } else if is_numeral(head) {
                tokens.push(Token {
                    kind: Kind::Numeral,
                    text: head.into(),
                    byte,
                    aksara: index,
                    line: line_no,
                });
            } else {
                match split_sigil(head) {
                    Some((base, karaka)) => {
                        tokens.push(Token {
                            kind: Kind::Operand {
                                base: base.to_string(),
                                karaka,
                            },
                            text: head.into(),
                            byte,
                            aksara: index,
                            line: line_no,
                        });
                    }
                    None => tokens.push(Token {
                        kind: Kind::Word,
                        text: head.into(),
                        byte,
                        aksara: index,
                        line: line_no,
                    }),
                }
            }

            if let Some(p) = trailing
                && let Some(k) = punctuation(p)
            {
                if k == Kind::DoubleDanda {
                    in_ascii = false;
                    in_jal = false;
                }
                let pbyte = byte + head.len();
                // THE RUNNING COUNT PLUS THE HEAD, NOT A RESCAN OF THE WHOLE
                // SOURCE. This was `aksharas(&source[..pbyte]).count()` — one
                // segmentation of everything before the token, for every trailing
                // punctuation token, which is the danda that ends nearly every
                // line: O(lines × size). Measured 2026-09-14 on the fixpoint
                // module's emitted text (36k data lines, 40 MB): the .t1 front
                // half finished in 28 min and this lexer then spun for hours
                // in one frame with memory frozen; 1,000 literals (9 MB) did not
                // lex in 40 s. `current_akshara_count` already holds the count up
                // to `byte`, and the head is a whole word, so its own count
                // finishes the sum. Falsified by token-for-token identity over
                // every spec file and an emitted module text.
                tokens.push(Token {
                    kind: k,
                    text: p.into(),
                    byte: pbyte,
                    aksara: current_akshara_count + aksharas(head).count(),
                    line: line_no,
                });
            }
        }
        // At end of line, Sassembly doesn't enforce single-line directives, but we can't be sure
        // wait, actually we don't need to do anything at end of line.
    }

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Kind> {
        lex(src)
            .expect("lexes")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn the_worked_example_from_doc_02_lexes() {
        // योगः  कम्  खन  गन ।   →   क ← ख + ग
        let k = kinds("योगः कम् खन गन ।");
        assert_eq!(k.len(), 5);
        assert_eq!(k[0], Kind::Word, "योगः has no sigil, so it lexes as a word");
        assert_eq!(
            k[1],
            Kind::Operand {
                base: "क".into(),
                karaka: Karaka::Destination
            }
        );
        assert_eq!(
            k[2],
            Kind::Operand {
                base: "ख".into(),
                karaka: Karaka::Source
            }
        );
        assert_eq!(k[4], Kind::Danda);
    }

    #[test]
    fn load_and_store_differ_only_in_the_sigil() {
        // This is the claim doc 02 §2.2 makes for the design: no bracket
        // convention, and no way to write a store that looks like a load.
        let load = kinds("आहारः कम् खत् ०षोड्८ ।");
        let store = kinds("निधानम् खय् ०षोड्८ कन ।");
        assert!(matches!(
            load[2],
            Kind::Operand {
                karaka: Karaka::SourceAddress,
                ..
            }
        ));
        assert!(matches!(
            store[1],
            Kind::Operand {
                karaka: Karaka::DestAddress,
                ..
            }
        ));
    }

    #[test]
    fn a_virama_final_name_still_takes_its_sigil() {
        // ADR-0005. पुनःस्थानम् (ra) ends in म्, and so does the destination
        // sigil, so the two conjoin into one akṣara and the sigil vanishes from
        // the cluster list. Before this, `ra` could not be written as a
        // destination at all.
        let toks = lex("योगः पुनःस्थानम्म् क्षणिक१न ।").expect("lexes");
        assert_eq!(
            toks[1].kind,
            Kind::Operand {
                base: "पुनःस्थानम्".into(),
                karaka: Karaka::Destination
            },
            "the base must keep its own म्"
        );
    }

    #[test]
    fn the_conjunct_split_applies_to_every_virama_sigil() {
        // म् त् and य् all end in a virama; न and ए do not and were never
        // affected.
        for (src, want) in [
            ("पुनःस्थानम्त्", Karaka::SourceAddress),
            ("पुनःस्थानम्य्", Karaka::DestAddress),
        ] {
            let toks = lex(&format!("आहारः {src} ।")).expect("lexes");
            assert_eq!(
                toks[1].kind,
                Kind::Operand {
                    base: "पुनःस्थानम्".into(),
                    karaka: want
                },
                "{src}"
            );
        }
    }

    #[test]
    fn a_separate_sigil_still_wins_over_the_conjunct_rule() {
        // The ordinary case must not regress: when the sigil IS its own akṣara,
        // that reading is taken first and the conjunct rule never runs.
        let toks = lex("योगः क्षणिक०म् ।").expect("lexes");
        assert_eq!(
            toks[1].kind,
            Kind::Operand {
                base: "क्षणिक०".into(),
                karaka: Karaka::Destination
            }
        );
    }

    #[test]
    fn a_bare_sigil_is_not_an_operand() {
        // `म्` alone has no base. Splitting it would produce an empty name.
        let toks = lex("योगः म् ।").expect("lexes");
        assert_eq!(toks[1].kind, Kind::Word, "a lone sigil is not an operand");
    }

    #[test]
    fn the_lexer_does_not_guess_word_class() {
        // गुणनम् ends in म्, the destination sigil, so lexically it IS an
        // operand shape. Deciding otherwise here would require knowing where the
        // statement began, which is not a lexical property — a statement ends at
        // the daṇḍa and may span lines. The parser resolves it, against the
        // registry, which R-02-1 makes the authority anyway.
        let k = kinds("गुणनम् कम् खन गन ।");
        assert!(
            matches!(k[0], Kind::Operand { .. }),
            "the lexer reports shape, not class: {:?}",
            k[0]
        );
    }

    #[test]
    fn conjuncts_are_one_aksara_and_survive_the_split() {
        // क्षम् is [क्ष][म्]. If the lexer split code points it would produce a
        // base of "क्" and lose the conjunct entirely.
        let toks = lex("योगः क्षम् खन ।").expect("lexes");
        assert_eq!(
            toks[1].kind,
            Kind::Operand {
                base: "क्ष".into(),
                karaka: Karaka::Destination
            }
        );
    }

    #[test]
    fn numerals_are_matched_whole_not_cluster_by_cluster() {
        // ०षोड्८ segments as [०][षो][ड्][८]; recognising it cluster-wise is
        // impossible, and its last cluster is not a sigil.
        let k = kinds("आहारः कम् ०षोड्८ ।");
        assert_eq!(k[2], Kind::Numeral);
        assert_eq!(kinds("आहारः कम् १२३ ।")[2], Kind::Numeral);
    }

    #[test]
    fn punctuation_need_not_be_spaced() {
        let k = kinds("योगः कम् खन।");
        assert_eq!(k.last(), Some(&Kind::Danda));
        assert_eq!(k.len(), 4);
    }

    #[test]
    fn comments_run_to_end_of_line_and_are_not_tokens() {
        let k = kinds("योगः कम् खन गन ।  ॰ this is ignored\nवियोगः कम् खन गन ।");
        assert_eq!(k.iter().filter(|x| **x == Kind::Word).count(), 2);
        assert_eq!(k.iter().filter(|x| **x == Kind::Danda).count(), 2);
    }

    #[test]
    fn latin_is_rejected_naming_the_offender_and_where() {
        // The doc 15 guarantee. A diagnostic that said only "invalid character"
        // in a script the reader may be learning is not a diagnostic.
        let errs = lex("योगः add खन ।").expect_err("Latin must be rejected");
        assert_eq!(errs.len(), 1, "one word wrong is one diagnostic: {errs:?}");
        assert_eq!(errs[0].line, 1);
        assert_eq!(
            errs[0].aksara, "a",
            "must name the offender, not describe it"
        );
        assert!(errs[0].byte > 0);
        let msg = errs[0].to_string();
        assert!(msg.contains('a') && msg.contains("repertoire"), "{msg}");
        assert!(msg.contains("2 more"), "should count the rest: {msg}");
    }

    #[test]
    fn every_violation_is_reported_not_only_the_first() {
        // Four stray words should be four diagnostics, not four compile runs.
        let errs = lex("योगः add sub mul ।").expect_err("rejected");
        assert_eq!(errs.len(), 3, "one per bad word: {errs:?}");
        assert_eq!(errs[0].aksara, "a");
        assert_eq!(errs[1].aksara, "s");
        assert_eq!(errs[2].aksara, "m");
    }

    #[test]
    fn offsets_are_aksara_counts_not_byte_counts() {
        // A Devanagari akṣara is 3–12 bytes, so a byte offset alone would be
        // meaningless to a human counting letters.
        let toks = lex("योगः कम् खन ।").expect("lexes");
        assert_eq!(toks[0].aksara, 0);
        assert!(
            toks[1].aksara < toks[1].byte,
            "akṣara index must be < byte offset"
        );
        assert!(toks[2].aksara > toks[1].aksara);
    }

    #[test]
    fn every_karaka_sigil_round_trips() {
        for k in [
            Karaka::Destination,
            Karaka::Source,
            Karaka::SourceAddress,
            Karaka::DestAddress,
            Karaka::Locus,
        ] {
            assert_eq!(Karaka::from_sigil(k.sigil()), Some(k));
        }
    }

    #[test]
    fn an_empty_source_lexes_to_nothing() {
        assert!(lex("").expect("lexes").is_empty());
        assert!(lex("   \n  \n").expect("lexes").is_empty());
        assert!(lex("॰ only a comment").expect("lexes").is_empty());
    }
}
