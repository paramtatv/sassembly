//! **दृश्यम्** — the T2 UI sub-language, doc 07 §4.2, task `F-004f`.
//!
//! Decision D-07-B (doc 07 §4.1, `research/07-…:76`) reads *"Sassembly-web is
//! T1 with a declarative UI sub-language — structure as expressions, style as
//! typed values, and a pure `स्थिति → दृश्यम्` render function. No DOM, no
//! mutation API, no CSS cascade."* This module is the **sub-language** half of
//! that sentence and nothing else: it reads the seven words doc 07 §4.2's
//! frozen counter sketch writes — `दृश्यम्`, `स्तम्भः`, `पङ्क्तिः`, `पाठः`,
//! `कुञ्जिका`, `सन्ततिः`, `विन्यासः` — out of a T1 token stream and evaluates
//! them, for one state, into a [`Rupa`].
//!
//! It is not a compiler. It does not lower and emits no code, and the one
//! thing it checks is stated in its own section below: a `कुञ्जिका`'s event
//! is a variant of the module's `घटना`, or the view is refused (`F-004f5`).
//! `super::parse` already reads `वृत्तिः`, `प्रकारः` and `संरचना`, so the
//! sketch's state type and its functions were already within reach; what no
//! file under `crates/` read was this vocabulary, and that is the gap `F-004f`
//! names.
//!
//! # The sketch cannot be transcribed character for character, and that is
//! ratified rather than a liberty taken here
//!
//! Doc 07 §4.2 is written with `꣺ ꣻ` for blocks, `꣼ ꣸` for grouping,
//! `ᳵ ᳶ` for indexing and `꣹` for strings. Task A-033 measured all seven of
//! those signs at **0 of 27 Devanagari faces**, and ADR-0003 replaced each with
//! a word — a table `spec/grammar-t1.ebnf` reprints and freezes:
//!
//! ```text
//! group   ꣼ ꣸   ->  आरभ्य  … समाप्तम्
//! block   ꣺ ꣻ   ->  आदि    … इति
//! index   ᳵ ᳶ   ->  अङ्कः   … अन्तः
//! string  ꣹ ꣹   ->  उक्तम्  … इति
//! ```
//!
//! So the grammar below is doc 07 §4.2's sketch in the spelling
//! `spec/grammar-t1.ebnf` froze, not a second design. `ऽ`, `ॱ`, `ॱॱ`, `।` and
//! `॥` are already the signs the sketch uses and are carried over unchanged.
//!
//! # The grammar, as read
//!
//! ```text
//! view      = "वृत्तिः" , "दृश्यम्" , "आरभ्य" , ident , "ॱॱ" , ident ,
//!             "समाप्तम्" , "फलम्" , "रूपम्" , "आदि" ,
//!             "प्रत्यागमनम्" , rupa , "।" , "इति" , "॥" ;
//! rupa      = column | row | text | button ;
//! column    = "स्तम्भः" , "आदि" , [ vinyasa , "ऽ" ] , santati , "इति" ;
//! row       = "पङ्क्तिः" , "आदि" , [ vinyasa , "ऽ" ] , santati , "इति" ;
//! vinyasa   = "विन्यासः" , "भवति" , "आदि" ,
//!             "अन्तरम्" , "भवति" , signed_numeral , "ऽ" ,
//!             "संरेखः" , "भवति" , samrekha , "इति" ;
//! samrekha  = "आदि" | "मध्यम्" | "अन्त" ;
//! santati   = "सन्ततिः" , "भवति" , "अङ्कः" , [ rupa , { "ऽ" , rupa } ] , "अन्तः" ;
//! text      = "पाठः" , "आरभ्य" , text_expr , "समाप्तम्" ;
//! button    = "कुञ्जिका" , "आरभ्य" , string , "ऽ" , event , "समाप्तम्" ;
//! event     = ident | "घटना" , "ॱ" , ident ;   (* a variant of ganana, F-004f5 *)
//! text_expr = text_atom , { "अधि" , text_atom } ;
//! text_atom = string | layout | "अङ्कपाठः" , "आरभ्य" , path , "समाप्तम्" ;
//! string    = "उक्तम्" , { word } , "इति" ;
//! layout    = "यतिः" | "विवरम्" ;   (* ADR-0018: LINE FEED, SPACE *)
//! path      = ident , "ॱ" , ident ;
//!
//! ganana    = "प्रकारः" , "घटना" , "भवति" , "गणना" , "आदि" ,
//!             [ ident , { "ऽ" , ident } , [ "ऽ" ] ] , "इति" , "॥" ;
//! sanrachana = "प्रकारः" , "स्थितिः" , "भवति" , "संरचना" , "आदि" ,
//!             [ anga , { "ऽ" , anga } , [ "ऽ" ] ] , "इति" , "॥" ;
//! anga      = ident , "ॱॱ" , type_words ;   (* the type is collected, not parsed *)
//! ```
//!
//! `ganana` and `sanrachana` are the only declarations outside
//! `वृत्तिः दृश्यम्` this module reads, and it reads them because the view
//! refers to both — `सॱसङ्ख्या` is a member of the one and `वर्धनम्` a variant
//! of the other. `sanrachana` arrived with `F-004f` after `ganana` had been
//! read since `F-004f5`, and the asymmetry between them was the gap: the event
//! was typed by the source and the state was typed by whatever the host put in
//! the map. See [`Reader::state_field`].
//!
//! On `ganana`: doc 07 §4.2's
//! `प्रकारः घटना भवति गणना ꣺ वर्धनम् ऽ ह्रासः ꣻ ॥` is the type of every
//! `कुञ्जिका`'s event. `spec/grammar-t1.ebnf` says `enum_body` is NOT frozen,
//! so the shape above is the corpus's — `गणकः.सस` and
//! `tests/corpus/t1/अवस्थायन्त्रम्.सस` both write `आदि v ऽ v ऽ इति ॥` with a
//! trailing separator — read permissively at the one point the corpus leaves
//! open (the last `ऽ`), and nowhere else.
//!
//! # `घटना` — the event is typed, and what typed it is not a type checker
//!
//! `F-004f5` said resolving `वर्धनम्` against `प्रकारः घटना भवति गणना`
//! *"needs the T1 type checker"*. MEASURED 2026-09-03, it does not, and the
//! checker it named could not have done it: `अर्थ` is
//! `crates/sadhana-t1/src/artha.t1`, T1 SOURCE that nothing in the tree
//! executes — `D-002j`, the T1 interpreter, is `todo` — and the Rust checker
//! at `t1/typecheck.rs` has no enum knowledge because `t1/ast.rs:58`'s
//! `TypeDecl { name, is_struct }` carries no variants and `t1/parse.rs:127`
//! reads `संरचना` only, dropping a `गणना`'s body token by token in its
//! fallback arm. What resolving an event actually needs is the LIST OF NAMES
//! the `गणना` declares and the position of one name in it, and a reader that
//! already finds `वृत्तिः दृश्यम्` in a token stream can find `प्रकारः घटना`
//! the same way. So [`Ghatana`] is the variant's name and its place in the
//! declaration, [`render`] reads the declaration before the view, and a
//! `कुञ्जिका` whose event is not in it — or a module with buttons and no
//! `घटना` at all — is refused rather than carried as text.
//!
//! **Which `गणना` is the event type is decided by NAME, as the state type
//! already is.** `Reader::view` requires the parameter to be a `स्थितिः`;
//! this requires the event to be a `घटना`. Both are doc 07 §4.2's words for
//! the two halves of D-07-B's `स्थिति × घटना → स्थिति`, and `परिवर्तनम्`'s
//! second parameter is `घॱॱ घटना`. A `गणना` of another name with the same
//! variants is another type, and `a_ganana_of_another_name_does_not_type_events`
//! pins that. When a T1 front end resolves `परिवर्तनम्`'s signature the name
//! can come from there instead; nothing here prevents it.
//!
//! # `संरेखः भवति आदि` — the one collision, and why position settles it
//!
//! `आदि` is both the block-open word and the name of the alignment
//! [`Samrekha::Adi`]. That is ADR-0003's word for *"beginning"* used in both of
//! its senses, and it is not ambiguous **here** because an alignment can only
//! stand immediately after `संरेखः भवति`, where no block may open. This is the
//! same rule ADR-0004 already applies to the `म्` sigil in T0 — position
//! disambiguates, and only position. `the_alignment_named_adi_is_not_a_block`
//! pins it.
//!
//! # Two things this module decides out loud, because the grammar is silent
//!
//! **A space between two text pieces.** This used to be a *convention*, then
//! ADR-0017 made half of it a fact, and ADR-0018 has deleted the other half.
//! The frozen `token` production listed no string, so *"a T1 front end (B-080)
//! cannot lex a string literal"* and `उक्तम्` lexed as an ordinary word — which
//! meant whitespace inside `उक्तम् … इति` never reached a parser at all, because
//! the lexer had already split on it. Both readers papered over that by joining
//! a literal's words with one space, and `text_expr` joined the pieces of an
//! `अधि` chain the same way, for the same reason.
//!
//! **Every space in a `पाठः` is now written down.** The literal's own spacing is
//! the source's, kept by the lexer and carried in [`crate::lex::Kind::Str`]; the
//! space *between* two pieces is `विवरम्`, ADR-0018's value for U+0020; and
//! `अधि` joins with nothing. That is doc 07 §4.2's own design restored rather
//! than a new one — the sketch puts its space **in the text**, as
//! `उक्तम् गणना॰ॱ इति अधि अङ्कपाठः …`, and never in the operator. The one-space
//! join existed because `॰ॱ` did not decode and a literal cannot end in a space.
//! `crates/sadhana/tests/गणकः.सस` now writes
//! `उक्तम् गणना इति अधि विवरम् अधि अङ्कपाठः …` and still renders `गणना 0`: the
//! output is unchanged and the reason for it is not.
//!
//! **A `पाठः` may now be more than one line**, because `यतिः` is the other half
//! of ADR-0018. Nothing in doc 07 §4.2 wants one; the program that does is
//! `crates/sadhana/src/t1/emit.rs`'s `emit_program`, and
//! `crates/sadhana/tests/t1_layout_words.rs` is where that is proved.
//!
//! `॰ॱ` itself still **lexes and is still not decoded** — `॰` inside a string is
//! text, so it no longer eats the rest of the line, but neither ADR-0017 nor
//! ADR-0018 revives `॰` as an escape lead-in, which ADR-0011 refused. What
//! ADR-0018 settles is the *respelling*, which is what the open question
//! actually was: it needed a word for *space*, and there is one now.
//!
//! **Which digits `अङ्कपाठः` writes: DEVANAGARI, settled by ADR-0023 (`F-004f3`).**
//! Doc 07 §4.2 does not say, and this module used to write ASCII — not because
//! the language chose, but because `F-004a`'s fixture was `format!("गणना {}", …)`
//! and equality with that fixture was the acceptance. **The fixture was choosing
//! the language's numerals**, which is the wrong way round: every other numeral
//! here is Devanagari, including this module's own `१२`. `sanskrit-text` could
//! already READ `१२` and could not WRITE it, so there was nothing to call; the
//! writer is now `numeral::to_devanagari_signed`, negatives take the language's
//! own `ऋण` marker rather than `-`, and the four ASCII fixtures moved to match.

