//! **THE `.data` READER, SHARED BY THE TWO FILES THAT READ EMITTED SASSEMBLY.**
//!
//! `t1_emitted_labels.rs` built this to read ONE object at a time — the rungs'
//! own sources, compiled by the `.t1` chain, three states where a `String` had
//! been. `t1_corpus_globals.rs` runs the same reader over EVERY object the
//! corpus emits. **A SECOND HAND-WRITTEN COPY WOULD BE A COPY THAT CAN DRIFT
//! FROM THE ONE THE RUNGS TRUST AND STILL BE GREEN** — the same reason
//! `chain.rs`'s `CHAIN` margin gives for the manifest being `pub`: there is
//! exactly one answer in this tree to "what does `॥ वैश्विकम् ॥` mean".
//!
//! So this module holds the READING and no fixture, no chain and no assertion.
//! The callers own their own sources, their own loaders and their own claims.

// The two callers use different halves — `t1_emitted_labels.rs` needs the
// numeral printer for its derived directives, the corpus sweep does not — and
// each test crate compiles this file on its own, so an item unused by ONE of
// them is dead code in that crate. `hopladder/mod.rs:29` carries the same
// allow for the same reason.
#![allow(dead_code)]

/// A `.data` global's STORAGE, and it has THREE states rather than two.
///
/// `riscv64.rs:1429-1459` and its `.t1` twin `yantrotsarjana.t1:2293-2380` write
/// EVERY global with the same two opening lines and the same third directive:
///
/// ```text
///   ॥ वैश्विकम् कग ॥          ॥ वैश्विकम् कग ॥
///   कगॱॱ                       कगॱॱ
///   ॥ अष्टाष्टकाः ०षोड्७ ॥      ॥ अष्टाष्टकाः कगभण्डार ॥
/// ```
///
/// **THE LEFT WORD IS A VALUE AND THE RIGHT ONE IS AN ADDRESS, AND A READER
/// THAT STOPPED AT THAT LINE WOULD REPORT THEM AS THE SAME THING.** The first
/// version of this function did exactly that: it answered `("कग", "कगभण्डार")`
/// for a run global and called `कगभण्डार` an initialiser. `ADR-0013` is the rule
/// that separates them — in a data directive a token beginning with a LETTER is
/// a NAME and a name is the ADDRESS of what it names (`parse.rs:903-915` pushes
/// it onto `Datum.addresses` as a relocation) — so the discriminator is the
/// token's first character and nothing else, which is what [`is_data_literal`]
/// encodes.
///
/// So the three states, each one a thing the emitter can really be:
///
/// * [`Storage::Word`] — a scalar. The word IS the value.
/// * [`Storage::Run`] — a pointer word plus the `ॱरिक्त` block it names, WITH
///   THAT BLOCK'S SIZE. `W-293`.
/// * [`Storage::DanglingPointer`] — a pointer word whose storage this object
///   never places. Every read of that global would then resolve to nothing, and
///   **no length of an object can see it**: the pointer word is eight octets
///   whether or not the storage behind it exists, and `ॱरिक्त` is space in
///   memory and absent from the file, so the run's own 1,024 octets never
///   appear in an octet count either. A rung answering ११०७२ answers ११०७२.
pub fn data_globals(text: &str) -> Vec<(String, Storage)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for w in lines.windows(3) {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let decl: Vec<&str> = w[0].split_whitespace().collect();
        let init: Vec<&str> = w[2].split_whitespace().collect();
        let (label, word) = match (decl.as_slice(), init.as_slice()) {
            (["॥", "वैश्विकम्", label, "॥"], ["॥", "अष्टाष्टकाः", word, "॥"]) => {
                (*label, *word)
            }
            _ => continue,
        };
        // The middle line places the storage AT that label. Without this check a
        // routine whose first emitted line happened to be a data directive would
        // be read as a global.
        if w[1] != format!("{label}ॱॱ") {
            continue;
        }
        let storage = if is_data_literal(word) {
            Storage::Word(word.to_string())
        } else {
            match run_storage(&lines, word) {
                Some(octets) => Storage::Run {
                    store: word.to_string(),
                    octets,
                },
                None => Storage::DanglingPointer(word.to_string()),
            }
        };
        out.push((label.to_string(), storage));
    }
    out
}

