//! T0 parser — tokens to instruction records, per `spec/grammar-t0.ebnf`.
//!
//! # The registry is compiled in, not read at runtime
//!
//! `spec/mnemonics-riscv64.src.tsv` is `include_str!`d. Rule R-02-1 says the ISA
//! surface *is* that file and there is no `.insn 0x…` escape hatch, so the set
//! of writable instructions must be fixed when the assembler is built rather
//! than discovered from whatever happens to be on disk. It also means the
//! assembler has no runtime file dependency to go missing, and doc 03 §6's
//! reproducibility argument does not have to reason about the working directory.
//!
//! # An unknown mnemonic is an error, never a passthrough
//!
//! Assemblers traditionally let an unrecognised opcode fall through to a
//! directive, a macro, or a raw-encoding escape. R-02-1 removes all three. A
//! name absent from the registry cannot be assembled, and the diagnostic says
//! so — with the nearest registered name, because in a script the reader may be
//! learning, "unknown mnemonic" alone leaves nowhere to go.

use crate::lex::{Karaka, Kind, LexError, Token};
use crate::nidana::{Diagnostic, Language};
use sanskrit_text::numeral::NumeralError;

/// The mnemonic registry, fixed at build time. See the module note.
const REGISTRY: &str = include_str!("../../../spec/mnemonics-riscv64.src.tsv");
const WEB_DICTIONARY: &str = include_str!("../../../spec/web-dictionary.tsv");

/// Operand width — व्याप्ति, "extent".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// `ॱअ८`
    W8,
    /// `ॱअ१६`
    W16,
    /// `ॱअ३२`
    W32,
    /// `ॱअ६४` — the default when no suffix is written.
    W64,
}

impl Width {
    fn from_digits(d: &str) -> Option<Self> {
        match d {
            "८" => Some(Width::W8),
            "१६" => Some(Width::W16),
            "३२" => Some(Width::W32),
            "६४" => Some(Width::W64),
            _ => None,
        }
    }

    /// The width part of a doc 02 §2.5 type name: `प६४` is 64 bits.
    fn from_type(t: &str) -> Option<Self> {
        Self::from_digits(strip_type_class(t)?.1)
    }

    /// Bit width as a number.
    #[must_use]
    pub fn bits(self) -> u32 {
        match self {
            Width::W8 => 8,
            Width::W16 => 16,
            Width::W32 => 32,
            Width::W64 => 64,
        }
    }
}

/// One entry of the mnemonic registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    /// Latin family key, e.g. `add`.
    pub key: String,
    /// The Devanagari mnemonic.
    pub name: String,
    /// English gloss.
    pub gloss: String,
    /// RV64GC extension: `I`, `M`, `A`, `FD`, `Zicsr`, `Zifencei`.
    pub ext: String,
    /// The RISC-V encodings this family covers.
    pub covers: Vec<String>,
}

/// An operand, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operand {
    /// Text with the sigil removed.
    pub base: String,
    /// The role the sigil marked.
    pub karaka: Karaka,
    /// True when the base is a numeral rather than a register or label.
    pub is_numeral: bool,
}

/// One parsed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// The registry entry the mnemonic resolved to.
    pub family: Family,
    /// Requested width, or [`Width::W64`] by default.
    pub width: Width,
    /// The type names written after the mnemonic, in written order.
    ///
    /// Empty for a bare mnemonic, one for a व्याप्ति, two for a conversion.
    /// [`width`](Self::width) is the first one's width; the second exists only
    /// to say what a conversion READS, which no operand carries (`B-075`).
    pub types: Vec<String>,
    /// Operands in written order. Order carries no meaning — the kāraka does.
    pub operands: Vec<Operand>,
    /// 1-based line.
    pub line: usize,
}

/// Bytes a directive emitted, and where they go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datum {
    /// The section this belongs to.
    pub section: Section,
    /// The bytes, little-endian, already widened to the directive's size.
    pub bytes: Vec<u8>,
    /// Addresses this datum names but does not know yet: the byte offset into
    /// [`bytes`](Self::bytes), and the label whose address goes there.
    ///
    /// ADR-0013. A datum stops being complete when it is parsed, for the same
    /// reason an instruction's label reference is: an address is a property of
    /// the whole program, and layout has not run. Eight bytes are reserved and
    /// left zero, and the linker fills them once every label has an address.
    pub addresses: Vec<(usize, String)>,
    /// 1-based line.
    pub line: usize,
}

/// A label definition: a name, and the instruction it marks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The name as written, without the `ॱॱ`.
    pub name: String,
    /// Which section it names a place in.
    pub section: Section,
    /// Bytes into that section's data, for a label in `ॱदत्त`.
    pub data_offset: usize,
    /// Index of the instruction this label marks. Equal to the instruction
    /// count when the label sits at the end of the program.
    pub at: usize,
    /// 1-based line.
    pub line: usize,
}

/// Which section the bytes that follow belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    /// `ॱपाठ` — instructions.
    #[default]
    Text,
    /// `ॱदत्त` — initialised data.
    Data,
    /// `ॱरिक्त` — space that is zero at load and costs nothing in the file.
    ///
    /// रिक्त is "empty", already in the lexicon. A zeroed buffer in `ॱदत्त`
    /// costs its own size on disk; here it costs only address space, which is
    /// what `.bss` is for and what doc 18's budgets are about.
    Bss,
}

impl Section {
    /// The name as written, without the `ॱ`.
    #[must_use]
    fn from_name(name: &str) -> Option<Self> {
        match name.trim_start_matches('ॱ') {
            "पाठ" => Some(Section::Text),
            "दत्त" => Some(Section::Data),
            "रिक्त" => Some(Section::Bss),
            _ => None,
        }
    }
}

/// The directive registry, `spec/directives.tsv`.
const DIRECTIVES: &str = include_str!("../../../spec/directives.tsv");

/// One directive: what doc 02 §2.5 named, and whether it is implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveKind {
    /// The Devanagari name.
    pub name: String,
    /// What another assembler would call it.
    pub gas: String,
    /// `live` or `planned`.
    pub status: String,
}

/// Every directive doc 02 named.
#[must_use]
pub fn directives() -> Vec<DirectiveKind> {
    DIRECTIVES
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 5).then(|| DirectiveKind {
                name: f[0].into(),
                gas: f[1].into(),
                status: f[3].into(),
            })
        })
        .collect()
}

/// What kind of statement a line held.
///
/// A comment is absent: the lexer strips from `॰` to end of line before the
/// parser is reached (`B-067`), so it is not a statement this can see. Any
/// parser checked against this must account for that itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementKind {
    /// A sentence: a verb and its kāraka-marked operands, ending in `।`.
    Instruction,
    /// A name and its `ॱॱ`.
    Label,
    /// `॥ … ॥` — ADR-0012.
    Directive,
}

impl StatementKind {
    /// The word this kind is written as in `spec/parse-shape-t0.tsv`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            StatementKind::Instruction => "instruction",
            StatementKind::Label => "label",
            StatementKind::Directive => "directive",
        }
    }
}