use crate::lex::{Kind, Token};

/// `ॱ` U+0971, the member mark. ADR-0003 ratified it; `spec/grammar-t1.ebnf`
/// lists it as `member_mark`.
const MEMBER_MARK: char = 'ॱ';

/// `घटना` — the name of the `गणना` that types a `कुञ्जिका`'s event, doc 07
/// §4.2. Fixed by name for the same reason `स्थितिः` is in `Reader::view`;
/// see the module docs.
const GHATANA: &str = "घटना";

/// `स्थितिः` — the name of the `संरचना` that types the state a view reads,
/// doc 07 §4.2. Fixed by name for the same reason [`GHATANA`] is, and with
/// less room for doubt: [`Reader::view`] already requires the view's parameter
/// to be declared `ॱॱ स्थितिः`, so the declaration an `अङ्कपाठः` resolves
/// against is the one wearing that name and there is no second candidate.
const STHITI: &str = "स्थितिः";

/// `अ३२` — the one member type an `अङ्कपाठः` may read.
///
/// Doc 07 §4.2 declares `सङ्ख्याॱॱ अ३२` and [`Sthiti`] holds `i32`, so this is
/// not a restriction this module adds: it is the whole of what the state a
/// `दृश्यम्` is rendered against can carry. A member of any other type is a
/// member `अङ्कपाठः` has no value for, and saying so is the point of reading
/// the declaration at all.
const A32: &str = "अ३२";

/// `संरेखः` — how a [`Vinyasa`] aligns children on the cross axis.
///
/// The three values are `crates/renderer/src/vastu/rupa.rs`'s, named the same
/// way and for the same reason: doc 07 §4.2 writes only `मध्यम्`, and an
/// alignment with a centre and no ends cannot express a column at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Samrekha {
    /// `आदि` — flush to the beginning of the cross axis. The default, because
    /// it is the arrangement that invents no measurement.
    #[default]
    Adi,
    /// `मध्यम्` — centred; the sketch's own value.
    Madhyama,
    /// `अन्त` — flush to the end of the cross axis.
    Anta,
}

impl Samrekha {
    /// The word that names this alignment in the sub-language.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Adi => "आदि",
            Self::Madhyama => "मध्यम्",
            Self::Anta => "अन्त",
        }
    }

    /// The alignment a word names, if it names one.
    #[must_use]
    fn from_word(word: &str) -> Option<Self> {
        [Self::Adi, Self::Madhyama, Self::Anta]
            .into_iter()
            .find(|s| s.word() == word)
    }
}

/// `विन्यासः` — the style record: doc 07 §4.2's *"plain record value —
/// composable, type-checked, no cascade, no specificity"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Vinyasa {
    antara: i32,
    samrekha: Samrekha,
}

impl Vinyasa {
    /// An arrangement with gap `antara` and cross-axis alignment `samrekha`.
    ///
    /// Returns [`None`] if `antara` is negative. A negative gap would place a
    /// child before the one preceding it, and there is no z-order in a
    /// Sassembly-web view to resolve the overlap with.
    #[must_use]
    pub const fn new(antara: i32, samrekha: Samrekha) -> Option<Self> {
        if antara < 0 {
            return None;
        }
        Some(Self { antara, samrekha })
    }

    /// `अन्तरम्` — the gap between adjacent children. Never negative.
    #[must_use]
    pub const fn antara(self) -> i32 {
        self.antara
    }

    /// `संरेखः` — how the children sit on the cross axis.
    #[must_use]
    pub const fn samrekha(self) -> Samrekha {
        self.samrekha
    }
}

/// **घटना** — a typed event: one variant of the module's
/// `प्रकारः घटना भवति गणना`, resolved (`F-004f5`).
///
/// This is what `crates/renderer/src/vastu/rupa.rs`'s `Rupa<E>` leaves
/// generic — *"a value the application named rather than a string an engine
/// would have to interpret"* — made concrete for a view read from a file: the
/// application named it in a `गणना`, and this is that name together with its
/// place in the declaration. The place is what `Ty::Enum`'s discriminant is
/// one level down (`t1/types.rs`), and it is what a router can match on
/// without comparing strings. Only [`render`] constructs one, and it does so
/// only after finding the name in the declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ghatana {
    /// The variant's name, as the `गणना` declares it — `वर्धनम्`, not
    /// `घटनाॱवर्धनम्`, whichever way the view wrote it.
    pub nama: String,
    /// The variant's place in the `गणना`, counted from `0` in declaration
    /// order.
    pub krama: usize,
}

/// **रूपम्** — a view, as doc 07 §4.2's sketch writes one.
///
/// The variants and fields are `crates/renderer/src/vastu/rupa.rs`'s, because
/// both are naming the same seven words of one language. Where that type is
/// generic in its event, this one carries a [`Ghatana`]: the view came out of
/// a module, so the module's own `गणना` is what the event is a value of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rupa {
    /// `पाठः` — text. A leaf.
    Patha(String),
    /// `स्तम्भः` — a column: children stacked one below the next.
    Stambha {
        /// How the children are arranged.
        vinyasa: Vinyasa,
        /// `सन्ततिः` — the children, in order.
        santati: Vec<Rupa>,
    },
    /// `पङ्क्तिः` — a row: children placed one after the next.
    Pankti {
        /// How the children are arranged.
        vinyasa: Vinyasa,
        /// `सन्ततिः` — the children, in order.
        santati: Vec<Rupa>,
    },
    /// `कुञ्जिका` — a button: a label and the event pressing it emits. A leaf.
    Kunjika {
        /// The label drawn on the button.
        patha: String,
        /// `घटना` — the event this button emits, resolved against the
        /// module's `गणना` of that name.
        ghatana: Ghatana,
    },
}

impl Rupa {
    /// This node's children, in order. Empty for the two leaf forms.
    #[must_use]
    pub fn children(&self) -> &[Self] {
        match self {
            Self::Stambha { santati, .. } | Self::Pankti { santati, .. } => santati,
            Self::Patha(_) | Self::Kunjika { .. } => &[],
        }
    }

    /// The arrangement this node applies **to its own children**, or [`None`]
    /// for a leaf. This is the whole of the style lookup: there is no second
    /// step that consults an ancestor, which is D-07-B's *"no CSS cascade"*.
    #[must_use]
    pub const fn arrangement(&self) -> Option<Vinyasa> {
        match self {
            Self::Stambha { vinyasa, .. } | Self::Pankti { vinyasa, .. } => Some(*vinyasa),
            Self::Patha(_) | Self::Kunjika { .. } => None,
        }
    }

    /// Every node of the view in document order, self first.
    ///
    /// The walk holds an explicit stack, so walking costs heap and not frames.
    #[must_use]
    pub fn pre_order(&self) -> Vec<&Self> {
        let mut out = Vec::new();
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            out.push(node);
            stack.extend(node.children().iter().rev());
        }
        out
    }

    /// The number of nodes in the view, this one included.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.pre_order().len()
    }

    /// The number of nodes on the longest root-to-leaf path; `1` for a leaf.
    #[must_use]
    pub fn depth(&self) -> usize {
        let mut deepest = 0;
        let mut stack = vec![(self, 1usize)];
        while let Some((node, depth)) = stack.pop() {
            deepest = deepest.max(depth);
            stack.extend(node.children().iter().map(|c| (c, depth + 1)));
        }
        deepest
    }

    /// Every string the view puts on the surface, in document order: each
    /// `पाठः`'s text and each `कुञ्जिका`'s label. A label is drawn, so it is
    /// counted.
    #[must_use]
    pub fn text(&self) -> Vec<&str> {
        self.pre_order()
            .into_iter()
            .filter_map(|node| match node {
                Self::Patha(t) | Self::Kunjika { patha: t, .. } => Some(t.as_str()),
                Self::Stambha { .. } | Self::Pankti { .. } => None,
            })
            .collect()
    }

    /// Every event the view can emit, by name, in document order.
    #[must_use]
    pub fn events(&self) -> Vec<&str> {
        self.pre_order()
            .into_iter()
            .filter_map(|node| match node {
                Self::Kunjika { ghatana, .. } => Some(ghatana.nama.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// `स्थितिः` — the state a `दृश्यम्` is rendered against.
///
/// Doc 07 §4.2's state is `संरचना ꣺ सङ्ख्याॱॱ अ३२ ऽ ꣻ`, one `अ३२` field, and
/// this holds exactly that shape: named fields with `i32` values. A richer
/// state is a T1 type-system question and not a sub-language one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sthiti {
    fields: Vec<(String, i32)>,
}

impl Sthiti {
    /// A state with no fields.
    #[must_use]
    pub const fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Bind `name` to `value`, replacing any previous binding.
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: i32) -> Self {
        let name = name.into();
        self.fields.retain(|(n, _)| *n != name);
        self.fields.push((name, value));
        self
    }

    /// The value bound to `name`, if any.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<i32> {
        self.fields.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }
}

/// Why reading a `दृश्यम्` stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrishyaError {
    /// 1-based line of the token the reader stopped at; `0` if it ran out.
    pub line: usize,
    /// Akṣara index of that token.
    pub aksara: usize,
    /// What was wrong, in the language.
    pub reason: String,
}

impl core::fmt::Display for DrishyaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "पङ्क्ति {} अक्षर {}: {}",
            self.line, self.aksara, self.reason
        )
    }
}