/// What a global's data word holds. See [`data_globals`] for why this is three
/// variants and not a `String`.
#[derive(Debug, PartialEq, Eq)]
pub enum Storage {
    /// `॥ अष्टाष्टकाः ०षोड्७ ॥` — the initialiser itself.
    Word(String),
    /// `॥ अष्टाष्टकाः कगभण्डार ॥` plus the `ॱरिक्त` block that label names, and
    /// the block's size in octets.
    Run { store: String, octets: u64 },
    /// A pointer to storage this object never places.
    DanglingPointer(String),
}

/// `ADR-0013`: in a data directive a token that begins with a DIGIT is a
/// literal and one that begins with a letter is a NAME — the address of what it
/// names. Devanagari digits are `०`..`९`, U+0966..U+096F.
pub fn is_data_literal(token: &str) -> bool {
    matches!(token.chars().next(), Some(c) if ('\u{0966}'..='\u{096F}').contains(&c))
}

/// A Devanagari numeral as a number. `Python re \b does not work with
/// Devanagari` is this crate's standing warning about text tools; this one
/// reads code points and does not pattern-match at all.
pub fn devanagari_int(token: &str) -> Option<u64> {
    if token.is_empty() {
        return None;
    }
    let mut n: u64 = 0;
    for c in token.chars() {
        if !('\u{0966}'..='\u{096F}').contains(&c) {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(c as u64 - 0x0966)?;
    }
    Some(n)
}

/// A Devanagari numeral as a SIGNED number, `ऋण` for a negative
/// (`riscv64.rs:118` is the writer; `spec/grammar-t0.ebnf` `numeral`, ADR-0009).
///
/// [`devanagari_int`] answers `u64` and is the right reader for a SIZE and a
/// spill OFFSET, neither of which the emitter ever writes negative. An
/// IMMEDIATE is a different question: `lower_constant` materialises any value
/// in `-2048..=2047` as one `योगः <r>म् शून्यःन <k>न ।`, and the corpus writes
/// negative constants. A reader that used `devanagari_int` here would answer
/// `None` for `ऋण१` and the operand would fall back to
/// [`Origin::Written`] — a correctly lowered literal reported in the same bucket
/// as one this reader has not been taught, which is the two-states-for-three
/// failure this module keeps splitting.
///
/// `i128` and not `i64` because the IR compares `i128`s (`ir.rs:81`), and
/// because a reader that overflowed on a numeral would answer `None` — again
/// the correct-lowering bucket.
pub fn devanagari_signed(token: &str) -> Option<i128> {
    let (neg, digits) = match token.strip_prefix("ऋण") {
        Some(rest) => (true, rest),
        None => (false, token),
    };
    if digits.is_empty() {
        return None;
    }
    let mut n: i128 = 0;
    for c in digits.chars() {
        if !('\u{0966}'..='\u{096F}').contains(&c) {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(c as i128 - 0x0966)?;
    }
    Some(if neg { -n } else { n })
}

/// The size of the `ॱरिक्त` block a run global's pointer names, or `None` if
/// this object never places it.
///
/// The storage line is `{store}ॱॱ` and the directive after it is
/// `॥ स्थानम् {octets} ॥` — the SIZE, which is the one claim a pointer word
/// cannot carry and the reason this returns a number rather than a bool. A
/// storage truncated to eight octets is present, and a check that asked only
/// "is it there?" would score it green while every element past the first wrote
/// into whatever follows.
///
/// Matched by EQUALITY and not by prefix, because `{store}शीर्षॱॱ` — the length
/// header, emitted first — begins with `{store}` and a prefix match would stop
/// there and read the header's `८` as the storage's size.
///
/// **THE WHOLE OBJECT IS SEARCHED, AND THE FIRST VERSION OF THIS SEARCHED
/// FORWARD FROM THE POINTER ONLY.** `riscv64.rs:1433` places a run's storage
/// immediately after its pointer word, so on every object this tree emits the
/// two readings agree — which is exactly why the narrowness survived a cycle.
/// It was found by `t1_corpus_globals.rs`'s ALIASING control, which aims one
/// global's data word at ANOTHER global's block: that block is placed EARLIER
/// in the object, a forward search never reached it, and the reader answered
/// [`Storage::DanglingPointer`] for a storage sitting a hundred lines above.
/// Two globals sharing one block would have been reported as two defects of
/// the WRONG KIND — and a dangling pointer is the one this file was built to
/// find, so the instrument would have been loudest exactly where it was wrong.
///
/// The question a `.data` reader is actually asking is whether THIS OBJECT
/// places the storage at all. The assembler resolves a name against the whole
/// object and does not care where in it the label sits (`ADR-0013`), so
/// emission ORDER is not a property this reader may assume.
fn run_storage(lines: &[&str], store: &str) -> Option<u64> {
    let store_line = format!("{store}ॱॱ");
    let at = lines.iter().position(|l| *l == store_line)?;
    let size: Vec<&str> = lines.get(at + 1)?.split_whitespace().collect();
    match size.as_slice() {
        ["॥", "स्थानम्", n, "॥"] => devanagari_int(n),
        _ => None,
    }
}

/// A number as a Devanagari numeral — the inverse of [`devanagari_int`], used
/// to DERIVE the expected octet directive rather than quote it.
pub fn devanagari(n: u64) -> String {
    if n == 0 {
        return "०".to_string();
    }
    let mut digits = Vec::new();
    let mut n = n;
    while n > 0 {
        digits.push(char::from_u32(0x0966 + (n % 10) as u32).expect("a Devanagari digit"));
        n /= 10;
    }
    digits.iter().rev().collect()
}

// ── THE CONTROL-TRANSFER READER ─────────────────────────────────────────────
//
// `t1_emitted_labels.rs` built these to read the LADDER's seven objects and
// `t1_corpus_conditionals.rs` runs the same predicate over every object the
// corpus emits. They live here for the reason the module header gives: a
// second hand-written copy is a copy that can drift from the one the rungs
// trust and still be green. The margins below name `transfers`,
// `comparison_source` and `RungReading`, which are `t1_emitted_labels.rs`'s
// own items — the READING moved and the fixtures did not.

/// `पुनःस्थानम्` — the return-address register (`riscv64.rs:70`).
pub const RA: &str = "पुनःस्थानम्";
/// `शून्यः` — x0, hardwired to zero (`riscv64.rs:69`). **NOTHING WRITES IT AND
/// NOTHING CAN**: `लङ्घनम् शून्यःम् …` names it precisely to DISCARD the link.
/// See [`Origin::Zero`] for what that cost the first corpus-scale reading.
pub const ZERO: &str = "शून्यः";

/// **WHAT A TRANSFER DOES TO CONTROL, AND THERE ARE THREE ANSWERS.**
///
/// This enum carried TWO — `Call` and `Branch` — and `Branch` folded the
/// unconditional jump together with the conditional test. That fold is what the
/// ladder's table ran out of after `W-279`: with the dead edges gone, nothing in
/// `RungReading` could tell a routine that returns straight from one standing
/// behind a `यदि`, because both answer the same count of `Branch`. An instrument
/// with two states where the truth has three hides its own breakage.
///
/// See `transfers` for how the three are told apart, and why the CONDITIONAL
/// arm is the complement rather than a mnemonic list.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Kind {
    /// `लङ्घनम् पुनःस्थानम्म् …` — writes the return address.
    Call,
    /// `लङ्घनम् शून्यःम् …` — discards it. Control does NOT continue past it.
    Jump,
    /// Anything else that names a label. Control MAY continue past it.
    Conditional,
}

/// **THE ONE PLACE A LINE IS DECIDED TO BE A LABEL TRANSFER, AND WHAT KIND.**
///
/// Answers `(mnemonic, target, kind)` or nothing. Extracted from `transfers`
/// so that [`conditional_operand_slots`] takes the CONDITIONAL arm off exactly
/// the same predicate: two readers of the same class, each with its own copy of
/// the rule, is two readers that disagree the first time the rule moves — and
/// the disagreement shows up as a SHORTER operand list beside a full mnemonic
/// list, which reads like an emitter that placed no branch.
///
/// See `transfers` for why the unconditional form is the keyed arm and the
/// conditional one its complement.
pub fn label_transfer<'a>(f: &[&'a str]) -> Option<(&'a str, &'a str, Kind)> {
    if f.len() < 3 || *f.last().unwrap() != "।" {
        return None;
    }
    let target = f[f.len() - 2].strip_suffix("य्")?;
    let kind = match f[0] {
        "लङ्घनम्" if f[1] == format!("{RA}म्") => Kind::Call,
        "लङ्घनम्" => Kind::Jump,
        _ => Kind::Conditional,
    };
    Some((f[0], target, kind))
}