/// One statement, in written order.
///
/// `Program`'s other fields record what a statement MEANT — an instruction, a
/// label, some bytes — and three directives mean nothing storable: `कोष्ठकम्`
/// changes a section, `वैश्विकम्` pushes a name, `स्थानीयम्` does nothing at
/// all. So the shape of the file was not recoverable from the parse, and a
/// parser checked against it could drop those three unseen (`B-034a`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// 1-based line.
    pub line: usize,
    /// Which of the three it is.
    pub kind: StatementKind,
    /// The first word: a mnemonic, a label's name, a directive's name.
    pub name: String,
}

/// A parsed program: instructions in order, and the labels between them.
///
/// Labels carry an instruction INDEX rather than an address. Addresses belong
/// to the encoder, which is the only part that knows how many bytes an
/// instruction takes — and `B-007`'s relaxation fixpoint is exactly the problem
/// of that answer changing as compressed forms are chosen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    /// Instructions in written order.
    pub instructions: Vec<Instruction>,
    /// Every label, in written order.
    pub labels: Vec<Label>,
    /// Data emitted by directives, in written order.
    pub data: Vec<Datum>,
    /// Names declared visible outside this object.
    pub globals: Vec<String>,
    /// Bytes reserved in `ॱरिक्त`: present in memory, absent from the file.
    pub bss: usize,
    /// Where `॥ संरेखः n ॥` asked `ॱपाठ` to be aligned: the index of the
    /// instruction the padding sits in front of, and the boundary `n`.
    ///
    /// Alignment in a text section cannot be resolved while parsing, for the
    /// same reason a branch's width cannot: a compressed instruction is two
    /// bytes and an uncompressed one is four, so how far the address is from a
    /// multiple of `n` is not known until the relaxation fixpoint in `encode`
    /// has settled. `W-071`. Recording the request and letting layout satisfy
    /// it is what makes the directive act on `ॱपाठ` at all — it previously
    /// emitted `n` bytes into `ॱदत्त`, a section the author never named.
    pub text_aligns: Vec<(usize, usize)>,
    /// Every statement in written order, whatever it meant.
    ///
    /// This is the file's SHAPE rather than its content, and it is what a
    /// second parser can be checked against (`B-034a`).
    pub statements: Vec<Statement>,
    /// Line of the first reservation in `ॱरिक्त`, or 0 if there is none.
    ///
    /// A diagnostic about `ॱरिक्त` is about a statement, and blaming line 1 for
    /// something written on line 40 is a false statement in an error message.
    pub bss_line: usize,
}

impl Instruction {
    /// The operand filling a given role, if any.
    ///
    /// This is what makes free operand order real: a backend asks for the
    /// destination rather than for "the first operand".
    #[must_use]
    pub fn by_role(&self, k: Karaka) -> Option<&Operand> {
        self.operands.iter().find(|o| o.karaka == k)
    }
}

/// Why parsing failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line.
    pub line: usize,
    /// Akṣara index into the source.
    pub aksara: usize,
    /// Which diagnostic this is, from `spec/diagnostics.tsv`.
    ///
    /// Empty while a message is still formatted by hand (`B-078e`). A caller
    /// asking *what went wrong* reads this rather than the prose, so a test
    /// does not pin whichever language the parser was built to speak.
    pub code: &'static str,
    /// What varies, in the order the template names it.
    pub args: Vec<String>,
    /// The message, in the language the parser was built to speak.
    pub reason: String,
}

impl ParseError {
    /// The message in a chosen language.
    ///
    /// A diagnostic with no code yet falls back to its rendered text.
    #[must_use]
    pub fn message(&self, lang: crate::nidana::Language) -> String {
        if self.code.is_empty() {
            return self.reason.clone();
        }
        let args: Vec<&str> = self.args.iter().map(String::as_str).collect();
        crate::nidana::Diagnostic::new(self.code, &args).render(lang)
    }
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "line {}, akṣara {}: {}",
            self.line, self.aksara, self.reason
        )
    }
}

/// Parse the registry. Cheap enough to do per call; the file is 49 rows.
#[must_use]
pub fn families() -> Vec<Family> {
    REGISTRY
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("family\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 5).then(|| Family {
                key: f[0].into(),
                name: f[1].into(),
                gloss: f[2].into(),
                ext: f[3].into(),
                covers: f[4]
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && !s.starts_with('('))
                    .map(str::to_string)
                    .collect(),
            })
        })
        .collect()
}

/// Akṣara-level edit distance, for "did you mean". Operating on clusters rather
/// than code points matters: two mnemonics differing by one conjunct differ by
/// one akṣara, and a code-point distance would call them three apart.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<&str> = sanskrit_text::aksharas(a).collect();
    let b: Vec<&str> = sanskrit_text::aksharas(b).collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        core::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn nearest<'a>(name: &str, fams: &'a [Family]) -> Option<&'a Family> {
    fams.iter()
        .map(|f| (distance(name, &f.name), f))
        // Relative, not absolute. A fixed threshold of 2 offered व्यत्ययः for
        // क्षत्रियः — both are three akṣaras, so two edits apart is two thirds
        // different, and the words share nothing. Requiring the distance to be
        // at most a third of the longer name scales with length: one edit in
        // three akṣaras, two in six. An unrelated suggestion is worse than none,
        // because it invites a wrong edit that fails somewhere else.
        .filter(|(d, f)| {
            let len = sanskrit_text::aksharas(name)
                .count()
                .max(sanskrit_text::aksharas(&f.name).count());
            // A third of the length, but never less than one: a pure ratio
            // rejected योग for योगः, which differ by a single visarga and is
            // precisely the typo worth catching. One edit is always plausible;
            // beyond that the names must be long enough to have earned it.
            *d <= (len / 3).max(1)
        })
        .min_by_key(|(d, _)| *d)
        .map(|(_, f)| f)
}

/// The text of a string literal, with `इति इति` read as a literal `इति`.
///
/// ADR-0011. `args` runs from just after `उक्तम्` to the closing `इति`
/// inclusive; `None` means no unpaired `इति` closed it.
///
/// # Why doubling
///
/// ADR-0003 named `॰` as the in-string escape and it can never work: `॰` is the
/// comment mark, and [`crate::lex::lex`] strips everything after the first one
/// on a line **before** the parser is reached. The escape was unreachable by
/// construction, not merely unimplemented.
///
/// Doubling needs no character at all, and the language already does it —
/// ADR-0003 made the label mark `ॱॱ`, a doubled `ॱ`. So a delimiter that means
/// itself is written twice, which is also what SQL and Pascal do with the quote
/// and for the same reason: the closing mark is the only thing that has to be
/// escaped, so it is the only thing that needs a spelling.
fn string_body(args: &[&str]) -> Option<(String, usize)> {
    let mut words: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "इति" {
            // A pair is the word itself and the string continues; a lone one
            // closes. Scanning left to right is what makes that unambiguous.
            if args.get(i + 1) == Some(&"इति") {
                words.push("इति");
                i += 2;
                continue;
            }
            return Some((words.join(" "), i + 1));
        }
        words.push(args[i]);
        i += 1;
    }
    None
}