impl core::error::Error for DrishyaError {}

/// Read the `वृत्तिः दृश्यम्` of a T1 token stream and render it for `sthiti`.
///
/// Everything before `वृत्तिः दृश्यम्` is skipped unread — the module header
/// and `परिवर्तनम्` are T1's and belong to the rest of the front end — with
/// TWO exceptions, which are the two types the view NAMES:
///
/// - `प्रकारः घटना भवति गणना`, the type of every `कुञ्जिका`'s event
///   (`F-004f5`).
/// - `प्रकारः स्थितिः भवति संरचना`, the type of the state every `अङ्कपाठः`
///   reads a member of (`F-004f`).
///
/// A view refers to both, so a reader of the view has to know what they say.
/// This doc comment used to list the state type among the things skipped
/// unread, and it was right: `state_field` resolved a member against the
/// [`Sthiti`] the CALLER passed and never against the module's declaration, so
/// a view naming a member the module does not declare rendered whenever the
/// caller's map happened to carry it, and a view naming one it does declare was
/// refused in the same words when the caller's map did not. The runtime map is
/// the host's half; the declaration is the source's.
///
/// # Errors
///
/// [`DrishyaError`] if there is no `वृत्तिः दृश्यम्`, if its shape or either
/// declaration's does not match the grammar in the module docs, if a
/// `कुञ्जिका` emits an event the `घटना` does not declare, or if an `अङ्कपाठः`
/// reads a member the `स्थितिः` does not declare, declares at a type other
/// than `अ३२`, or the caller did not bind — including every `कुञ्जिका` of a
/// module that declares no `घटना` and every `अङ्कपाठः` of one that declares no
/// `स्थितिः`.
pub fn render(tokens: &[Token], sthiti: &Sthiti) -> Result<Rupa, DrishyaError> {
    let anga = read_sthiti(tokens, sthiti)?;
    let ghatana = read_ghatana(tokens, sthiti)?;
    let start = find_declaration(tokens, "वृत्तिः", "दृश्यम्").ok_or_else(|| DrishyaError {
        line: 0,
        aksara: 0,
        reason: "इदम् मण्डलम् वृत्तिम् दृश्यम् न वहति".into(),
    })?;
    let mut r = Reader {
        tokens,
        pos: start,
        sthiti,
        ghatana: ghatana.as_deref(),
        anga: anga.as_deref(),
    };
    r.view()
}

/// One member of `प्रकारः स्थितिः भवति संरचना` — its name and the words of its
/// declared type.
///
/// **The type is kept as the words it was written with and is NOT parsed.** The
/// T1 corpus declares members whose types run several tokens —
/// `अग्रिमःॱॱ सम्भाव्य स्थानम् गण्डिका` in `tests/corpus/t1/शृङ्खला.सस`, a whole
/// function type in `tests/corpus/t1/यन्त्रम्.सस` — and
/// `spec/grammar-t1.ebnf:1597` says `struct_body` is NOT frozen. This module
/// is not a type checker and `F-004f5` already measured that it does not need
/// to be one: it needs ONE fact about a member, whether it is [`A32`], and a
/// joined string answers that fact without claiming to answer any other.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Anga {
    /// The member's name, as `सॱसङ्ख्या`'s field half must spell it.
    nama: String,
    /// The member's declared type, as the words between its `ॱॱ` and the `ऽ`
    /// or `इति` that ends it, joined with one space.
    prakara: String,
}

/// The members of the module's `प्रकारः स्थितिः भवति संरचना`, in declaration
/// order, or [`None`] if the module declares no `स्थितिः`.
///
/// Read eagerly, for [`read_ghatana`]'s reason exactly: the declaration is the
/// module's, and a module whose state type does not read is not one whose view
/// should, whether or not that view happens to name a member of it.
fn read_sthiti(tokens: &[Token], sthiti: &Sthiti) -> Result<Option<Vec<Anga>>, DrishyaError> {
    let Some(start) = find_declaration(tokens, "प्रकारः", STHITI) else {
        return Ok(None);
    };
    let mut r = Reader {
        tokens,
        pos: start,
        sthiti,
        ghatana: None,
        anga: None,
    };
    r.sanrachana().map(Some)
}

/// The variants of the module's `प्रकारः घटना भवति गणना`, in declaration
/// order, or [`None`] if the module declares no `घटना`.
///
/// Read eagerly rather than at the first `कुञ्जिका`: the declaration is the
/// module's, and a module whose event type does not read is not one whose
/// view should, whether or not that view happens to press anything.
fn read_ghatana(tokens: &[Token], sthiti: &Sthiti) -> Result<Option<Vec<String>>, DrishyaError> {
    let Some(start) = find_declaration(tokens, "प्रकारः", GHATANA) else {
        return Ok(None);
    };
    let mut r = Reader {
        tokens,
        pos: start,
        sthiti,
        ghatana: None,
        anga: None,
    };
    r.ganana().map(Some)
}

/// The index of the `keyword` that declares `name`, if the stream has one.
///
/// Both words are matched, not the keyword alone: a mutation that took the
/// first `वृत्तिः` whatever its name survived the unit tests once, and the
/// same shape would take `प्रकारः स्थितिः` for `प्रकारः घटना`.
fn find_declaration(tokens: &[Token], keyword: &str, name: &str) -> Option<usize> {
    tokens
        .windows(2)
        .position(|w| w[0].text == keyword && w[1].text == name)
}