/// The stack pointer, as the emitter spells it. See `comparison_source`: a
/// parameter reaches the compare through its SPILL SLOT, never through the
/// argument register, so the slot is what names it.
pub const SP: &str = "स्तूपसूचकः";

/// **EVERY CONDITIONAL BRANCH WITH ITS TWO OPERANDS RESOLVED TO SPILL SLOTS.**
///
/// `<op>लङ्घनम् R(a)न R(b)त् <target>य् ।` — the करण is the subject and the
/// अपादान the standard (`yantrotsarjana.t1:1417`). Both are resolved back
/// through the last `आहारः <r>म् <SP>त् <off>न ।` that defined them, because the
/// register NUMBER is an allocation artefact: `व न्यूनम् द` and `द अधिकम् व` are
/// the same comparison and emit different register names, since the loads are
/// placed in source order. The SLOT is stable and the register is not.
///
/// The conditional arm is taken through [`label_transfer`] — the SAME predicate
/// `transfers` classifies on, the complement of `लङ्घनम्` — so this list is
/// exactly as long as `RungReading::conditional_mnemonics` BY CONSTRUCTION. A
/// branch nobody taught this reader is READ rather than dropped, and an operand
/// that resolves to no slot is `None` rather than a silent zero (`०` is a real
/// slot here; it is the FIRST parameter).
///
/// **AND A REGISTER THAT IS WRITTEN LOSES ITS SLOT.** The earlier form of this
/// reader recorded `आहारः` loads and never invalidated, so a register loaded
/// from slot `८` and then overwritten by a `योगः` materialising a literal still
/// answered `८` — the comparison fixture never reached that case because its
/// branch stands immediately after the two loads, and rungs 99, 100 and 101 all
/// compare against a materialised value. The rule is the कर्म: `f[1]` ending in
/// `म्` is the destination, and every write but a spill load leaves the register
/// holding a value that is in NO slot.
pub fn conditional_operand_slots(text: &str) -> Vec<(String, Option<u64>, Option<u64>)> {
    conditional_operands(text)
        .into_iter()
        .map(|c| (c.mnemonic, c.karana.slot(), c.apadana.slot()))
        .collect()
}