/// The `उक्तम् … इति` spelling of a run of data octets, if that spelling reads
/// back as **exactly** these octets.
///
/// `SAS-011` fix (1), the writer's half of [`string_body`]. The assembler pays
/// ~3,500 interpreter steps per octet emitted as its own numeral — ten
/// characters of text each, every one classified by the tokeniser and read
/// three times by the directive. The same octets as one text piece are read
/// once, so the emitters should write that form wherever it is lossless.
///
/// # The predicate is the point, not the spelling
///
/// `None` is the safe answer and most of this function is about returning it.
/// A literal is an arbitrary run of octets (`artha.t1:679` types it as unsigned
/// octets, not characters), and the text form survives a round trip only when
/// every stage between here and [`string_body`] leaves it alone:
///
/// * **Valid UTF-8, Devanagari base block only.** R-15-1 admits four blocks,
///   but [`crate::lex`] is the gate the emitted object passes through and
///   anything outside U+0900–U+097F buys no octets that the numeral form
///   cannot carry.
/// * **No `।`, `॥`, `॰`, `ॱ` or `ऽ`.** `॥` closes the directive, `॰` ends the line
///   (ADR-0017 recognises a string first, but only in T1's lexer — an object is
///   T0 text and the comment mark still cuts it), `।` ends the statement and
///   `ॱॱ` is the label mark, and the lexer peels a trailing `ऽ` off a word.
/// * **One word: no space at all** (the owner's NARROW RULE, 2026-10-06). The
///   reader does rebuild single interior spaces, and the four spaced runs in
///   `tests/sas011-string-payload.rs` round-trip — they are refused anyway, so
///   the T1 twin (`यन्त्रपाठरूपयोग्यम्`, `yantrotsarjana.t1`) is an octet walk
///   with no word splitting. It is what the ~1.5 KB of compiler image the
///   owner accepted pays for, and it still carries 97.55% of the corpus's
///   data octets.
/// * **Not the word `इति`.** ADR-0011 spells a close inside a literal by doubling
///   it and [`string_body`] reads the doubling back, so this case is
///   expressible — it is refused anyway because the T1 twin of this routine has
///   to agree octet for octet, and a conservative refusal costs one literal
///   where a doubling mismatch costs the image.
/// * **Not the word `आस्की` or `जाल`.** Those two words put [`crate::lex`]
///   into its ASCII and markup modes for the rest of the file, and a literal
///   that silently switches off the repertoire gate for everything after it is
///   not a lossless spelling of anything.
///
/// An empty run is `None` rather than `Some("")`: `अष्टकाः` takes `1+` operands,
/// so an empty literal gets its label and no directive at all.
#[must_use]
pub fn string_payload(bytes: &[u8]) -> Option<String> {
    let text = core::str::from_utf8(bytes).ok()?;
    // ONE WORD, NO SPACE (owner ruling, 2026-10-06). A space is not a payload
    // character, so the character test refuses every spaced run; the empty run
    // has no characters and is refused by name.
    if text.is_empty() || !text.chars().all(devanagari_payload_char) {
        return None;
    }
    // The literal rather than `lex`'s `STRING_CLOSE`, which is private to
    // that module; `string_body` above writes it the same way.
    if text == "इति" || text == "आस्की" || text == "जाल" {
        return None;
    }
    Some(text.to_string())
}

/// Whether one character may stand in a `उक्तम् … इति` data literal.
///
/// The base block less the four marks [`string_payload`] names, and less the
/// avagraha `ऽ` (U+093D): `lex.rs`'s `split_trailing_punct` peels a trailing
/// one off a word as the separator token, and the reader rejoins the pieces
/// with a space, so `कऽ` came back as `क ऽ` (`SAS-011` (b)). Refused in every
/// position, not only the trailing one, so the T1 twin has one rule to match.
fn devanagari_payload_char(ch: char) -> bool {
    matches!(ch, '\u{0900}'..='\u{097F}')
        && !matches!(
            ch,
            '\u{093D}' | '\u{0964}' | '\u{0965}' | '\u{0970}' | '\u{0971}'
        )
}

/// Split a doc 02 §2.5 type name into its class letter and its digits.
///
/// `अ` is a signed integer, `न` an unsigned one and `भ` a float. A bare width
/// with no class letter is signed, which is what `ॱ३२` used to mean and still
/// does.
fn strip_type_class(t: &str) -> Option<(char, &str)> {
    let mut chars = t.chars();
    match chars.next()? {
        c @ ('अ' | 'न' | 'प') => Some((c, chars.as_str())),
        _ => Some(('अ', t)),
    }
}

/// Split a mnemonic token into its name and its type suffixes: `योगःॱअ३२`.
///
/// Most instructions write one — the व्याप्ति of what they operate on. A
/// conversion writes two, because it names a pair of types (`B-075`): in
/// `प्लवरूपान्तरम्ॱअ३२ॱप६४` the first is what is written and the second what is
/// read, in the order a reader meets them.
fn split_types(text: &str) -> (&str, Vec<&str>) {
    let mut parts = text.split('ॱ');
    let name = parts.next().unwrap_or(text);
    (name, parts.collect())
}