/// The cursor. It holds the state because `अङ्कपाठः` is evaluated as it is
/// read: doc 07 §4.1's render function is `स्थिति → दृश्यम्`, a function of a
/// state, and building an AST first and folding the state in afterwards would
/// be a second representation with nothing to say. It holds the `घटना`'s
/// variants for the same reason: a `कुञ्जिका`'s event is resolved as it is
/// read.
struct Reader<'a> {
    tokens: &'a [Token],
    pos: usize,
    sthiti: &'a Sthiti,
    /// The module's `घटना` variants in declaration order; [`None`] if the
    /// module declares no `घटना`, in which case every `कुञ्जिका` is refused.
    ghatana: Option<&'a [String]>,
    /// The module's `स्थितिः` members in declaration order; [`None`] if the
    /// module declares no `स्थितिः`, in which case every `अङ्कपाठः` is
    /// refused. The same shape as `ghatana` and for the same reason: a view
    /// names two types, and a reader of the view resolves both as it reads.
    anga: Option<&'a [Anga]>,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    /// `W-274`: UNUSED TODAY, and kept rather than deleted because it is the
    /// other half of `peek`/`advance` — a reader that can look but not step is
    /// an incomplete cursor, and the next arm to need one would rewrite this
    /// line for line. Named here instead of hidden by the directory-wide allow
    /// that used to cover it; if it is still unused when this reader grows its
    /// next caller, delete it rather than renewing the allow.
    #[allow(dead_code)]
    fn advance(&mut self) -> Option<&'a Token> {
        let t = self.peek();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Where the reader is, for a diagnostic.
    fn here(&self) -> (usize, usize) {
        self.peek().map_or((0, 0), |t| (t.line, t.aksara))
    }

    fn err<T>(&self, reason: impl Into<String>) -> Result<T, DrishyaError> {
        let (line, aksara) = self.here();
        Err(DrishyaError {
            line,
            aksara,
            reason: reason.into(),
        })
    }

    /// Consume the next token if its text is exactly `word`.
    fn eat(&mut self, word: &str) -> bool {
        if self.peek().is_some_and(|t| t.text == word) {
            self.pos += 1;
            return true;
        }
        false
    }

    /// Consume `word` or fail naming it.
    fn expect(&mut self, word: &str) -> Result<(), DrishyaError> {
        if self.eat(word) {
            return Ok(());
        }
        let found = self.peek().map_or("अन्तः".to_string(), |t| t.text.clone());
        self.err(format!("{word} इति अपेक्षितम् ऽ {found} इति प्राप्तम्"))
    }

    fn eat_kind(&mut self, kind: &Kind) -> bool {
        if self.peek().is_some_and(|t| t.kind == *kind) {
            self.pos += 1;
            return true;
        }
        false
    }

    /// A name: any word or sigil-bearing operand, taken whole.
    ///
    /// `Kind::Operand` is accepted because the T0 lexer classifies by final
    /// akṣara and most of this vocabulary ends in `म्` — `दृश्यम्`, `वर्धनम्`,
    /// `मध्यम्`, `अन्तरम्` all arrive as operands with a कर्म kāraka they do
    /// not mean. `super::parse::match_word` already makes the same allowance.
    fn name(&mut self) -> Result<String, DrishyaError> {
        match self.peek() {
            Some(t) if matches!(t.kind, Kind::Word | Kind::Operand { .. }) => {
                self.pos += 1;
                Ok(t.text.clone())
            }
            _ => self.err("नाम अपेक्षितम्"),
        }
    }

    /// `वृत्तिः दृश्यम् आरभ्य स ॱॱ स्थितिः समाप्तम् फलम् रूपम् आदि …`
    fn view(&mut self) -> Result<Rupa, DrishyaError> {
        self.expect("वृत्तिः")?;
        self.expect("दृश्यम्")?;
        self.expect("आरभ्य")?;
        // The parameter's name is what `सॱसङ्ख्या` reaches the state through,
        // so it is bound rather than skipped.
        let param = self.name()?;
        if !self.eat_kind(&Kind::LabelMark) {
            return self.err("ॱॱ इति अपेक्षितम् प्राचलस्य प्रकाराय");
        }
        self.expect("स्थितिः")?;
        self.expect("समाप्तम्")?;
        self.expect("फलम्")?;
        // The return type IS the claim that this is a view function. A
        // `वृत्तिः दृश्यम्` returning anything else is a different function
        // that happens to share a name, and reading its body as a view would
        // be this module inventing a meaning for it.
        self.expect("रूपम्")?;
        self.expect("आदि")?;
        self.expect("प्रत्यागमनम्")?;
        let rupa = self.rupa(&param)?;
        if !self.eat_kind(&Kind::Danda) {
            return self.err("। इति अपेक्षितम् प्रत्यागमनस्य अन्ते");
        }
        self.expect("इति")?;
        if !self.eat_kind(&Kind::DoubleDanda) {
            return self.err("॥ इति अपेक्षितम् वृत्तेः अन्ते");
        }
        Ok(rupa)
    }

    /// One of the four forms.
    fn rupa(&mut self, param: &str) -> Result<Rupa, DrishyaError> {
        let Some(head) = self.peek().map(|t| t.text.clone()) else {
            return self.err("रूपम् अपेक्षितम्");
        };
        match head.as_str() {
            "स्तम्भः" | "पङ्क्तिः" => self.container(&head, param),
            "पाठः" => self.patha(param),
            "कुञ्जिका" => self.kunjika(),
            _ => self.err(format!("{head} इति रूपम् न भवति")),
        }
    }

    /// `स्तम्भः`/`पङ्क्तिः` — the only two forms that carry a `विन्यासः`,
    /// because a `विन्यासः` arranges *children* and a leaf has none. A
    /// container that omits one takes [`Vinyasa::default`], which is what doc
    /// 07 §4.2's inner `पङ्क्तिः` does.
    fn container(&mut self, head: &str, param: &str) -> Result<Rupa, DrishyaError> {
        self.expect(head)?;
        self.expect("आदि")?;
        let mut vinyasa = Vinyasa::default();
        if self.peek().is_some_and(|t| t.text == "विन्यासः") {
            vinyasa = self.vinyasa()?;
            if !self.eat_kind(&Kind::Separator) {
                return self.err("ऽ इति अपेक्षितम् विन्यासस्य अनन्तरम्");
            }
        }
        let santati = self.santati(param)?;
        self.expect("इति")?;
        Ok(if head == "स्तम्भः" {
            Rupa::Stambha { vinyasa, santati }
        } else {
            Rupa::Pankti { vinyasa, santati }
        })
    }

    /// `विन्यासः भवति आदि अन्तरम् भवति १२ ऽ संरेखः भवति मध्यम् इति`
    fn vinyasa(&mut self) -> Result<Vinyasa, DrishyaError> {
        self.expect("विन्यासः")?;
        self.expect("भवति")?;
        self.expect("आदि")?;
        self.expect("अन्तरम्")?;
        self.expect("भवति")?;
        let antara = self.signed_numeral()?;
        if !self.eat_kind(&Kind::Separator) {
            return self.err("ऽ इति अपेक्षितम् अन्तरस्य अनन्तरम्");
        }
        self.expect("संरेखः")?;
        self.expect("भवति")?;
        // Read as a bare name and *then* interpreted, which is what makes
        // `संरेखः भवति आदि` an alignment and not an opening block: nothing
        // here can open one, so `आदि` has only its other sense.
        let word = self.name()?;
        let Some(samrekha) = Samrekha::from_word(&word) else {
            return self.err(format!("{word} इति संरेखः न भवति"));
        };
        self.expect("इति")?;
        let Some(v) = Vinyasa::new(antara, samrekha) else {
            return self.err(format!("अन्तरम् {antara} ऋणात्मकम् ऽ सन्ततिः व्यत्यस्येत्"));
        };
        Ok(v)
    }

    /// `सन्ततिः भवति अङ्कः रूपम् ऽ रूपम् … अन्तः`
    fn santati(&mut self, param: &str) -> Result<Vec<Rupa>, DrishyaError> {
        self.expect("सन्ततिः")?;
        self.expect("भवति")?;
        self.expect("अङ्कः")?;
        let mut out = Vec::new();
        if self.eat("अन्तः") {
            return Ok(out);
        }
        loop {
            out.push(self.rupa(param)?);
            if self.eat_kind(&Kind::Separator) {
                continue;
            }
            self.expect("अन्तः")?;
            return Ok(out);
        }
    }

    /// `पाठः आरभ्य उक्तम् गणना इति अधि विवरम् अधि अङ्कपाठः आरभ्य सॱसङ्ख्या समाप्तम् समाप्तम्`
    fn patha(&mut self, param: &str) -> Result<Rupa, DrishyaError> {
        self.expect("पाठः")?;
        self.expect("आरभ्य")?;
        let text = self.text_expr(param)?;
        self.expect("समाप्तम्")?;
        Ok(Rupa::Patha(text))
    }

    /// `कुञ्जिका आरभ्य उक्तम् वर्धय इति ऽ वर्धनम् समाप्तम्`
    fn kunjika(&mut self) -> Result<Rupa, DrishyaError> {
        self.expect("कुञ्जिका")?;
        self.expect("आरभ्य")?;
        let patha = self.string()?;
        if !self.eat_kind(&Kind::Separator) {
            return self.err("ऽ इति अपेक्षितम् कुञ्जिकायाः पाठस्य अनन्तरम्");
        }
        let ghatana = self.event()?;
        self.expect("समाप्तम्")?;
        Ok(Rupa::Kunjika { patha, ghatana })
    }

    /// `वर्धनम्`, or `घटनाॱवर्धनम्` — a variant of the module's `घटना`,
    /// resolved to its place in the declaration (`F-004f5`).
    ///
    /// The sketch writes the variant bare and `W-189` found the corpus does
    /// too (*"a variant is a name at module scope"*);
    /// `tests/corpus/t1/अवस्थायन्त्रम्.सस` writes it qualified by its type.
    /// Both are read. A qualifier that is any other word names a variant of
    /// some other type and is refused, because a `कुञ्जिका` emits a `घटना`
    /// and nothing else.
    fn event(&mut self) -> Result<Ghatana, DrishyaError> {
        let Some(variants) = self.ghatana else {
            return self.err(format!(
                "इदम् मण्डलम् प्रकारम् {GHATANA} गणनाम् न घोषयति ऽ कुञ्जिका घटनाम् न वहति"
            ));
        };
        let word = self.name()?;
        let qualified = match word.split_once(MEMBER_MARK) {
            None => None,
            Some((root, variant)) if root == GHATANA => Some(variant.to_owned()),
            Some((root, _)) => {
                return self.err(format!(
                    "{root} इति {GHATANA} न भवति ऽ कुञ्जिका घटनायाः भेदम् एव वहति"
                ));
            }
        };
        let nama = qualified.unwrap_or(word);
        let Some(krama) = variants.iter().position(|v| *v == nama) else {
            return self.err(format!("{nama} इति घटनायां भेदः नास्ति"));
        };
        Ok(Ghatana { nama, krama })
    }

    /// `प्रकारः घटना भवति गणना आदि वर्धनम् ऽ ह्रासः ऽ इति ॥` — the variants,
    /// in order.
    ///
    /// `गणना` is required in the same spirit as `फलम् रूपम्` in [`Self::view`]:
    /// a `प्रकारः घटना भवति संरचना` is a different declaration that shares a
    /// name, and a struct has no variants to be an event. The separator after
    /// the LAST variant is optional and no other is: the sketch and the corpus
    /// both write a trailing `ऽ`, `enum_body` is not frozen, and this reads
    /// the one thing the corpus leaves open permissively and nothing else.
    fn ganana(&mut self) -> Result<Vec<String>, DrishyaError> {
        self.expect("प्रकारः")?;
        self.expect(GHATANA)?;
        self.expect("भवति")?;
        self.expect("गणना")?;
        self.expect("आदि")?;
        let mut variants: Vec<String> = Vec::new();
        while !self.eat("इति") {
            let nama = self.name()?;
            if variants.contains(&nama) {
                return self.err(format!("{nama} इति भेदः घटनायां द्विः घोषितः"));
            }
            variants.push(nama);
            if !self.eat_kind(&Kind::Separator) {
                self.expect("इति")?;
                break;
            }
        }
        if !self.eat_kind(&Kind::DoubleDanda) {
            return self.err("॥ इति अपेक्षितम् प्रकारस्य अन्ते");
        }
        Ok(variants)
    }

    /// `प्रकारः स्थितिः भवति संरचना आदि सङ्ख्याॱॱ अ३२ ऽ इति ॥` — the members,
    /// in order.
    ///
    /// `संरचना` is required for [`Self::ganana`]'s reason mirrored: a
    /// `प्रकारः स्थितिः भवति गणना` is a different declaration that shares a
    /// name, and an enum has no members for an `अङ्कपाठः` to read.
    ///
    /// # Why a member's type is collected rather than parsed
    ///
    /// `spec/grammar-t1.ebnf:1597` lists `struct_body` among the productions
    /// that are NOT frozen, and the corpus writes member types several tokens
    /// long. So the words of a type are gathered to the `ऽ` or `इति` that ends
    /// the member, at nesting depth zero, and joined — see [`Anga`]. The three
    /// bracket pairs ADR-0003 ratified all nest inside a type
    /// (`आरभ्य … समाप्तम्` in a function type, `अङ्कः … अन्तः` in an array
    /// type, `आदि … इति` in neither today but counted rather than assumed
    /// absent), so each is tracked: an interior `ऽ` — a function type's second
    /// argument — must not end the member, and `इति` must close its own
    /// bracket before it can close the body.
    ///
    /// The trailing separator is optional exactly as in [`Self::ganana`], and
    /// for the stronger reason: the sketch and every `संरचना` in the corpus
    /// write one.
    fn sanrachana(&mut self) -> Result<Vec<Anga>, DrishyaError> {
        self.expect("प्रकारः")?;
        self.expect(STHITI)?;
        self.expect("भवति")?;
        self.expect("संरचना")?;
        self.expect("आदि")?;
        let mut angani: Vec<Anga> = Vec::new();
        while !self.eat("इति") {
            let nama = self.name()?;
            if angani.iter().any(|a| a.nama == nama) {
                return self.err(format!("{nama} इति अङ्गम् स्थितौ द्विः घोषितम्"));
            }
            if !self.eat_kind(&Kind::LabelMark) {
                return self.err(format!("ॱॱ इति अपेक्षितम् {nama} इत्यस्य प्रकाराय"));
            }
            let (prakara, ended) = self.prakara_padani()?;
            if prakara.is_empty() {
                return self.err(format!("{nama} इत्यस्य प्रकारः न लिखितः"));
            }
            angani.push(Anga { nama, prakara });
            if ended {
                break;
            }
        }
        if !self.eat_kind(&Kind::DoubleDanda) {
            return self.err("॥ इति अपेक्षितम् प्रकारस्य अन्ते");
        }
        Ok(angani)
    }

    /// The words of one member's type, and whether the body ended with it.
    ///
    /// `true` means the member was closed by the body's own `इति` — which this
    /// consumes, so the caller must not look for it again — and `false` that it
    /// was closed by a `ऽ` and another member may follow.
    fn prakara_padani(&mut self) -> Result<(String, bool), DrishyaError> {
        let mut padani: Vec<String> = Vec::new();
        let mut gahanata = 0usize;
        loop {
            let Some(t) = self.peek() else {
                return self.err("इति इत्यस्य अन्तः प्रकारे न प्राप्तः");
            };
            match t.text.as_str() {
                "आरभ्य" | "अङ्कः" | "आदि" => gahanata += 1,
                "समाप्तम्" | "अन्तः" => {
                    gahanata = gahanata.saturating_sub(1)
                }
                "इति" if gahanata == 0 => {
                    self.pos += 1;
                    return Ok((padani.join(" "), true));
                }
                "इति" => gahanata -= 1,
                _ if gahanata == 0 && matches!(t.kind, Kind::Separator) => {
                    self.pos += 1;
                    return Ok((padani.join(" "), false));
                }
                _ => {}
            }
            padani.push(t.text.clone());
            self.pos += 1;
        }
    }

    /// Text pieces joined by `अधि`, with nothing between them.
    ///
    /// **The join used to insert one space, and ADR-0018 deleted it.** See the
    /// module docs: the space was there because a literal could not end in one
    /// and doc 07 §4.2's `॰ॱ` did not decode, so the operator supplied what the
    /// text could not say. `विवरम्` says it now, in the text, where the sketch
    /// wrote it — and a join that adds a character nobody wrote is exactly the
    /// class of defect ADR-0017 was written to remove one level down.
    fn text_expr(&mut self, param: &str) -> Result<String, DrishyaError> {
        let mut out = self.text_atom(param)?;
        while self.eat("अधि") {
            out.push_str(&self.text_atom(param)?);
        }
        Ok(out)
    }

    fn text_atom(&mut self, param: &str) -> Result<String, DrishyaError> {
        if self.peek().is_some_and(|t| t.text == "अङ्कपाठः") {
            self.pos += 1;
            self.expect("आरभ्य")?;
            let n = self.state_field(param)?;
            self.expect("समाप्तम्")?;
            // ADR-0023: Devanagari, because every other numeral in this
            // language is. This used to emit ASCII via `n.to_string()` and said
            // in its own docs that it did so only because `F-004a`'s fixture
            // was ASCII — the fixture chose, not the language. `F-004f3` is
            // that ADR, and the fixture below moved to match it.
            return Ok(sanskrit_text::numeral::to_devanagari_signed(i64::from(n)));
        }
        self.string()
    }

    /// `सॱसङ्ख्या` — the bound parameter, a member mark, a field of the state.
    ///
    /// **MEASURED: the member mark does not arrive as a token.** `lex.rs:262`
    /// peels only a *trailing* `ॱॱ`, `।`, `॥` and `ऽ` off a word, so an infix
    /// `ॱ` stays inside it and `सॱसङ्ख्या` reaches the parser whole — one
    /// [`Kind::Word`] and not three tokens. That is also how the T1 corpus
    /// already writes member access: `स्वयम्ॱदैर्घ्यम्` in
    /// `tests/corpus/t1/सामान्यम्.सस`. So the split is the parser's, and it is
    /// done here rather than in the lexer because the lexer serves T0 and
    /// `spec/grammar-t0.ebnf` is frozen.
    fn state_field(&mut self, param: &str) -> Result<i32, DrishyaError> {
        let word = self.name()?;
        let Some((root, field)) = word.split_once(MEMBER_MARK) else {
            return self.err(format!("{word} मध्ये ॱ नास्ति ऽ स्थितेः अङ्गम् न भवति"));
        };
        if root != param {
            return self.err(format!("{root} इति अज्ञातम् ऽ अस्याम् वृत्तौ {param} एव स्थितिः"));
        }
        // ── F-004f: THE MEMBER IS TYPED, AND THE SOURCE TYPES IT ──────────
        //
        // The two questions below used to be one, and collapsing them is what
        // made this reader half-typed. `self.sthiti` is the HOST's map, handed
        // in by whoever called `render`; the declaration is the SOURCE's. A
        // member absent from the declaration is a defect in the view, and a
        // member present in it but absent from the map is a defect in the
        // caller — the first refuses the program, the second refuses the call,
        // and they cannot share a sentence.
        let Some(angani) = self.anga else {
            return self.err(format!(
                "अस्मिन् मण्डले प्रकारः स्थितिः न घोषितः ऽ {field} इत्यस्य अङ्कपाठः न शक्यः"
            ));
        };
        let Some(anga) = angani.iter().find(|a| a.nama == field) else {
            return self.err(format!("स्थितौ {field} इति अङ्गम् न घोषितम्"));
        };
        if anga.prakara != A32 {
            let prakara = &anga.prakara;
            return self.err(format!(
                "{field} इति अङ्गम् {prakara} प्रकारस्य ऽ अङ्कपाठः {A32} एव पठति"
            ));
        }
        match self.sthiti.get(field) {
            Some(v) => Ok(v),
            None => self.err(format!("{field} इति अङ्गम् घोषितम् ऽ मूल्यम् तु न बद्धम्")),
        }
    }

    /// `उक्तम् … इति`, exactly as it was written — or a layout word, which is
    /// the same kind of token and needed no second rule here.
    ///
    /// ADR-0018's `यतिः` and `विवरम्` arrive as [`Kind::Str`] too, and that is
    /// the point of giving them that kind: a layout character is a value, and
    /// the reader of a value did not have to learn what one is.
    ///
    /// ADR-0017 made the literal a [`Kind::Str`] token, so this reads a value
    /// instead of rebuilding one. What it used to do — walk words to the first
    /// `इति` and join them with one space — is why the module docs above had a
    /// one-space *convention* to explain: the lexer had already dropped the
    /// spacing. It no longer does, and the convention is now a fact about the
    /// source rather than a rule about the parser.
    fn string(&mut self) -> Result<String, DrishyaError> {
        let Some(t) = self.peek() else {
            return self.err("उक्तम् इत्यस्य अन्तः इति न प्राप्तम्");
        };
        let Kind::Str { value } = &t.kind else {
            let found = t.text.clone();
            return self.err(format!("उक्तम् अपेक्षितम् ऽ {found} इति प्राप्तम्"));
        };
        let value = value.clone();
        self.pos += 1;
        Ok(value)
    }

    /// A numeral, optionally preceded by `ऋण`.
    ///
    /// `sanskrit_text::numeral` puts the sign OUTSIDE the radix prefix and its
    /// `is_numeral` is false for a signed literal, so the lexer hands `ऋण` over
    /// as a separate word and this is where the two are put back together.
    fn signed_numeral(&mut self) -> Result<i32, DrishyaError> {
        let negative = self.eat("ऋण");
        let Some(t) = self.peek() else {
            return self.err("सङ्ख्या अपेक्षिता");
        };
        if t.kind != Kind::Numeral {
            let found = t.text.clone();
            return self.err(format!("सङ्ख्या अपेक्षिता ऽ {found} इति प्राप्तम्"));
        }
        let text = t.text.clone();
        self.pos += 1;
        let Ok(v) = sanskrit_text::numeral::value(&text) else {
            return self.err(format!("{text} इति सङ्ख्या न पठ्यते"));
        };
        let Ok(v) = i32::try_from(v) else {
            return self.err(format!("{text} इति सङ्ख्या अ३२ मध्ये न माति"));
        };
        Ok(if negative { -v } else { v })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::lex_t1 as lex;

    /// The sketch's `दृश्यम्`, alone, in ADR-0003's spelling. The whole module
    /// is in `crates/sadhana/tests/गणकः.सस`; this is the declaration the unit
    /// tests below vary.
    const VIEW: &str = "\
वृत्तिः दृश्यम् आरभ्य सॱॱ स्थितिः समाप्तम् फलम् रूपम् आदि
    प्रत्यागमनम् स्तम्भः आदि
        विन्यासः भवति आदि अन्तरम् भवति १२ ऽ संरेखः भवति मध्यम् इति ऽ
        सन्ततिः भवति अङ्कः
            पाठः आरभ्य उक्तम् गणना इति अधि विवरम् अधि अङ्कपाठः आरभ्य सॱसङ्ख्या समाप्तम् समाप्तम् ऽ
            पङ्क्तिः आदि सन्ततिः भवति अङ्कः
                कुञ्जिका आरभ्य उक्तम् वर्धय इति ऽ वर्धनम् समाप्तम् ऽ
                कुञ्जिका आरभ्य उक्तम् ह्रासय इति ऽ ह्रासः समाप्तम्
            अन्तः इति
        अन्तः
    इति ।
इति ॥
";

    /// The sketch's event type, alone. `F-004f5`: a `कुञ्जिका`'s event is a
    /// variant of THIS declaration, so every view below that has a button is
    /// read with it in front — which is where `गणकः.सस` puts it too.
    const GHATANA_GANANA: &str = "प्रकारः घटना भवति गणना आदि वर्धनम् ऽ ह्रासः ऽ इति ॥\n";

    /// The sketch's state type, alone. `F-004f`: an `अङ्कपाठः` reads a member
    /// of THIS declaration, so every view below that names one is read with it
    /// in front — which is where `गणकः.सस` puts it too, first of the two.
    const STHITI_SANRACHANA: &str = "प्रकारः स्थितिः भवति संरचना आदि सङ्ख्याॱॱ अ३२ ऽ इति ॥\n";

    /// `view` with the sketch's two types declared before it, in the sketch's
    /// own order.
    fn module(view: &str) -> String {
        format!("{STHITI_SANRACHANA}{GHATANA_GANANA}{view}")
    }

    /// The typed event `F-004f5` resolves `word` to, at `krama`.
    fn ghatana(word: &str, krama: usize) -> Ghatana {
        Ghatana {
            nama: word.into(),
            krama,
        }
    }

    fn view_at(n: i32) -> Result<Rupa, DrishyaError> {
        let tokens = lex(&module(VIEW)).expect("the sketch lexes");
        render(&tokens, &Sthiti::new().with("सङ्ख्या", n))
    }

    /// Read `src` exactly as given — no declaration is put in front of it.
    fn read_module(src: &str) -> Result<Rupa, DrishyaError> {
        let tokens = lex(src).expect("lexes");
        render(&tokens, &Sthiti::new().with("सङ्ख्या", 0))
    }

    /// Read a view with the sketch's `घटना` declared before it.
    fn read(src: &str) -> Result<Rupa, DrishyaError> {
        read_module(&module(src))
    }

    #[test]
    fn the_sketch_reads_as_the_tree_f_004a_builds() {
        let view = view_at(0).expect("the sketch reads");
        assert_eq!(
            view,
            Rupa::Stambha {
                vinyasa: Vinyasa::new(12, Samrekha::Madhyama).expect("12 is not negative"),
                santati: vec![
                    Rupa::Patha("गणना ०".into()),
                    Rupa::Pankti {
                        vinyasa: Vinyasa::default(),
                        santati: vec![
                            Rupa::Kunjika {
                                patha: "वर्धय".into(),
                                ghatana: ghatana("वर्धनम्", 0),
                            },
                            Rupa::Kunjika {
                                patha: "ह्रासय".into(),
                                ghatana: ghatana("ह्रासः", 1),
                            },
                        ],
                    },
                ],
            }
        );
    }

    /// Every observable `F-004a`'s own test asserts, asserted on the *parsed*
    /// tree rather than on a hand-built one.
    #[test]
    fn the_parsed_view_answers_what_f_004a_asks_of_the_built_one() {
        let view = view_at(0).expect("reads");
        assert_eq!(view.node_count(), 5, "column, text, row, two buttons");
        assert_eq!(view.depth(), 3, "column > row > button");
        assert_eq!(view.text(), ["गणना ०", "वर्धय", "ह्रासय"]);
        assert_eq!(view.events(), ["वर्धनम्", "ह्रासः"]);
    }

    /// D-07-B's *"no CSS cascade"*, pinned on a parsed tree: the column
    /// arranges with 12 and centring, the row inside it with the default 0 and
    /// start. If anything ever inherits, this says so.
    #[test]
    fn an_arrangement_does_not_reach_the_children_it_arranges() {
        let view = view_at(0).expect("reads");
        let outer = view.arrangement().expect("a column arranges");
        assert_eq!(outer.antara(), 12);
        assert_eq!(outer.samrekha(), Samrekha::Madhyama);

        let inner = view.children()[1].arrangement().expect("a row arranges");
        assert_eq!(inner.antara(), 0, "the row did not inherit the column's 12");
        assert_eq!(inner.samrekha(), Samrekha::Adi, "nor its centring");
    }

    /// D-07-B's *"pure `स्थिति → दृश्यम्`"*: the same state twice gives the
    /// same view, and a different state a different one. The interpolation is
    /// the only thing the state can reach, so this is also the test that the
    /// state is READ rather than ignored.
    #[test]
    fn the_view_is_a_function_of_the_state() {
        assert_eq!(view_at(7).expect("reads"), view_at(7).expect("reads"));
        assert_ne!(view_at(7).expect("reads"), view_at(8).expect("reads"));
        assert_eq!(view_at(7).expect("reads").text()[0], "गणना ७");
        assert_eq!(view_at(-3).expect("reads").text()[0], "गणना ऋण३");
    }

    /// The one collision in the grammar. `आदि` opens a block *and* names the
    /// default alignment; after `संरेखः भवति` only the second sense is
    /// reachable.
    #[test]
    fn the_alignment_named_adi_is_not_a_block() {
        let src = VIEW.replace("संरेखः भवति मध्यम्", "संरेखः भवति आदि");
        let view = read(&src).expect("आदि is an alignment here");
        assert_eq!(
            view.arrangement().expect("a column arranges").samrekha(),
            Samrekha::Adi
        );
    }

    #[test]
    fn every_alignment_the_type_has_can_be_written() {
        for (word, want) in [
            ("आदि", Samrekha::Adi),
            ("मध्यम्", Samrekha::Madhyama),
            ("अन्त", Samrekha::Anta),
        ] {
            let src = VIEW.replace("संरेखः भवति मध्यम्", &format!("संरेखः भवति {word}"));
            assert_eq!(
                read(&src)
                    .expect("alignment reads")
                    .arrangement()
                    .expect("a column arranges")
                    .samrekha(),
                want,
                "{word}"
            );
        }
    }

    /// A container with no `विन्यासः` takes the arrangement that adds nothing,
    /// which is exactly what the sketch's inner `पङ्क्तिः` relies on.
    #[test]
    fn a_container_without_an_arrangement_takes_the_one_that_adds_nothing() {
        let src = VIEW.replace("विन्यासः भवति आदि अन्तरम् भवति १२ ऽ संरेखः भवति मध्यम् इति ऽ", "");
        let view = read(&src).expect("a column may omit its arrangement");
        let v = view.arrangement().expect("a column still arranges");
        assert_eq!(v.antara(), 0);
        assert_eq!(v.samrekha(), Samrekha::Adi);
    }

    #[test]
    fn refuses_a_negative_gap() {
        let src = VIEW.replace("अन्तरम् भवति १२", "अन्तरम् भवति ऋण १");
        let e = read(&src).expect_err("a negative gap is refused");
        assert!(e.reason.contains("ऋणात्मकम्"), "got {}", e.reason);
    }

    /// The refusal above is only worth having if `ऋण` is read at all — a
    /// reader that dropped the sign would refuse nothing and pass.
    #[test]
    fn the_sign_is_read_rather_than_dropped() {
        let src = VIEW.replace("अन्तरम् भवति १२", "अन्तरम् भवति ऋण ० ");
        let view = read(&src).expect("negative zero is zero");
        assert_eq!(view.arrangement().expect("arranges").antara(), 0);
    }

    #[test]
    fn refuses_a_state_field_that_is_not_bound() {
        let tokens = lex(&module(VIEW)).expect("lexes");
        let e = render(&tokens, &Sthiti::new()).expect_err("no सङ्ख्या is bound");
        assert!(e.reason.contains("सङ्ख्या"), "got {}", e.reason);
        // `F-004f`: the member IS declared, so this is the CALLER's omission
        // and must not read as the view's. `an_undeclared_member_is_refused`
        // below is the other half, and the two messages are disjoint.
        assert!(
            e.reason.contains("न बद्धम्"),
            "the binding is what is missing: {}",
            e.reason
        );
        assert!(
            !e.reason.contains("न घोषितम्"),
            "and the declaration is not: {}",
            e.reason
        );
    }

    #[test]
    fn refuses_a_root_that_is_not_the_bound_parameter() {
        let src = VIEW.replace("सॱसङ्ख्या", "कॱसङ्ख्या");
        let e = read(&src).expect_err("क is not the parameter");
        assert!(e.reason.contains("अज्ञातम्"), "got {}", e.reason);
    }

    #[test]
    fn refuses_a_word_that_is_not_a_form() {
        let src = VIEW.replace("पङ्क्तिः आदि", "मञ्चः आदि");
        let e = read(&src).expect_err("मञ्चः is not a form");
        assert!(e.reason.contains("मञ्चः"), "got {}", e.reason);
    }

    /// A `वृत्तिः दृश्यम्` that does not return a `रूपम्` is a different
    /// function with the same name, and reading its body as a view would be
    /// inventing a meaning for it.
    #[test]
    fn refuses_a_drishya_that_does_not_yield_a_rupa() {
        let src = VIEW.replace("फलम् रूपम्", "फलम् अ३२");
        let e = read(&src).expect_err("not a view function");
        assert!(e.reason.contains("रूपम्"), "got {}", e.reason);
    }

    #[test]
    fn refuses_a_module_with_no_drishya_at_all() {
        let tokens = lex("ॐ\n॥ मण्डलम् रिक्तम् ॥\n").expect("lexes");
        let e = render(&tokens, &Sthiti::new()).expect_err("there is no view");
        assert!(e.reason.contains("दृश्यम्"), "got {}", e.reason);
    }

    /// The sub-language is embedded in T1, so what precedes it is skipped
    /// unread rather than parsed — with the two exceptions `render` names, the
    /// two types the view itself refers to. The declaration this module does
    /// not understand, and skips, is `वृत्तिः परिवर्तनम्`.
    ///
    /// **`वृत्तिः परिवर्तनम्` is one of them, and it is load-bearing.** A
    /// mutation that made the view finder (now `find_declaration`) take the first `वृत्तिः` whatever its
    /// name SURVIVED this test while its prefix held only `प्रकारः`
    /// declarations — the integration test caught it and this one did not,
    /// because there was no competing function to be confused with. The
    /// competing function is now here.
    #[test]
    fn the_declarations_before_the_view_are_not_read() {
        let src = format!(
            "ॐ\n॥ मण्डलम् गणकः ॥\n\
             प्रकारः स्थितिः भवति संरचना आदि सङ्ख्याॱॱ अ३२ ऽ इति ॥\n\
             प्रकारः घटना भवति गणना आदि वर्धनम् ऽ ह्रासः ऽ इति ॥\n\
             वृत्तिः परिवर्तनम् आरभ्य सॱॱ स्थितिः ऽ घॱॱ घटना समाप्तम् फलम् स्थितिः आदि\n\
             विकल्पना घ आदि\n\
             वर्धनम् ततः प्रत्यागमनम् स्थितिः आदि सङ्ख्या भवति सॱसङ्ख्या अधि १ इति ।\n\
             इति\nइति ॥\n{VIEW}"
        );
        // Read as written: the prefix already declares both `स्थितिः`
        // (`F-004f`) and `घटना` (`F-004f5`), which are the two declarations
        // before the view that ARE read, and nothing else in it is.
        assert_eq!(
            read_module(&src).expect("the view is found"),
            view_at(0).expect("ok")
        );
    }

    #[test]
    fn an_empty_santati_is_a_view_with_nothing_in_it() {
        let tokens = lex("वृत्तिः दृश्यम् आरभ्य सॱॱ स्थितिः समाप्तम् फलम् रूपम् आदि\n\
             प्रत्यागमनम् स्तम्भः आदि सन्ततिः भवति अङ्कः अन्तः इति ।\nइति ॥\n")
        .expect("lexes");
        let view = render(&tokens, &Sthiti::new()).expect("reads");
        assert_eq!(view.node_count(), 1);
        assert_eq!(view.depth(), 1);
        assert!(view.text().is_empty());
        assert!(view.events().is_empty());
        assert!(view.arrangement().is_some(), "a column still arranges");
    }

    /// The walk pops a stack, so children must go on in reverse to come off in
    /// order. Three siblings is the smallest count a reversal cannot pass by
    /// symmetry.
    #[test]
    fn siblings_are_read_and_walked_left_to_right() {
        let tokens = lex("वृत्तिः दृश्यम् आरभ्य सॱॱ स्थितिः समाप्तम् फलम् रूपम् आदि\n\
             प्रत्यागमनम् पङ्क्तिः आदि सन्ततिः भवति अङ्कः\n\
             पाठः आरभ्य उक्तम् क इति समाप्तम् ऽ\n\
             पाठः आरभ्य उक्तम् ख इति समाप्तम् ऽ\n\
             पाठः आरभ्य उक्तम् ग इति समाप्तम्\n\
             अन्तः इति ।\nइति ॥\n")
        .expect("lexes");
        let view = render(&tokens, &Sthiti::new()).expect("reads");
        assert_eq!(view.text(), ["क", "ख", "ग"]);
    }

    /// A walk that pushed a whole level before descending would pass the test
    /// above and fail this one.
    #[test]
    fn a_subtree_is_finished_before_the_next_sibling_starts() {
        let column = |a: &str, b: &str| {
            format!(
                "स्तम्भः आदि सन्ततिः भवति अङ्कः \
                 पाठः आरभ्य उक्तम् {a} इति समाप्तम् ऽ \
                 पाठः आरभ्य उक्तम् {b} इति समाप्तम् अन्तः इति"
            )
        };
        let src = format!(
            "वृत्तिः दृश्यम् आरभ्य सॱॱ स्थितिः समाप्तम् फलम् रूपम् आदि\n\
             प्रत्यागमनम् पङ्क्तिः आदि सन्ततिः भवति अङ्कः {} ऽ {} अन्तः इति ।\nइति ॥\n",
            column("क", "ख"),
            column("ग", "घ")
        );
        let tokens = lex(&src).expect("lexes");
        let view = render(&tokens, &Sthiti::new()).expect("reads");
        assert_eq!(view.text(), ["क", "ख", "ग", "घ"]);
        assert_eq!(view.depth(), 3);
    }

    /// A literal of several words keeps the spaces it was written with — the
    /// lexer's doing (ADR-0017) — and the `अधि` join adds none of its own
    /// (ADR-0018). The one space before the count is the fixture's `विवरम्`.
    #[test]
    fn a_literal_keeps_its_own_spaces_and_the_join_adds_none() {
        let src = VIEW.replace("उक्तम् गणना इति", "उक्तम् नमस्कारः संसार इति");
        assert_eq!(read(&src).expect("reads").text()[0], "नमस्कारः संसार ०");
        // Take the `विवरम्` out and the pieces meet with nothing between them,
        // which is what says the space is written rather than supplied.
        let joined = VIEW.replace(" अधि विवरम् अधि ", " अधि ");
        assert_eq!(read(&joined).expect("reads").text()[0], "गणना०");
    }

    /// A button's label is a string and its event is a name; swapping them is
    /// refused rather than silently taking a label as an event.
    #[test]
    fn a_button_wants_a_string_before_its_event() {
        let src = VIEW.replace("कुञ्जिका आरभ्य उक्तम् वर्धय इति", "कुञ्जिका आरभ्य वर्धय");
        let e = read(&src).expect_err("a bare word is not a label");
        assert!(e.reason.contains("उक्तम्"), "got {}", e.reason);
    }

    /// A leaf carries no arrangement, because it has no children to arrange.
    #[test]
    fn a_leaf_arranges_nothing() {
        let view = view_at(0).expect("reads");
        let text = &view.children()[0];
        let button = &view.children()[1].children()[0];
        assert!(text.arrangement().is_none() && text.children().is_empty());
        assert!(button.arrangement().is_none() && button.children().is_empty());
    }

    // ── F-004f5: EVENT TYPING ─────────────────────────────────────────────
    //
    // A `कुञ्जिका`'s event used to be the identifier as written. It is now a
    // variant of the module's `प्रकारः घटना भवति गणना`, resolved to its place
    // in that declaration, and a name the declaration does not hold is
    // REFUSED. The refusal is the first test because it is the one the row
    // exists for: a reader that accepted any word would pass every test above.

    /// THE CASE THAT MUST BE REFUSED. `नाशः` is not a variant of the sketch's
    /// `घटना`, so a button emitting it is a button emitting a value of no type.
    #[test]
    fn an_event_the_ganana_does_not_declare_is_refused() {
        let src = VIEW.replace("ऽ ह्रासः समाप्तम्", "ऽ नाशः समाप्तम्");
        assert_ne!(src, VIEW, "the mutation must land on the second button");
        let e = read(&src).expect_err("नाशः is not a घटना");
        assert!(
            e.reason.contains("नाशः"),
            "the refusal names the word: {}",
            e.reason
        );
        assert!(
            e.reason.contains("घटना"),
            "and the type it is not in: {}",
            e.reason
        );
    }

    /// The refusal is only worth having if the accepted names are READ from
    /// the declaration rather than known to the reader: declare `नाशः` and
    /// the same view is a view.
    #[test]
    fn the_ganana_is_read_from_the_module_and_not_assumed() {
        let src = format!(
            "{STHITI_SANRACHANA}\
             प्रकारः घटना भवति गणना आदि वर्धनम् ऽ ह्रासः ऽ नाशः ऽ इति ॥\n{}",
            VIEW.replace("ऽ ह्रासः समाप्तम्", "ऽ नाशः समाप्तम्")
        );
        let view = read_module(&src).expect("नाशः is declared here");
        assert_eq!(view.events(), ["वर्धनम्", "नाशः"]);
        let Rupa::Kunjika { ghatana: g, .. } = &view.children()[1].children()[1] else {
            panic!("the second child of the row is a button");
        };
        assert_eq!(*g, ghatana("नाशः", 2), "third in the declaration");
    }

    /// An event is typed to its PLACE in the `गणना`, in declaration order —
    /// which is what `Ty::Enum`'s discriminant is one level down and what a
    /// router can match on without comparing strings. Reversing the
    /// declaration reverses the places, so the ordinal is read, not counted
    /// from the view.
    #[test]
    fn an_event_is_resolved_to_its_place_in_the_ganana() {
        let view = view_at(0).expect("reads");
        let row = view.children()[1].children();
        assert_eq!(
            row,
            [
                Rupa::Kunjika {
                    patha: "वर्धय".into(),
                    ghatana: ghatana("वर्धनम्", 0),
                },
                Rupa::Kunjika {
                    patha: "ह्रासय".into(),
                    ghatana: ghatana("ह्रासः", 1),
                },
            ]
        );

        let reversed =
            format!("{STHITI_SANRACHANA}प्रकारः घटना भवति गणना आदि ह्रासः ऽ वर्धनम् ऽ इति ॥\n{VIEW}");
        let view = read_module(&reversed).expect("reads");
        let row = view.children()[1].children();
        let Rupa::Kunjika { ghatana: first, .. } = &row[0] else {
            panic!("a button")
        };
        let Rupa::Kunjika {
            ghatana: second, ..
        } = &row[1]
        else {
            panic!("a button")
        };
        assert_eq!(*first, ghatana("वर्धनम्", 1));
        assert_eq!(*second, ghatana("ह्रासः", 0));
    }

    /// A module that declares no `घटना` has no event type, so a button in it
    /// emits a value of nothing and is refused. This is the sketch's `दृश्यम्`
    /// on its own, which every test above `F-004f5` used to read.
    #[test]
    fn a_button_in_a_module_with_no_ghatana_is_refused() {
        // The state type IS declared here and the event type is not: the
        // `पाठः` precedes the buttons in the tree, so a module declaring
        // NEITHER is refused for the state and this test's subject never
        // runs. That is the two refusals being distinct, not an accident.
        let e =
            read_module(&format!("{STHITI_SANRACHANA}{VIEW}")).expect_err("no घटना is declared");
        assert!(e.reason.contains("घटना"), "got {}", e.reason);
        assert!(
            e.reason.contains("कुञ्जिका"),
            "and says what needed it: {}",
            e.reason
        );
    }

    /// The event type is the `गणना` NAMED `घटना` — doc 07 §4.2's own name and
    /// the second parameter of `परिवर्तनम्` — exactly as the state type is the
    /// one named `स्थितिः`. Another enum with the right variants is another
    /// type.
    #[test]
    fn a_ganana_of_another_name_does_not_type_events() {
        let src =
            format!("{STHITI_SANRACHANA}प्रकारः अवस्था भवति गणना आदि वर्धनम् ऽ ह्रासः ऽ इति ॥\n{VIEW}");
        let e = read_module(&src).expect_err("अवस्था is not घटना");
        assert!(e.reason.contains("घटना"), "got {}", e.reason);
    }

    /// `प्रकारः घटना भवति संरचना` is a different declaration that shares the
    /// name, and a struct has no variants to be an event. Same rule as
    /// `फलम् रूपम्`: the kind is the claim, and the reader does not invent one.
    #[test]
    fn a_ghatana_that_is_not_a_ganana_is_refused() {
        let src =
            format!("{STHITI_SANRACHANA}प्रकारः घटना भवति संरचना आदि सङ्ख्याॱॱ अ३२ ऽ इति ॥\n{VIEW}");
        let e = read_module(&src).expect_err("a struct is not an event type");
        assert!(e.reason.contains("गणना"), "got {}", e.reason);
    }

    /// A `गणना` that names a variant twice cannot give it one place.
    #[test]
    fn a_ganana_that_repeats_a_variant_is_refused() {
        let src =
            format!("{STHITI_SANRACHANA}प्रकारः घटना भवति गणना आदि वर्धनम् ऽ वर्धनम् ऽ इति ॥\n{VIEW}");
        let e = read_module(&src).expect_err("a repeated variant");
        assert!(e.reason.contains("वर्धनम्"), "got {}", e.reason);
    }

    /// `tests/corpus/t1/अवस्थायन्त्रम्.सस` writes a variant as
    /// `घटनाॱप्रारम्भघटना`, qualified by its type; the sketch writes it bare.
    /// Both name the same variant and both read — but a qualifier that is not
    /// `घटना` names a variant of some OTHER type, and is refused.
    #[test]
    fn an_event_may_be_qualified_by_its_own_type_and_no_other() {
        let qualified = VIEW.replace("ऽ ह्रासः समाप्तम्", "ऽ घटनाॱह्रासः समाप्तम्");
        assert_eq!(
            read(&qualified).expect("reads"),
            view_at(0).expect("reads"),
            "the qualifier changes nothing about the value"
        );
        let other = VIEW.replace("ऽ ह्रासः समाप्तम्", "ऽ अवस्थाॱह्रासः समाप्तम्");
        let e = read(&other).expect_err("अवस्था is not the event type");
        assert!(e.reason.contains("अवस्था"), "got {}", e.reason);
    }

    /// A view with no button needs no event type at all: the declaration is
    /// read for the buttons, and a module with none is not asked for it. The
    /// two walk-order tests above read button-less views with nothing in
    /// front of them and pin the same fact from the other side.
    #[test]
    fn a_view_without_a_button_needs_no_ghatana() {
        let src = VIEW.replace(
            "कुञ्जिका आरभ्य उक्तम् वर्धय इति ऽ वर्धनम् समाप्तम् ऽ\n                \
             कुञ्जिका आरभ्य उक्तम् ह्रासय इति ऽ ह्रासः समाप्तम्",
            "पाठः आरभ्य उक्तम् क इति समाप्तम्",
        );
        assert_ne!(src, VIEW, "the buttons must be replaced");
        let src = format!("{STHITI_SANRACHANA}{src}");
        let view = read_module(&src).expect("a view without buttons reads without a घटना");
        assert!(view.events().is_empty());
        assert_eq!(view.text(), ["गणना ०", "क"]);
    }

    /// A `गणना` may end with or without the trailing `ऽ`: the sketch and the
    /// corpus both write one, and `enum_body` is not frozen, so neither
    /// spelling is refused — while two variants with nothing between them are.
    #[test]
    fn the_last_variant_may_or_may_not_be_followed_by_a_separator() {
        let no_trailing =
            format!("{STHITI_SANRACHANA}प्रकारः घटना भवति गणना आदि वर्धनम् ऽ ह्रासः इति ॥\n{VIEW}");
        assert_eq!(
            read_module(&no_trailing).expect("reads"),
            view_at(0).expect("reads")
        );
        let unseparated =
            format!("{STHITI_SANRACHANA}प्रकारः घटना भवति गणना आदि वर्धनम् ह्रासः इति ॥\n{VIEW}");
        let e = read_module(&unseparated).expect_err("two names with no ऽ between");
        assert!(
            e.reason.contains("ह्रासः"),
            "names the word it stopped at: {}",
            e.reason
        );
    }
    // ── F-004f: THE STATE IS TYPED BY THE SOURCE ─────────────────────────
    //
    // The seven tests below are the `घटना` block's shape applied to the other
    // type a view names. Before them, `state_field` asked `self.sthiti` — the
    // map the CALLER passes — and nothing else, so the view's two references
    // were typed by two different authorities: the event by the module, the
    // state by the host.

    /// A member no `प्रकारः स्थितिः` declares is refused even when the caller's
    /// map carries a value for it. This is the defect, stated: the host's map
    /// could type the view, and here it would have.
    #[test]
    fn an_undeclared_member_is_refused_even_when_the_caller_binds_it() {
        let src = format!(
            "प्रकारः स्थितिः भवति संरचना आदि अन्यत्ॱॱ अ३२ ऽ इति ॥\n\
             {GHATANA_GANANA}{VIEW}"
        );
        let tokens = lex(&src).expect("lexes");
        // The caller binds `सङ्ख्या`, exactly as every passing test does.
        let e = render(&tokens, &Sthiti::new().with("सङ्ख्या", 7))
            .expect_err("सङ्ख्या is not declared in this module");
        assert!(e.reason.contains("सङ्ख्या"), "got {}", e.reason);
        assert!(
            e.reason.contains("न घोषितम्"),
            "the declaration is what is missing: {}",
            e.reason
        );
    }

    /// A module that declares no `स्थितिः` at all has no state type, so an
    /// `अङ्कपाठः` in it reads a member of nothing and is refused — the exact
    /// rule `a_button_in_a_module_with_no_ghatana_is_refused` applies to a
    /// `कुञ्जिका`.
    #[test]
    fn an_ankapatha_in_a_module_with_no_sthiti_is_refused() {
        let e = read_module(&format!("{GHATANA_GANANA}{VIEW}")).expect_err("no स्थितिः is declared");
        assert!(e.reason.contains("स्थितिः"), "got {}", e.reason);
        assert!(
            e.reason.contains("अङ्कपाठः"),
            "and says what needed it: {}",
            e.reason
        );
    }

    /// A view with no `अङ्कपाठः` needs no state type at all, for
    /// `a_view_without_a_button_needs_no_ghatana`'s reason: the declaration is
    /// read for its readers, and a module with none is not asked for it.
    #[test]
    fn a_view_without_an_ankapatha_needs_no_sthiti() {
        let src = VIEW.replace(
            "उक्तम् गणना इति अधि विवरम् अधि अङ्कपाठः आरभ्य सॱसङ्ख्या समाप्तम्",
            "उक्तम् गणना इति",
        );
        assert_ne!(src, VIEW, "the अङ्कपाठः must be replaced");
        let view = read_module(&format!("{GHATANA_GANANA}{src}"))
            .expect("a view with no अङ्कपाठः reads without a स्थितिः");
        // `text()` reaches a `कुञ्जिका`'s label too, so the two buttons are
        // here; the point is that `गणना` lost its numeral and nothing asked
        // for a state.
        assert_eq!(view.text(), ["गणना", "वर्धय", "ह्रासय"]);
    }

    /// `प्रकारः स्थितिः भवति गणना` is a different declaration that shares the
    /// name, and an enum has no members. Same rule as
    /// `a_ghatana_that_is_not_a_ganana_is_refused`, the other way round.
    #[test]
    fn a_sthiti_that_is_not_a_sanrachana_is_refused() {
        let src = format!("प्रकारः स्थितिः भवति गणना आदि सङ्ख्या ऽ इति ॥\n{GHATANA_GANANA}{VIEW}");
        let e = read_module(&src).expect_err("an enum has no members");
        assert!(e.reason.contains("संरचना"), "got {}", e.reason);
    }

    /// A `संरचना` that names a member twice cannot give it one type.
    #[test]
    fn a_sanrachana_that_repeats_a_member_is_refused() {
        let src = format!(
            "प्रकारः स्थितिः भवति संरचना आदि सङ्ख्याॱॱ अ३२ ऽ सङ्ख्याॱॱ अ३२ ऽ इति ॥\n\
             {GHATANA_GANANA}{VIEW}"
        );
        let e = read_module(&src).expect_err("a repeated member");
        assert!(e.reason.contains("द्विः"), "got {}", e.reason);
    }

    /// `अङ्कपाठः` writes a numeral and [`Sthiti`] holds `i32`, so a member
    /// declared at any other type is one it has no value for. The type here is
    /// `पाठः`, which `tests/corpus/t1/यन्त्रम्.सस` declares a member at.
    #[test]
    fn a_member_declared_at_another_type_is_refused() {
        let src = format!(
            "प्रकारः स्थितिः भवति संरचना आदि सङ्ख्याॱॱ पाठः ऽ इति ॥\n\
             {GHATANA_GANANA}{VIEW}"
        );
        let tokens = lex(&src).expect("lexes");
        let e = render(&tokens, &Sthiti::new().with("सङ्ख्या", 7)).expect_err("पाठः is not अ३२");
        assert!(
            e.reason.contains("पाठः"),
            "names the type it found: {}",
            e.reason
        );
        assert!(
            e.reason.contains(A32),
            "and the one it wanted: {}",
            e.reason
        );
    }

    /// A member type is COLLECTED and not parsed ([`Anga`]), so the corpus's
    /// multi-token types read — including a function type whose own `आरभ्य …
    /// समाप्तम्` holds the `ऽ` that would otherwise end the member early. The
    /// `अ३२` member beside it still resolves, which is the fact at issue: a
    /// reader that mis-split the first type would lose the second.
    #[test]
    fn a_multi_word_member_type_does_not_end_the_member_early() {
        let src = format!(
            "प्रकारः स्थितिः भवति संरचना आदि \
             अग्रिमःॱॱ सम्भाव्य स्थानम् गण्डिका ऽ \
             चालकःॱॱ वृत्तिस्थानम् आरभ्य अ३२ ऽ अ३२ समाप्तम् फलम् शून्यम् ऽ \
             सङ्ख्याॱॱ अ३२ ऽ इति ॥\n{GHATANA_GANANA}{VIEW}"
        );
        let tokens = lex(&src).expect("lexes");
        let view = render(&tokens, &Sthiti::new().with("सङ्ख्या", 7)).expect("सङ्ख्या resolves");
        assert_eq!(view.text()[0], "गणना ७");
        // And the two members before it kept their whole types, so neither
        // half of a split type was mistaken for a member of its own.
        let angani = read_sthiti(&tokens, &Sthiti::new())
            .expect("reads")
            .expect("declared");
        assert_eq!(
            angani,
            vec![
                Anga {
                    nama: "अग्रिमः".into(),
                    prakara: "सम्भाव्य स्थानम् गण्डिका".into()
                },
                Anga {
                    nama: "चालकः".into(),
                    prakara: "वृत्तिस्थानम् आरभ्य अ३२ ऽ अ३२ समाप्तम् फलम् शून्यम्".into(),
                },
                Anga {
                    nama: "सङ्ख्या".into(),
                    prakara: A32.into()
                },
            ]
        );
    }

    /// A member may end with or without the trailing `ऽ`, exactly as a variant
    /// may: the sketch and every `संरचना` in the corpus write one, and
    /// `spec/grammar-t1.ebnf:1597` says `struct_body` is not frozen.
    #[test]
    fn the_last_member_may_or_may_not_be_followed_by_a_separator() {
        let no_trailing =
            format!("प्रकारः स्थितिः भवति संरचना आदि सङ्ख्याॱॱ अ३२ इति ॥\n{GHATANA_GANANA}{VIEW}");
        assert_eq!(
            read_module(&no_trailing).expect("reads"),
            view_at(0).expect("reads")
        );
    }
}