/// WHERE ONE OPERAND OF A CONDITIONAL CAME FROM — **FOUR STATES, NOT TWO.**
///
/// `conditional_operand_slots` answers `Option<u64>`, and over the ladder's
/// seven hand-built rungs that is enough: every operand there is either a
/// parameter's spill slot or a materialised literal, and the fixtures know
/// which. Over the corpus it is not. `ashtaka.t1:12-15` emits
///
/// ```text
///   आहारः स्थिर०म् क्षणिक०त् ०न ।      a load through a GLOBAL's pointer
///   योगः स्थिर१म् शून्यःन ०न ।          a materialised literal
///   समलङ्घनम् स्थिर०न स्थिर१त् …य् ।    and both operands answer `None`
/// ```
///
/// — and a third `None` would come from a register the object never defines at
/// all, which is a DEFECT, while those two are the emitter working correctly.
/// **A SWEEP THAT REPORTED ONE `None` COUNT WOULD BE REPORTING A DEFECT AND A
/// CORRECT LOWERING AS THE SAME NUMBER**, which is the two-states-for-three
/// failure `Kind` was split to end. So:
///
/// * [`Origin::Slot`] — resolved, through a spill load off `SP`.
/// * [`Origin::Zero`] — the operand is `शून्यः`, x0. **FULLY RESOLVED AND NOT A
///   SLOT**: the comparison is against the constant zero, which is the shape
///   every nil test and every boolean test takes.
/// * [`Origin::Literal`] — the operand is a MATERIALISED CONSTANT, written by
///   `योगः <r>म् शून्यःन <k>न ।` — `lower_constant`'s `addi rd, x0, k` form for
///   any value in `-2048..=2047`. **FULLY RESOLVED AND NOT A SLOT**, and it is
///   the state without which the `अधिकम्` exchange cannot be checked on the
///   corpus at all: 3,854 of 8,586 operands measured `written:योगः`, and
///   2,297 of 4,293 conditionals compare a slot against something else — the
///   shape `<name> अधिकम् <numeral>` takes. **THE ADDITION IS NOT A LITERAL:**
///   `योगः कम् खन गन ।` adds two registers and `योगः कम् खन ७न ।` adds an
///   immediate to a register; neither is a constant and both stay
///   [`Origin::Written`]. Only x0 as the source makes the destination a value
///   this reader knows.
/// * [`Origin::Written`] — the register was last written by THIS mnemonic, and
///   it is not a spill load. Correct, and it names what to teach the reader
///   next.
/// * [`Origin::Undefined`] — a register this routine never writes. Reaching a
///   compare is either an incoming argument read without its prologue spill or
///   a use before definition.
/// * [`Origin::Unreadable`] — the token is not of the operand shape at all
///   (`…न` for the करण, `…त्` for the अपादान). Named rather than dropped.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Origin {
    Slot(u64),
    Zero,
    Literal(i128),
    Written(String),
    Undefined,
    Unreadable(String),
}