/// Parse a token stream into instructions.
///
/// # Errors
/// Returns every error found rather than stopping at the first, so one run
/// reports one file's problems.
pub fn parse(tokens: &[Token]) -> Result<Program, Vec<ParseError>> {
    let fams = families();
    let mut out = Program::default();
    let mut errors = Vec::new();
    let mut section = Section::default();

    // Statements are daṇḍa-terminated, so split on it rather than on lines: an
    // instruction may be written across two lines and stay one sentence.
    let mut statement: Vec<&Token> = Vec::new();
    // The `॥` that opened the directive being read, if one is open.
    let mut opened: Option<&Token> = None;
    for tok in tokens {
        if matches!(tok.kind, Kind::Danda) {
            if !statement.is_empty() {
                if section == Section::Data {
                    errors.push(ParseError {
                        line: statement[0].line,
                        aksara: statement[0].aksara,
                        code: "P01",
                        args: vec![statement[0].text.clone()],
                        reason: Diagnostic::new("P01", &[&statement[0].text])
                            .render(Language::default()),
                    });
                } else {
                    // Recorded whether or not it encodes: the shape of the file
                    // is what was WRITTEN, and a statement that fails to resolve
                    // is still a statement a second parser must see.
                    out.statements.push(Statement {
                        line: statement[0].line,
                        kind: StatementKind::Instruction,
                        name: statement[0].text.clone(),
                    });
                    match parse_statement(&statement, &fams) {
                        Ok(i) => out.instructions.push(i),
                        Err(e) => errors.push(e),
                    }
                }
                statement.clear();
            }
            continue;
        }
        // `॥` opens and closes (ADR-0012). It is a paired delimiter rather than
        // a terminator, so which of the two this one is depends on whether a
        // directive is already open — and an unpaired one must not be allowed
        // to swallow the directive after it.
        if matches!(tok.kind, Kind::DoubleDanda) {
            if opened.take().is_some() {
                if let Some(head) = statement.first() {
                    out.statements.push(Statement {
                        line: head.line,
                        kind: StatementKind::Directive,
                        name: head.text.clone(),
                    });
                }
                if statement.is_empty() {
                    errors.push(ParseError {
                        line: tok.line,
                        code: "P26",
                        args: Vec::new(),
                        aksara: tok.aksara,
                        reason: Diagnostic::new("P26", &[]).render(Language::default()),
                    });
                } else if let Err(e) = apply_directive(&statement, &mut out, &mut section) {
                    errors.push(e);
                }
            } else if let Some(first) = statement.first() {
                // Words before an unopened `॥`: the opening mark is missing.
                // Reported rather than treated as an opener, because opening
                // here would report a SECOND fault at the end of the file and
                // leave the reader two diagnostics for one omission.
                errors.push(ParseError {
                    line: first.line,
                    code: "P24",
                    args: vec![first.text.clone()],
                    aksara: first.aksara,
                    reason: Diagnostic::new("P24", &[&first.text]).render(Language::default()),
                });
            } else {
                opened = Some(tok);
            }
            statement.clear();
            continue;
        }
        // A label ends at its mark, not at a daṇḍa: it is not a sentence, it
        // names the place where the next one begins.
        if matches!(tok.kind, Kind::LabelMark) {
            // Any single token, not only a `Word`. A name ending in a kāraka
            // sigil lexes as an operand — `दत्तम्` is read as "दत्त, destination"
            // — and requiring `Word` here would bar most Sanskrit neuter nouns
            // from being labels, which is the ban ADR-0005 refused for operands.
            //
            // Position settles it exactly as ADR-0004 settles the verb: one
            // token before `ॱॱ` is a name, whatever its ending, and the token's
            // full text is that name.
            match statement.as_slice() {
                [name] if !matches!(name.kind, Kind::Danda | Kind::DoubleDanda) => {
                    out.statements.push(Statement {
                        line: name.line,
                        kind: StatementKind::Label,
                        name: name.text.clone(),
                    });
                    out.labels.push(Label {
                        name: name.text.clone(),
                        section,
                        data_offset: if section == Section::Bss {
                            out.bss
                        } else {
                            out.data.iter().map(|d| d.bytes.len()).sum()
                        },
                        at: out.instructions.len(),
                        line: name.line,
                    });
                }
                [] => errors.push(ParseError {
                    line: tok.line,
                    code: "P06",
                    args: Vec::new(),
                    aksara: tok.aksara,
                    reason: Diagnostic::new("P06", &[]).render(Language::default()),
                }),
                other => {
                    let words = other
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    errors.push(ParseError {
                        line: tok.line,
                        code: "P07",
                        args: vec![words.clone(), other.len().to_string()],
                        aksara: tok.aksara,
                        reason: Diagnostic::new("P07", &[&words, &other.len().to_string()])
                            .render(Language::default()),
                    });
                }
            }
            statement.clear();
            continue;
        }
        statement.push(tok);
    }

    // A name this file exports and does not define is a promise nothing keeps.
    //
    // Checked HERE rather than at link time, because `वैश्विकम् X` says *this
    // file defines X and shows it to others* — a claim about one translation
    // unit, which every path that reads a unit should test. `बन्धकः` tested it
    // and the object path did not, so `--वस्तु` wrote a file whose export was
    // a name nothing in it defined and said nothing (`B-096`).
    for g in &out.globals {
        if !out.labels.iter().any(|l| &l.name == g) {
            let at = out
                .statements
                .iter()
                .find(|s| s.kind == StatementKind::Directive && s.name == "वैश्विकम्")
                .map_or(1, |s| s.line);
            errors.push(ParseError {
                line: at,
                code: "L02",
                args: vec![g.clone()],
                aksara: 0,
                reason: Diagnostic::new("L02", &[g]).render(Language::default()),
            });
        }
    }

    // A directive left open swallows whatever follows it, so it is reported at
    // the mark that opened it rather than at the end of the file.
    if let Some(tok) = opened {
        errors.push(ParseError {
            line: tok.line,
            code: "P25",
            args: Vec::new(),
            aksara: tok.aksara,
            reason: Diagnostic::new("P25", &[]).render(Language::default()),
        });
    }

    // A trailing statement with no daṇḍa is not a sentence.
    if let Some(first) = statement.first() {
        errors.push(ParseError {
            line: first.line,
            code: "P02",
            args: vec![first.text.clone()],
            aksara: first.aksara,
            reason: Diagnostic::new("P02", &[&first.text]).render(Language::default()),
        });
    }

    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// Apply one `॥ … ॥` directive.
///
/// A directive doc 02 named but nothing implements is REFUSED, not ignored. An
/// assembler that quietly drops `॥ संरेखः ४ ॥` produces a program wrong by four
/// bytes and says nothing about it, which is the failure mode this project
/// keeps finding in other guises.
fn apply_directive(
    toks: &[&Token],
    out: &mut Program,
    section: &mut Section,
) -> Result<(), ParseError> {
    let head = toks[0];
    // The closure carries the code, so nine call sites become nine codes
    // rather than nine `format!`s (`B-078e`).
    let err = |code: &'static str, args: &[&str]| ParseError {
        line: head.line,
        code,
        args: args.iter().map(|a| (*a).to_string()).collect(),
        aksara: head.aksara,
        reason: Diagnostic::new(code, args).render(Language::default()),
    };

    let Some(kind) = directives().into_iter().find(|d| d.name == head.text) else {
        return Err(err("P13", &[&head.text]));
    };
    if kind.status != "live" {
        return Err(err("P14", &[&head.text, &kind.gas]));
    }

    // Operands of a directive carry no kāraka: they are arguments to the
    // assembler, not rôles in a sentence.
    let args: Vec<&str> = toks[1..].iter().map(|t| t.text.as_str()).collect();

    match head.text.as_str() {
        "कोष्ठकम्" => {
            let [name] = args[..] else {
                return Err(err("P16", &["कोष्ठकम्"]));
            };
            *section = Section::from_name(name).ok_or_else(|| err("P15", &[name]))?;
        }
        "वैश्विकम्" => {
            let [name] = args[..] else {
                return Err(err("P16", &["वैश्विकम्"]));
            };
            out.globals.push((*name).to_string());
        }
        "स्थानीयम्" => {
            let [_name] = args[..] else {
                return Err(err("P16", &["स्थानीयम्"]));
            };
            // Local is the default; naming it is documentation, not an effect.
        }
        // `संरेखः n` — "pad until the address divides by n" (spec/directives.tsv).
        //
        // It is NOT `स्थानम् n`, and reading it as one was `W-071`: the argument
        // is a BOUNDARY, not a count, so `॥ संरेखः १६ ॥` at an address already
        // divisible by sixteen emits nothing at all, while `स्थानम् १६` always
        // emits sixteen. Sharing the arm with `स्थानम्` made it emit `n` bytes
        // unconditionally, and into `ॱदत्त` whichever section was open.
        //
        // Note the gas column in `spec/directives.tsv` says `.align`, which on
        // RISC-V means 2^n and is a different directive. `.balign` is the one
        // that means this. Recorded rather than fixed here: the mapping is the
        // spec's to correct, not the assembler's to reinterpret.
        "संरेखः" => {
            let [n] = args[..] else {
                return Err(err("P16", &["संरेखः"]));
            };
            if sanskrit_text::numeral::classify(n).is_none() {
                return Err(err("P19", &[n]));
            }
            // A boundary is a MAGNITUDE, so `value` and not `bits` (`W-075`).
            // Read as a bit pattern, `ऋण१६` is 0xffff…f0 — not a power of two,
            // so it would be refused for a reason that is not the one it broke.
            let boundary = match sanskrit_text::numeral::value(n) {
                Ok(v) => usize::try_from(v).map_err(|_| err("P29", &[n]))?,
                Err(NumeralError::Signed) => return Err(err("P30", &["संरेखः", n])),
                Err(NumeralError::TooLarge) => return Err(err("P29", &[n])),
                Err(_) => return Err(err("P19", &[n])),
            };
            // Zero and one are not alignment. Zero would ask for a multiple of
            // nothing; one is satisfied by every address, so it is a statement
            // that does nothing and reads like a statement that does something.
            if boundary < 2 || !boundary.is_power_of_two() {
                return Err(err("P19", &[n]));
            }
            match *section {
                Section::Text => out.text_aligns.push((out.instructions.len(), boundary)),
                Section::Data => {
                    // `ॱदत्त` has no relaxation: the bytes emitted so far are
                    // the offset, so the padding is known now.
                    let at: usize = out.data.iter().map(|d| d.bytes.len()).sum();
                    let pad = at.next_multiple_of(boundary) - at;
                    if pad != 0 {
                        out.data.push(Datum {
                            section: *section,
                            bytes: vec![0; pad],
                            addresses: Vec::new(),
                            line: head.line,
                        });
                    }
                }
                Section::Bss => {
                    // `B-108`. This was refused, by `B-068`'s rule that only
                    // `स्थानम्` may appear in a section holding no octets. The
                    // refusal was inherited rather than reasoned: `संरेखः` sat
                    // in the same arm as the directives that emit values, so it
                    // was caught by a rule about emitting values.
                    //
                    // P20 states that rule in its own words — "`{0}` writes
                    // values, and ॱरिक्त holds no bytes — only स्थानम्". `संरेखः`
                    // writes no values. It advances the location counter, which
                    // is precisely and only what `स्थानम्` does here (`out.bss
                    // += n`); neither puts a byte in the file, because a NOBITS
                    // section has none to put. So the one directive `ॱरिक्त`
                    // already permits is the same KIND of operation as this one,
                    // and gas accepts `.balign` in `.bss` for the same reason.
                    //
                    // The cost of the refusal was real: `C-001e3a` needs a
                    // 4096-aligned arena, because buddy arithmetic walks outside
                    // a region not aligned to its own size, and had to round the
                    // base up at RUNTIME instead — where the rounding then
                    // became load-bearing in its own right.
                    //
                    // Alignment at offset zero is a no-op rather than a special
                    // case: `0` is already a multiple of every boundary, so an
                    // empty `ॱरिक्त` stays empty and `bss_line` stays unset.
                    out.bss = out.bss.next_multiple_of(boundary);
                }
            }
            return Ok(());
        }
        "आस्की" => {
            if args.is_empty() {
                return Err(err("P16", &[&head.text]));
            }
            // Simply join the args with a space since split_whitespace loses spaces.
            let text = args.join(" ");
            out.data.push(Datum {
                section: *section,
                bytes: text.into_bytes(),
                addresses: Vec::new(),
                line: head.line,
            });
            return Ok(());
        }
        "जाल" => {
            if args.is_empty() {
                return Err(err("P16", &[&head.text]));
            }

            // Build the web dictionary once (or statically)
            let mut dict = std::collections::HashMap::new();
            for line in WEB_DICTIONARY.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() == 2 {
                    dict.insert(parts[0], parts[1]);
                }
            }

            let mut result = String::new();
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    result.push(' ');
                }
                if let Some(mapped) = dict.get(*arg) {
                    result.push_str(mapped);
                } else {
                    result.push_str(arg);
                }
            }

            out.data.push(Datum {
                section: *section,
                bytes: result.into_bytes(),
                addresses: Vec::new(),
                line: head.line,
            });
            return Ok(());
        }
        "स्थानम्" | "अष्टकाः" | "द्वयष्टकाः" | "चतुरष्टकाः" | "अष्टाष्टकाः" =>
        {
            let width = match head.text.as_str() {
                "अष्टकाः" => 1usize,
                "द्वयष्टकाः" => 2,
                "चतुरष्टकाः" => 4,
                "अष्टाष्टकाः" => 8,
                _ => 0, // स्थानम् counts bytes rather than emitting values
            };
            if args.is_empty() {
                return Err(err("P16", &[&head.text]));
            }
            let mut bytes = Vec::new();
            let mut addresses = Vec::new();

            // A string literal — ADR-0003. Doc 15 §3.1 proposed `꣹ … ꣹`;
            // task A-033 measured it at 0 of 27 faces, so ADR-0003 replaced it
            // with the words `उक्तम् … इति`, "spoken … thus". `इति` is how
            // classical Sanskrit closes a quotation, so the delimiter is the
            // one the language already used.
            //
            // The bytes are the text's UTF-8, and nothing is appended: doc 02
            // §2.5 maps this to `.ascii` rather than `.asciz`, so a program
            // that wants a terminator writes `॥ अष्टकाः ० ॥` and can see it.
            // ADR-0044 D3: `वर्णाष्टकम् … इति` is the same literal — the same
            // close, the same doubling, the same P17/P22/P18 — and its datum
            // is the letters packed ONE OCTET EACH (`devanagari8::pack`). The
            // emitter writes it only where `devanagari8::payload` says it reads
            // back exactly; a body that does not pack (a space between two
            // words, a letter outside the block) is P19 naming it.
            let devanagari8 = args.first() == Some(&crate::devanagari8::OPEN);
            if args.first() == Some(&"उक्तम्") || devanagari8 {
                if width != 1 {
                    return Err(err("P17", &[&head.text]));
                }
                let Some((text, used)) = string_body(&args[1..]) else {
                    return Err(err("P22", &[]));
                };
                // Nothing may follow the close. Dropping it silently would let
                // `उक्तम् क इति ख` assemble as `क`, which is a program that
                // does not say what it does.
                if used != args.len() - 1 {
                    return Err(err("P18", &[args[1 + used]]));
                }
                let bytes = if devanagari8 {
                    match crate::devanagari8::pack(&text) {
                        Ok(b) => b,
                        Err(_) => return Err(err("P19", &[args[1]])),
                    }
                } else {
                    text.into_bytes()
                };
                out.data.push(Datum {
                    section: *section,
                    bytes,
                    addresses: Vec::new(),
                    line: head.line,
                });
                return Ok(());
            }

            for a in &args {
                // A token that begins with a digit or a radix prefix is a
                // numeral; one that begins with a letter is a name, and a name
                // is the address of what it names (ADR-0013). That is the same
                // rule that makes `अर्थ०` one name rather than a word and a
                // number, so nothing new decides it.
                if sanskrit_text::numeral::classify(a).is_none() {
                    if width != 8 {
                        return Err(err("P27", &[&head.text]));
                    }
                    addresses.push((bytes.len(), (*a).to_string()));
                    bytes.extend_from_slice(&0u64.to_le_bytes());
                    continue;
                }
                // Two readers, because these are two different questions
                // (`W-075`). A datum is a BIT PATTERN, so `०षोड्इआऊ२९इउ४८४२२२३२५`
                // is a good `अष्टाष्टकाः` operand and `ऋण१` is all ones. A count
                // is a MAGNITUDE, and `ऋण` means nothing on it — read as a bit
                // pattern, `॥ स्थानम् ऋण५ ॥` would be 0xffff…fb and ask to
                // reserve the address space. One reader for both is how a
                // literal too large became `i64::MAX` and was emitted as one.
                let v = if width == 0 {
                    // स्थानम् n: n zero bytes, unconditionally. `संरेखः` used to
                    // share this line and does not any more (`W-071`) — it asks
                    // for a boundary, which is a different question.
                    match sanskrit_text::numeral::value(a) {
                        Ok(n) => n,
                        Err(NumeralError::Signed) => {
                            return Err(err("P30", &[&head.text, a]));
                        }
                        Err(NumeralError::TooLarge) => return Err(err("P29", &[a])),
                        Err(_) => return Err(err("P19", &[a])),
                    }
                } else {
                    match sanskrit_text::numeral::bits(a) {
                        Ok(b) => b,
                        Err(NumeralError::TooLarge) => return Err(err("P29", &[a])),
                        Err(_) => return Err(err("P19", &[a])),
                    }
                };
                if width == 0 {
                    let n = usize::try_from(v).map_err(|_| err("P29", &[a]))?;
                    bytes.resize(bytes.len() + n, 0);
                } else {
                    // A value must FIT the destination, not be trimmed to it
                    // (`W-082`). `॥ अष्टकाः ३०० ॥` used to emit 0x2c silently —
                    // GNU `as` warns and truncates, but this toolchain has no
                    // warning channel and R-02-1's stance is emit-what-was-
                    // written-or-refuse. So it is `W-075`'s rule with 64
                    // replaced by N: writable iff it fits N bits unsigned, or
                    // with `ऋण` as signed — the two's-complement range of N.
                    //
                    // `bits` has already produced the 64-bit pattern, so the
                    // unsigned test is `v >> n == 0` and the signed one is that
                    // every bit above the sign is set. Width 8 is skipped
                    // because a 64-bit datum holds every u64 by construction —
                    // and because `v >> 64` is not a shift Rust will perform.
                    let n = width * 8;
                    if width < 8 {
                        let fits_unsigned = v >> n == 0;
                        let fits_signed = (v as i64) >> (n - 1) == -1;
                        if !fits_unsigned && !fits_signed {
                            let max = (1u64 << n) - 1;
                            return Err(err(
                                "P31",
                                &[&head.text, &width.to_string(), a, &max.to_string()],
                            ));
                        }
                    }
                    bytes.extend_from_slice(&v.to_le_bytes()[..width]);
                }
            }
            // `ॱपाठ` holds instructions. A data directive written there used to
            // put its bytes in `ॱदत्त` — a section the author never named —
            // while any label around it kept naming the NEXT INSTRUCTION, so
            // two labels either side of it compared equal and reading through
            // one returned an opcode byte (`W-071`, found from the other side
            // by `E-004`).
            //
            // Refused rather than placed, because placing it needs `ॱपाठ` to
            // address bytes instead of instructions, and that is a change to
            // how the section is modelled rather than to this directive. The
            // registry's own rule applies: an assembler that quietly does
            // something else produces a program that is wrong and says nothing.
            if *section == Section::Text {
                return Err(err("P28", &[&head.text]));
            }
            if *section == Section::Bss {
                // Only `स्थानम्` belongs here. A value written into `ॱरिक्त`
                // would be dropped at load, because nothing in the file backs
                // it — so it is refused rather than silently lost.
                if head.text != "स्थानम्" {
                    return Err(err("P20", &[&head.text]));
                }
                if out.bss == 0 {
                    out.bss_line = head.line;
                }
                out.bss += bytes.len();
                return Ok(());
            }
            out.data.push(Datum {
                section: *section,
                bytes,
                addresses,
                line: head.line,
            });
        }
        other => return Err(err("P21", &[other])),
    }
    Ok(())
}