impl Origin {
    /// The projection `conditional_operand_slots` answers with.
    pub fn slot(&self) -> Option<u64> {
        match self {
            Origin::Slot(s) => Some(*s),
            _ => None,
        }
    }

    /// The CONSTANT this operand is, or `None` if it is not one. `शून्यः` is
    /// the constant zero every bit as much as `योगः <r>म् शून्यःन ०न ।` is, and
    /// the two spellings are the same value — a reader that answered `None` for
    /// x0 would call `<name> अधिकम् ०` unresolved on the one shape the corpus
    /// writes most.
    pub fn literal(&self) -> Option<i128> {
        match self {
            Origin::Literal(k) => Some(*k),
            Origin::Zero => Some(0),
            _ => None,
        }
    }

    /// A one-word class name, for grouping a corpus-scale finding by WHY the
    /// operand did not resolve rather than by how many did not.
    pub fn class(&self) -> String {
        match self {
            Origin::Slot(_) => "slot".to_string(),
            Origin::Zero => "zero".to_string(),
            Origin::Literal(_) => "literal".to_string(),
            Origin::Written(m) => format!("written:{m}"),
            Origin::Undefined => "undefined".to_string(),
            Origin::Unreadable(t) => format!("unreadable:{t}"),
        }
    }
}

/// One conditional branch, with BOTH operands traced back to where they came
/// from and the label it would take. See [`Origin`] and
/// [`conditional_operand_slots`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ConditionalOperands {
    pub mnemonic: String,
    pub target: String,
    pub karana: Origin,
    pub apadana: Origin,
    /// The routine the branch sits in — the last `॥ वैश्विकम् X ॥` above it.
    /// Empty only for a branch before any export, which no object emits.
    pub routine: String,
}

/// **THE WALK.** One pass, tracking what last wrote each register.
///
/// **AND THE TRACKING IS RESET AT EVERY ROUTINE**, which the seven-rung
/// fixtures could not have shown: each of them is ONE routine, so a slot
/// learned in one function and read in the next never arose. `riscv64.rs:519`
/// exports every routine label, so `॥ वैश्विकम् X ॥` is the boundary — and
/// without the reset a corpus object attributes `स्थिर०`'s slot from routine A
/// to a compare in routine Z, which is a WRONG SLOT reported with the same
/// confidence as a right one.
pub fn conditional_operands(text: &str) -> Vec<ConditionalOperands> {
    let mut origin_of: Vec<(String, Origin)> = Vec::new();
    let mut routine = String::new();
    let mut out = Vec::new();
    for l in text.lines() {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = l.split_whitespace().collect();
        // A NEW ROUTINE (or a global) — nothing a register held survives it.
        if let ["॥", "वैश्विकम्", name, "॥"] = f.as_slice() {
            origin_of.clear();
            routine = (*name).to_string();
            continue;
        }
        if let Some((mnemonic, target, Kind::Conditional)) = label_transfer(&f) {
            let trace = |o: Option<&&str>, suffix: &str| match o {
                None => Origin::Unreadable(String::new()),
                Some(tok) => match tok.strip_suffix(suffix) {
                    None => Origin::Unreadable((*tok).to_string()),
                    Some(ZERO) => Origin::Zero,
                    Some(reg) => origin_of
                        .iter()
                        .find(|(n, _)| n == reg)
                        .map(|(_, o)| o.clone())
                        .unwrap_or(Origin::Undefined),
                },
            };
            out.push(ConditionalOperands {
                mnemonic: mnemonic.to_string(),
                target: target.to_string(),
                karana: trace(f.get(1), "न"),
                apadana: trace(f.get(2), "त्"),
                routine: routine.clone(),
            });
            continue;
        }
        // NOT A CONDITIONAL, SO IT MAY WRITE A REGISTER — including the two
        // unconditional forms of `लङ्घनम्`, whose कर्म is the link register.
        let Some(dst) = f.get(1).and_then(|d| d.strip_suffix("म्")) else {
            continue;
        };
        // **x0 IS NOT A DESTINATION**, however a line spells it. `लङ्घनम् शून्यःम्`
        // names it to throw the link away, and a reader that recorded that as a
        // write answered `written:लङ्घनम्` for `शून्यः` in the 322 routines that
        // jump before they compare and `undefined` in the 21 that do not — one
        // register, two answers, neither of them what it is.
        if dst == ZERO {
            continue;
        }
        origin_of.retain(|(r, _)| r != dst);
        let origin = match f.as_slice() {
            ["आहारः", _, base, off, "।"]
                if *base == format!("{SP}त्")
                    && off.strip_suffix('न').and_then(devanagari_int).is_some() =>
            {
                Origin::Slot(off.strip_suffix('न').and_then(devanagari_int).unwrap())
            }
            // **A MATERIALISED CONSTANT, AND ONLY WHEN THE SOURCE IS x0.**
            // `lower_constant` writes `योगः <r>म् शून्यःन <k>न ।` — an `addi`
            // off the hardwired zero — for every value that fits the immediate.
            // `योगः कम् खन गन ।` is an ADDITION of two registers and
            // `योगः कम् खन ७न ।` an addition of an immediate to a register;
            // the destination of either holds a value this reader does not
            // know, so both stay `Written`. Reading the second as a literal `७`
            // would answer a WRONG CONSTANT with exactly the confidence of a
            // right one — which is the failure the `अधिकम्` column exists to
            // catch, manufactured by the column itself.
            ["योगः", _, src, imm, "।"]
                if src.strip_suffix('न') == Some(ZERO)
                    && imm.strip_suffix('न').and_then(devanagari_signed).is_some() =>
            {
                Origin::Literal(imm.strip_suffix('न').and_then(devanagari_signed).unwrap())
            }
            _ => Origin::Written(f[0].to_string()),
        };
        origin_of.push((dst.to_string(), origin));
    }
    out
}