fn parse_statement(toks: &[&Token], fams: &[Family]) -> Result<Instruction, ParseError> {
    let head = toks[0];

    // ADR-0004: position marks the verb. The lexer cannot apply that — a
    // statement ends at the daṇḍa and may span lines — so the first token of
    // the statement is taken as the verb here, and the registry decides whether
    // it is one. Using the registry as the authority is also what R-02-1 asks
    // for: that file is the ISA surface.
    let (name, types) = split_types(&head.text);
    for t in &types {
        if Width::from_type(t).is_none() {
            return Err(ParseError {
                line: head.line,

                aksara: head.aksara,
                code: "P23",
                args: vec![t.to_string()],
                reason: Diagnostic::new("P23", &[t]).render(Language::default()),
            });
        }
    }
    if types.len() > 2 {
        return Err(ParseError {
            line: head.line,

            aksara: head.aksara,
            code: "P08",
            args: vec![head.text.clone(), types.len().to_string()],
            reason: Diagnostic::new("P08", &[&head.text, &types.len().to_string()])
                .render(Language::default()),
        });
    }
    // The first suffix is the width of the result, which for everything but a
    // conversion is the width of the whole instruction.
    let width = types
        .first()
        .map_or(Width::W64, |t| Width::from_type(t).unwrap_or(Width::W64));
    let types: Vec<String> = types.iter().map(|t| (*t).to_string()).collect();

    let Some(family) = fams.iter().find(|f| f.name == name) else {
        // A sigil-bearing first token is almost certainly a statement whose verb
        // was omitted, not a misspelt mnemonic. Saying so names the rule that
        // was broken instead of sending the reader to check their spelling.
        if matches!(head.kind, Kind::Operand { .. }) {
            return Err(ParseError {
                line: head.line,

                aksara: head.aksara,
                code: "P09",
                args: vec![head.text.clone()],
                reason: Diagnostic::new("P09", &[&head.text]).render(Language::default()),
            });
        }
        let hint = nearest(name, fams).map_or_else(String::new, |f| {
            format!(" — did you mean `{}` ({})?", f.name, f.gloss)
        });
        return Err(ParseError {
            line: head.line,

            aksara: head.aksara,
            code: "P03",
            args: vec![name.to_string(), hint.clone()],
            reason: Diagnostic::new("P03", &[name, &hint]).render(Language::default()),
        });
    };

    let mut operands = Vec::new();
    for t in &toks[1..] {
        match &t.kind {
            Kind::Operand { base, karaka } => operands.push(Operand {
                base: base.clone(),
                karaka: *karaka,
                is_numeral: sanskrit_text::is_numeral(base),
            }),
            Kind::Numeral => {
                return Err(ParseError {
                    line: t.line,
                    code: "P04",
                    args: vec![t.text.clone()],
                    aksara: t.aksara,
                    reason: Diagnostic::new("P04", &[&t.text]).render(Language::default()),
                });
            }
            Kind::Word => {
                return Err(ParseError {
                    line: t.line,

                    aksara: t.aksara,
                    code: "P10",
                    args: vec![t.text.clone()],
                    reason: Diagnostic::new("P10", &[&t.text]).render(Language::default()),
                });
            }
            _ => {
                return Err(ParseError {
                    line: t.line,

                    aksara: t.aksara,
                    code: "P11",
                    args: vec![t.text.clone()],
                    reason: Diagnostic::new("P11", &[&t.text]).render(Language::default()),
                });
            }
        }
    }

    // Which roles may repeat is a semantic question, and the grammar answers
    // it the way Sanskrit does.
    //
    // करण is the *instrumental*: "by means of ख and ग". Multiple instruments
    // are natural in the language and required by the ISA — `योगः कम् खन गन ।`
    // is क ← ख + ग, doc 02 §2.2's own first example, and it carries two न.
    // A first version of this check rejected it as a contradiction, which would
    // have made addition unwritable.
    //
    // The other roles are singular by their sense: you write to one place
    // (कर्म), load from one address (अपादान), store to one address (सम्प्रदान).
    // Two of those in one sentence is a genuine contradiction and the sort free
    // operand order makes easy to write by accident.
    for (i, a) in operands.iter().enumerate() {
        let singular = matches!(
            a.karaka,
            Karaka::Destination | Karaka::SourceAddress | Karaka::DestAddress
        );
        if !singular {
            continue;
        }
        if let Some(b) = operands[i + 1..].iter().find(|b| b.karaka == a.karaka) {
            return Err(ParseError {
                line: head.line,
                code: "P05",
                args: vec![a.base.clone(), b.base.clone(), a.karaka.name().into()],
                aksara: head.aksara,
                reason: Diagnostic::new("P05", &[&a.base, &b.base, a.karaka.name()])
                    .render(Language::default()),
            });
        }
    }

    Ok(Instruction {
        family: family.clone(),
        width,
        types,
        operands,
        line: head.line,
    })
}