/// **WHICH REGION OF A FRAME AN OFFSET OFF `स्तूपसूचकः` FALLS IN.**
///
/// [`Origin::Slot`] answers an OFFSET, and an offset alone says nothing about
/// what it addresses. `riscv64.rs:397-408` lays the frame out in four regions —
/// spills at `8k`, then `W-245`'s locals at `8(num_spills + k)`, then the saved
/// `स्थिर`s at their recorded offsets, then `पुनःस्थानम्` at `bytes - 8` — and a
/// sweep that reads a slot number without asking which region it landed in is
/// reporting a spilled temporary and a declared `चरः` as the same kind of
/// thing.
///
/// **THE POINT OF THE ENUM IS [`SlotBand::Unaccounted`].** The other five arms
/// are regions the frame declares; the sixth is the honest answer for an offset
/// that belongs to none of them — the 16-octet rounding pad, or a reader that
/// has the wrong frame entirely. An offset nothing accounts for is exactly the
/// answer a census must not fold into a neighbour, because folding it turns a
/// reader bug into a plausible-looking count.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SlotBand {
    /// The allocator's own spill region. NOT a source word: a spilled temporary
    /// has no name and its number says nothing about declaration order.
    Spill(u64),
    /// `W-245`'s local region: slot `k` is a parameter or a `चरः`, by
    /// first-declaration order.
    Local(u64),
    /// A saved `स्थिर`, restored by the epilogue.
    Saved,
    /// `पुनःस्थानम्`.
    ReturnAddress,
    /// The ९th and later incoming arguments, which live in the CALLER's frame.
    IncomingArgument,
    /// Inside the frame and in none of the regions above — the 16-octet
    /// rounding pad, or a reader that has the wrong frame. **NAMED, because an
    /// offset nothing accounts for is the one answer a census must not fold
    /// into a neighbour.**
    Unaccounted,
}

impl SlotBand {
    /// The local slot this offset names, or `None` — the projection the
    /// `W-245` local table is read with.
    #[must_use]
    pub fn local(self) -> Option<u64> {
        match self {
            SlotBand::Local(k) => Some(k),
            _ => None,
        }
    }

    /// A one-word class name, for a census keyed on WHY rather than on how
    /// many.
    #[must_use]
    pub fn class(self) -> &'static str {
        match self {
            SlotBand::Spill(_) => "spill",
            SlotBand::Local(_) => "local",
            SlotBand::Saved => "saved",
            SlotBand::ReturnAddress => "return-address",
            SlotBand::IncomingArgument => "incoming-argument",
            SlotBand::Unaccounted => "unaccounted",
        }
    }
}

/// Classify one offset off `स्तूपसूचकः` against the routine's own frame.
///
/// **THE ORDER OF THE TESTS IS THE FRAME'S OWN ORDER AND IS NOT ARBITRARY.**
/// `ra_offset` is `bytes - 8` (`riscv64.rs:402`), so it is BELOW `bytes` and the
/// `IncomingArgument` test — which asks for `>= bytes` — cannot swallow it. An
/// offset that is not a multiple of eight is answered `Unaccounted` before any
/// region is consulted, because every region is addressed in whole words and a
/// misaligned offset means the reader, not the frame, is wrong.
#[must_use]
pub fn slot_band(off: u64, frame: &sadhana::t1::riscv64::Frame) -> SlotBand {
    let k = off / 8;
    let (spills, locals) = (frame.num_spills as u64, frame.num_locals as u64);
    if !off.is_multiple_of(8) {
        return SlotBand::Unaccounted;
    }
    if k < spills {
        return SlotBand::Spill(k);
    }
    if k < spills + locals {
        return SlotBand::Local(k - spills);
    }
    let signed = off as i64;
    if signed >= frame.bytes {
        return SlotBand::IncomingArgument;
    }
    if signed == frame.ra_offset {
        return SlotBand::ReturnAddress;
    }
    if frame.saved.iter().any(|(_, at)| *at == signed) {
        return SlotBand::Saved;
    }
    SlotBand::Unaccounted
}