/// Lex and parse in one step.
///
/// # Errors
/// Lexical errors are returned as strings so a caller can report both stages
/// uniformly; a caller wanting structure should use [`crate::lex::lex`] and
/// [`parse`] directly.
pub fn assemble_program(source: &str) -> Result<Program, Vec<String>> {
    let tokens = crate::lex::lex(source)
        .map_err(|es: Vec<LexError>| es.iter().map(ToString::to_string).collect::<Vec<_>>())?;
    parse(&tokens).map_err(|es| es.iter().map(ToString::to_string).collect())
}

/// Lex and parse, keeping only the instructions.
///
/// # Errors
/// Every lexical and syntactic error found, not only the first.
pub fn assemble_source(source: &str) -> Result<Vec<Instruction>, Vec<String>> {
    assemble_program(source).map(|p| p.instructions)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The codes a source raises, in order.
    ///
    /// `assemble_source` renders every error to a sentence, which is the right
    /// thing for a person and the wrong thing for a test: `B-015` moved the
    /// wording into `spec/diagnostics.tsv` precisely so it could change, and a
    /// test that matches on wording is what that would break. It also matched
    /// on the ENGLISH wording, which stopped being what the parser says the
    /// moment the default language became Sanskrit.
    fn codes(source: &str) -> Vec<&'static str> {
        let tokens = crate::lex::lex(source).expect("lexes");
        parse(&tokens)
            .expect_err("must reject")
            .iter()
            .map(|e| e.code)
            .collect()
    }

    #[test]
    fn the_registry_compiles_in_and_has_every_extension() {
        let f = families();
        // 55: `B-043` split `loadu` out of `load`, and `B-056` split five more
        // unsigned siblings out by the same rule — अचिह्नभागः, अचिह्नशेषः,
        // अचिह्नन्यूनम् and the two atomic extremes. `B-059` then split the six
        // branch conditions out of `यदिलङ्घनम्`, which had made every
        // conditional branch a `beq`. R-02-1 says every
        // architectural instruction has a name, and lbu/lhu/lwu had none —
        // one family name cannot cover both sign behaviours, exactly as
        // दक्षिणसरणम् and सचिह्नदक्षिणसरणम् are already two families.
        // 73: `C-001c1` added `sfence`, the first PRIVILEGED family here.
        // Everything above it is usable in any mode; that one is the kernel's,
        // and paging cannot be turned on without it.
        // 74: `C-001d2` added `sret`. A handler that cannot return can only shut
        // the machine down, which is why `C-001d1` had to.
        // 75: `V-008` part 2 added `vsetvli` (`व्यूहदैर्घ्यम्`), the one vector
        // family with no scalar twin; the other six vector forms joined the
        // families they share a word with (owner ruling, option A).
        assert_eq!(f.len(), 75, "registry should hold 75 families");
        for ext in ["I", "M", "A", "FD", "Zicsr", "Zifencei", "V"] {
            assert!(f.iter().any(|x| x.ext == ext), "no family for {ext}");
        }
    }

    #[test]
    fn the_worked_example_parses_and_roles_resolve() {
        let is = assemble_source("योगः कम् खन गन ।").expect("parses");
        assert_eq!(is.len(), 1);
        let i = &is[0];
        assert_eq!(i.family.key, "add");
        assert_eq!(i.width, Width::W64, "default width is ॱअ६४");
        assert_eq!(i.by_role(Karaka::Destination).unwrap().base, "क");
        assert_eq!(i.operands.len(), 3);
    }

    #[test]
    fn operand_order_really_is_free() {
        // The claim D-02-C makes. If this fails, the kāraka design bought
        // nothing and a positional convention would be simpler.
        let a = assemble_source("योगः कम् खन गन ।").expect("parses");
        let b = assemble_source("योगः खन गन कम् ।").expect("parses");
        assert_eq!(
            a[0].by_role(Karaka::Destination),
            b[0].by_role(Karaka::Destination)
        );
        assert_eq!(a[0].family, b[0].family);
    }

    #[test]
    fn load_and_store_resolve_to_different_roles() {
        let load = assemble_source("आहारः कम् खत् ।").expect("parses");
        let store = assemble_source("निधानम् खय् कन ।").expect("parses");
        assert!(load[0].by_role(Karaka::SourceAddress).is_some());
        assert!(load[0].by_role(Karaka::DestAddress).is_none());
        assert!(store[0].by_role(Karaka::DestAddress).is_some());
    }

    #[test]
    fn an_unknown_mnemonic_is_an_error_with_a_suggestion() {
        // R-02-1: no passthrough, no raw-encoding escape. And a bare "unknown
        // mnemonic" in a script the reader may be learning leaves nowhere to go.
        assert_eq!(codes("योग कम् खन ।"), ["P03"]);
        let e = assemble_source("योग कम् खन ।").expect_err("must reject");
        assert!(
            e[0].contains("योगः"),
            "should suggest the near miss: {}",
            e[0]
        );
    }

    #[test]
    fn an_unrelated_name_gets_no_suggestion() {
        // A fixed distance threshold offered व्यत्ययः (bitwise not) for
        // क्षत्रियः — three akṣaras each, two edits apart, sharing nothing.
        // A suggestion that is not close invites a wrong edit that fails
        // somewhere else.
        // भाषा was in this list and should not have been: it is exactly one
        // akṣara from भागः (divide), so suggesting it is correct behaviour.
        // The fixture was wrong, not the heuristic — bending the algorithm to
        // satisfy a badly chosen example would have made it worse at its job.
        for bad in ["क्षत्रियः", "गणकयन्त्रम्", "संस्कृतम्"]
        {
            let e = assemble_source(&format!("{bad} कम् खन ।")).expect_err("must reject");
            assert!(!e[0].contains("did you mean"), "{bad}: {}", e[0]);
        }
    }

    #[test]
    fn a_near_miss_still_gets_its_suggestion() {
        // The threshold must not be so tight that it stops helping.
        let e = assemble_source("योग कम् खन ।").expect_err("must reject");
        assert!(e[0].contains("योगः"), "{}", e[0]);
    }

    #[test]
    fn a_statement_without_a_verb_says_so() {
        // ADR-0004's named failure mode. Before that decision this would have
        // reported a missing mnemonic; it now names the rule.
        assert_eq!(codes("कम् खन गन ।"), ["P09"]);
        // And the rule it names travels in both languages, because a reader
        // sent to ADR-0004 is the whole value of this diagnostic.
        for lang in [Language::Sanskrit, Language::English] {
            let m = Diagnostic::new("P09", &["कम्"]).render(lang);
            assert!(m.contains("ADR-0004"), "{m}");
        }
    }

    #[test]
    fn an_operand_without_a_sigil_is_rejected() {
        assert_eq!(codes("योगः कम् गज ।"), ["P10"]);
    }

    #[test]
    fn two_destinations_are_a_contradiction() {
        // Free order makes this easy to write by accident.
        assert_eq!(codes("योगः कम् खम् गन ।"), ["P05"]);
        // The role is named in Sanskrit. `{:?}` on the enum printed
        // `Destination`, an English word inside a Sanskrit sentence.
        let e = assemble_source("योगः कम् खम् गन ।").expect_err("must reject");
        assert!(e[0].contains("कर्म"), "{}", e[0]);
    }

    #[test]
    fn two_sources_are_normal_because_करण_is_the_instrumental() {
        // "by means of ख and ग". Doc 02 §2.2's own first example carries two न,
        // and a first version of the uniqueness check rejected it — which would
        // have made addition unwritable.
        let is = assemble_source("योगः कम् खन गन ।").expect("two sources are legal");
        assert_eq!(
            is[0]
                .operands
                .iter()
                .filter(|o| o.karaka == Karaka::Source)
                .count(),
            2
        );
    }

    #[test]
    fn a_missing_danda_is_reported() {
        let e = assemble_source("योगः कम् खन गन").expect_err("must reject");
        // The statement at fault, not the English word for the mark: `B-078`
        // put this message in a table and the parser speaks Sanskrit.
        assert!(e[0].contains("योगः"), "{}", e[0]);
    }

    #[test]
    fn the_width_suffix_selects_among_the_encodings_one_family_covers() {
        // R-02-2: one name covers add/addi/addw/addiw, and this is how the
        // assembler is told which.
        for (src, want) in [
            ("योगः कम् खन ।", Width::W64),
            ("योगःॱअ३२ कम् खन ।", Width::W32),
            ("योगःॱअ८ कम् खन ।", Width::W8),
        ] {
            assert_eq!(assemble_source(src).expect("parses")[0].width, want);
        }
        let e = assemble_source("योगःॱअ७ कम् खन ।").expect_err("bad width");
        assert!(e[0].contains("व्याप्ति"), "{}", e[0]);
    }

    #[test]
    fn a_conversion_writes_two_types_and_everything_else_writes_one() {
        // `B-075`. `प्लवरूपान्तरम्ॱअ३२ॱप६४` is "convert, writing a signed 32-bit
        // integer, reading a 64-bit float" — the pair doc 02 §2.5 already had
        // names for, which is why no new vocabulary was coined.
        let i = &assemble_source("प्लवरूपान्तरम्ॱअ३२ॱप६४ कम् खन ।").expect("parses")[0];
        assert_eq!(i.types, vec!["अ३२", "प६४"]);
        assert_eq!(i.width, Width::W32, "the width is the RESULT's");

        // One suffix is the ordinary case and leaves the list one long.
        assert_eq!(
            assemble_source("योगःॱअ३२ कम् खन ।").expect("parses")[0].types,
            vec!["अ३२"]
        );
        assert!(
            assemble_source("योगः कम् खन ।").expect("parses")[0]
                .types
                .is_empty()
        );

        // Nothing reads three types.
        assert_eq!(codes("प्लवरूपान्तरम्ॱअ३२ॱप६४ॱप३२ कम् खन ।"), ["P08"]);
    }

    #[test]
    fn the_unsigned_and_float_kinds_are_writable_type_names() {
        // Doc 02 §2.5 fixed three classes and the parser had only ever
        // accepted one, because until `B-075` one width was all a suffix said.
        for src in ["योगःॱन३२ कम् खन ।", "योगःॱप६४ कम् खन ।", "योगःॱ३२ कम् खन ।"]
        {
            assemble_source(src).unwrap_or_else(|e| panic!("{src}: {e:?}"));
        }
        assert_eq!(codes("योगःॱश३२ कम् खन ।"), ["P23"]);
    }

    #[test]
    fn a_statement_may_span_lines() {
        // Statements end at the daṇḍa, not at the newline.
        let is = assemble_source("योगः कम्\n  खन गन ।").expect("parses");
        assert_eq!(is.len(), 1);
        assert_eq!(is[0].operands.len(), 3);
    }

    #[test]
    fn numerals_carry_roles_too() {
        let is = assemble_source("आहारः कम् ०षोड्८त् ।").expect("parses");
        let a = is[0]
            .by_role(Karaka::SourceAddress)
            .expect("address operand");
        assert!(a.is_numeral, "०षोड्८ should be recognised as a numeral");
    }

    #[test]
    fn distance_counts_aksaras_not_code_points() {
        // क्ष is one akṣara. A code-point distance would call these three apart
        // and suppress a suggestion that should be offered.
        assert_eq!(distance("क्ष", "क"), 1);
        assert_eq!(distance("योगः", "योगः"), 0);
    }
}
