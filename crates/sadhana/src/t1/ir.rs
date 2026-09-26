use crate::t1::ast::*;
use std::collections::HashMap;

// `Ord` is here for REPRODUCIBILITY, not for convenience. `regalloc` sorts
// live intervals to assign registers, and a sort by start point alone leaves
// equal-start values in whatever order they came out of a `HashMap` — which
// Rust randomises per process. So the same source produced a different
// register assignment on different runs. Ordering `ValueId` lets the sort key
// be total, which makes the assignment a function of the IR alone (`W-144`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

#[derive(Debug, Clone)]
pub enum Instruction {
    /// Loads a constant integer
    ConstInt(i64),
    /// Function call — `(callee, arguments)`, and the `ValueId` it is paired
    /// with in a block is the RESULT at the call site. `W-204`.
    ///
    /// The ट loop of `research/22-two-conduits.md` §7: व (this site, which
    /// control returns to) → र (the arguments, out) → ट् ⇢ ट (the callee's
    /// entry) → त (its body) → व् ⇢ व (`Return`, its inverse). A call is an
    /// INSTRUCTION inside a block and not a terminator, because control
    /// comes back to the site — the shapes that would make it one (a tail
    /// call, an exception) the grammar has not got.
    ///
    /// Declared here long before anything built it. `build_expression` still
    /// cannot: `ast::Expression` has no call (`ast.rs:46` defers it), so on
    /// this side the variant leads the builder, as `CondBranch` does. The
    /// `.t1` twin's `अभिव्यञ्जकरचना` is the one that lowers a call, refusing
    /// a callee the resolver wrote no symbol onto; the shape is pinned here
    /// by hand, as `W-198` pinned the conditional branch.
    Call(SymbolId, Vec<ValueId>),
    /// Addition
    Add(ValueId, ValueId),
    /// Subtraction
    Sub(ValueId, ValueId),
    /// Parameter fetch
    Param(usize),
    /// `W-245` — THE OPERATORS THE GRAMMAR FROZE (ADR-0032's `mul_op`,
    /// `shift_op`, `and_op`, `or_op`, `xor_op`), one kind each, AS THE ISA HAS
    /// THEM: `गुणनम्`, `भागः`, `शेषः`, `वामसरणम्`, `दक्षिणसरणम्`, `युक्तम्`,
    /// `विकल्पः`, `वैषम्यम्` (`spec/mnemonics-riscv64.src.tsv`). Multiply,
    /// divide and remainder are the M extension: `सङ्केतन` encodes them and
    /// `yantra` does not execute them (`crates/yantra/src/lib.rs:59`), so a
    /// program that REACHES one halts `Unimplemented` naming the word — counted
    /// by the census, never silently rounded to a neighbour (the refusal
    /// `ir.t1`'s द्विकर्म margin kept for two cycles).
    ///
    /// **CORRECTED `W-294`: `yantra` HAS EXECUTED THESE SINCE 2026-09-05, AND THE
    /// PARAGRAPH ABOVE CITES THE LINE THAT SAYS SO.** `lib.rs:59-62` reads *"It
    /// gained the **M extension** on 2026-09-05 — the eight multiply, divide and
    /// remainder forms at funct7 `0x01` — so it is RV64IMA rather than RV64IA"*.
    /// The citation was correct when written and the line moved underneath it to
    /// say the opposite.
    ///
    /// This is not academic: the index lowering multiplies an index by an element
    /// width on every run element access, and a local `अ८` slice stores and reads
    /// back today. **A margin claiming the machine cannot multiply, cited to the
    /// line announcing that it can, would have talked the next reader out of a
    /// lowering that works.**
    ///
    /// `Shr` is the LOGICAL shift `दक्षिणसरणम्` (`srl`), the lexicon twin of
    /// the operator `दक्षिणसृ`; the interpreter shifts its `i128` arithmetically,
    /// and the two differ only on a negative `अ६४` — said here, not found in a
    /// twin diff.
    Mul(ValueId, ValueId),
    Div(ValueId, ValueId),
    Rem(ValueId, ValueId),
    Shl(ValueId, ValueId),
    Shr(ValueId, ValueId),
    And(ValueId, ValueId),
    Or(ValueId, ValueId),
    Xor(ValueId, ValueId),
    /// `W-245` — A COMPARISON, `(condition, subject, standard)`: ADR-0008's six
    /// conditions, and the value is १ when `subject <op> standard` holds, ० when
    /// not. The grammar's five `compare_op`s lower to four of the six —
    /// `समम्` Eq, `असमम्` Ne, `न्यूनम्` Lt, `बृहत्समम्` Ge — and `अधिकम्` is
    /// `Lt` with its operands exchanged; the SIGNED pair is chosen because the
    /// reference interpreter compares `i128`s (`nirvahana.rs`, `binop`). `Ltu`
    /// and `Geu` are the ISA's other two, declared so the six are six; the
    /// builder writes neither until a type reaches it.
    ///
    /// Used ONCE, by the `CondBranch` that ends its block, it never becomes a
    /// value: `riscv64::lower_cond_branch` fuses the pair to the condition's
    /// branch instruction (`<op>लङ्घनम् R(a)न R(b)त् …`, करण the subject,
    /// अपादान the standard). Used as a value it is `slt`/`sltu` or a
    /// `sub`+`sltiu` pair (§2.5 of research/25, amended by this row).
    Cmp(CmpOp, ValueId, ValueId),
    /// `W-245` — A LOCAL READ: the value in frame slot `k`. A routine's `चरः`
    /// and its parameters live in FRAME SLOTS, not in SSA values, because the
    /// corpus assigns them inside `यावत्` bodies and a value that changes on
    /// a back-edge needs a join (a phi) the IR has not got; a slot needs
    /// nothing the emitter does not already have — `Location::Spill(k)` is
    /// addressed at `स्तूपसूचकः + 8k` today, and a local is the same traffic
    /// one region up the frame (`riscv64::Frame::num_locals`). The Rust builder
    /// cannot build one — `ast.rs` has no `Let` and no assignment — so on this
    /// side the variant leads the builder, as `Call` and `CondBranch` did.
    Load(usize),
    /// `W-245` — A LOCAL WRITE: `(slot, value)`. Defines a value nobody reads
    /// (the pair's `ValueId` is a placeholder), and is a DCE root because the
    /// next `Load` of the slot depends on it.
    Store(usize, ValueId),
    /// A MODULE-LEVEL GLOBAL READ: the word held at the global's own label.
    ///
    /// `W-278`, 2026-09-06. The global is named by its `SymbolId`, so the label
    /// is `routine_label`'s `{module}{name}` — the SAME scheme a `Call` uses,
    /// and for the same reason: THE STORAGE MUST BE ONE OBJECT ACROSS THE WHOLE
    /// IMAGE. `parse.t1` writes `वास्तुॱअभिव्यञ्जककोश` and `vastu.t1` reads it;
    /// if each module emitted its own copy of the slot the write would land in
    /// one and the read in another, and every cross-module global would silently
    /// answer its initialiser. The declaring module emits the storage with an
    /// exported label and everyone else references it by name.
    ///
    /// UNIFORM, AND THAT IS THE POINT. It lowers a read of ANY scalar global,
    /// written or not. The obvious optimisation — a global nothing assigns is a
    /// constant, so fold it to `ConstInt` — WAS PROPOSED AND WITHDRAWN, because
    /// it cannot be decided where the decision has to be made: `कार्यक्रमरचना`
    /// walks `व्याकरॱघोषणाकोश`, the declarations of ONE SOURCE, so a write
    /// performed by another module is invisible to it. And there is no escape
    /// hatch: all 541 module-level globals in the corpus are `सार्वजनिक` and
    /// none is module-private, so no candidate can be proven unwritten from its
    /// own source. Eleven qualified cross-module writes exist today
    /// (`parse.t1`'s `वास्तुॱअभिव्यञ्जकसूचकाङ्क भवति …` and the rest) — they are
    /// not an edge case to guard, they are proof the property is not locally
    /// decidable. Folding wrongly would freeze a global at its initialiser and
    /// every read would answer ०: a WRONG ANSWER, not a crash, green in every
    /// test that does not exercise a cross-module write.
    ///
    /// AN ARENA GLOBAL IS NOT THIS. Its value is an ADDRESS, not a word, so it
    /// stays stubbed under its own cause rather than folded in here.
    LoadGlobal(SymbolId),
    /// A RECORD FIELD READ: `(base, byte offset)` — one word from `base +
    /// offset`, 2026-09-06.
    ///
    /// THE FIRST INSTRUCTION IN THIS SET WHOSE BASE IS A RUNTIME VALUE.
    /// `Load(k)`/`Store(k, …)` address a SLOT: the emitter writes
    /// `आहारः rd, स्तूपसूचकः, 8*(spills+k)`, with the stack pointer hardcoded as
    /// the base and the offset computed at emit time. A record is reached
    /// through a reference held in a value, so its base cannot be known then —
    /// which is why no field access could be lowered at all, and why this is a
    /// new variant rather than a reuse of `Load`.
    ///
    /// The offset comes from [`crate::t1::types::StructLayout`]: fields one word
    /// each in declaration order, records addressed by reference. Both halves of
    /// that rule are provisional pending the owner's ruling on the data model.
    ///
    /// A READ ONLY. Writing a field is a separate shape and a separate cause in
    /// the census (`assign_field`, 350 sites against `field`'s 1,185), so it
    /// lands with its own measurement rather than riding in on this one.
    ///
    /// ॥ FOR THE `.t1` SIDE, WHICH LANDS LATER — THE TRAP AND THE KIND ॥
    ///
    /// Written here BEFORE that half exists, because the code will show the
    /// choice and nothing would show the REASON: the next reader sees two
    /// nearly identical offset fields on one record, tidies them into one, and
    /// reintroduces a defect that stays invisible until someone extends an
    /// unrelated pass.
    ///
    /// THE TWO FIELDS MUST NEVER BE MERGED. `मध्यरूपॱआज्ञा` already has
    /// `स्थानक्रम`, documented as "Load's and Store's FRAME SLOT" — a slot
    /// INDEX. A field read's offset is BYTES FROM A RECORD'S BASE. They are
    /// different units of different things and only look alike.
    ///
    /// WHAT GOES WRONG IF THEY ARE MERGED, and it is not hypothetical:
    /// `yantrotsarjana.t1`'s `यन्त्रस्थानीयगणना` sizes a routine's frame as
    /// `max(स्थानक्रम + १)` over every instruction whose kind is
    /// `आहाराज्ञाभेद` or `निधानाज्ञाभेद`. Feed it a byte offset and a field at
    /// offset 16 claims SEVENTEEN LOCALS. Today the new kind is not in that
    /// set, so nothing breaks — the damage waits for whoever extends the pass,
    /// which is the longest possible gap between cause and symptom.
    ///
    /// SO, on the `.t1` side:
    ///   ॱ the base goes in `वाम`, a `मूल्याङ्क` — already Store's value and
    ///     every binary's left operand, i.e. a VALUE operand, which is what a
    ///     base is;
    ///   ॱ the offset goes in a NEW field of `संरचना आज्ञा`, documented as
    ///     BYTES, never in `स्थानक्रम`;
    ///   ॱ and this kind is DELIBERATELY EXCLUDED from `यन्त्रस्थानीयगणना`'s
    ///     slot-bearing set: it addresses a record, not the frame, so it
    ///     contributes no local.
    /// That is why this variant carries `(ValueId, u64)` — two things in two
    /// fields — rather than one number doing both jobs.
    ///
    /// ITS INSTRUCTION KIND IS NOT RESERVED HERE, AND THIS MARGIN ONCE SAID IT
    /// WAS. It read "its instruction kind is १८", written while `ConstStr` was
    /// claiming १७ on an unlanded branch. THAT IS NOT A THING THE NUMBERING
    /// SUPPORTS: `t1_transcriptions.rs`'s `the_kind_table_scan_is_not_vacuous`
    /// asserts `ir.t1`'s kinds are CONTIGUOUS १..=N, so a number reserved by
    /// prose and declared nowhere leaves a hole the guard refuses — which it
    /// did, the moment a second instruction tried to land around it. १८ went to
    /// `LoadGlobal`, whose `.t1` half exists; a kind is claimed by being
    /// DECLARED in `ir.t1`, in the order things land, and this one takes the
    /// next free number when its own `.t1` half arrives.
    ///
    /// The collision that reservation was guarding against is still real and
    /// still worth checking for: two instructions sharing a number surface as a
    /// corrupt decode in whichever binary runs first, never as a merge
    /// conflict, because git cannot see that class of collision.
    ///
    /// AND THE KIND MUST BE ADDED TO ALL FOUR DECODERS IN THE SAME LANDING.
    /// Four copies of that table exist — `sadhana/src/t1/chain.rs`,
    /// `yantra/tests/paradigm_encode.rs`, `sadhana-t1/tests/t1_exec_riscv.rs`
    /// and `pradarshana/src/pathana.rs` — each refusing an unknown kind.
    ///
    /// THEY ARE GUARDED, and a partial landing is therefore impossible rather
    /// than merely unwise: `sadhana-t1/tests/t1_transcriptions.rs`'s
    /// `every_copy_of_a_kind_table_decodes_what_ir_t1_defines` compares `ir.t1`'s
    /// declarations against every transcription and names each copy that is
    /// behind, the missing kind by number AND by Rust variant, and the row count
    /// each copy has. Ship three decoders and the tree goes red at the fourth,
    /// by name — not later, as a corrupt decode in an unrelated binary.
    ///
    /// I FIRST WROTE HERE THAT THEY "AGREE BY EVERYONE HAVING REMEMBERED, NOT BY
    /// ANY GUARD", AND THAT WAS FALSE. I had inventoried the four copies and
    /// correctly found no guard AMONG them — the guard lives outside, in another
    /// crate's test directory, watching them. AN INVENTORY OF A MECHANISM DOES
    /// NOT FIND THE THING THAT WATCHES THE MECHANISM FROM OUTSIDE, and "I looked
    /// where it would be" is not the same as "it is not there".
    ///
    /// Adding a field to `संरचना आज्ञा` also moves the ADR-0030 conservation
    /// ledger. Take the new triple FROM THE FAILING ASSERTION; never compute
    /// it.
    ///
    /// NOTHING BUILDS ONE YET, deliberately: the `.t1` builder is what would,
    /// and `measure_corpus_twin_emit` asserts the two emitters never diverge on
    /// an IR-built source — so a shape only `riscv64.rs` could emit would go red
    /// the moment `ir.t1` built it. This side lands first and waits, as `Call`
    /// and `CondBranch` did.
    LoadField(ValueId, u64),
    /// A RUN'S ELEMENT — `आधारः अङ्कः सूचकः अन्तः`, instruction kind २०.
    ///
    /// THE SECOND OPERAND IS A BYTE OFFSET, NOT AN ELEMENT INDEX, and a reader
    /// meeting `LoadIndex(base, index)` will assume otherwise unless told. The
    /// scaling by the element's width is done by `Mul` instructions the builder
    /// emits, because an index is a VALUE where `LoadField`'s offset is a `u64`
    /// decided at build time — which is the whole reason this is not that.
    ///
    /// THE LOAD WAS ONE WORD, ALWAYS — **and since `W-294` the third field says
    /// how wide it is, in OCTETS.** The old sentence read: *"a run of anything
    /// narrower cannot use this and `ir.t1` refuses it rather than scaling by १
    /// and reading seven octets past each element. A narrower load is a kind this
    /// IR has not got."* The diagnosis was exactly right and is why the refusal
    /// stood; the conclusion was wrong on one point. **A narrower load is not a
    /// new KIND.** The T0 load family is already width-selected — `आहारःॱअ८`
    /// against `आहारःॱअ६४` — so what was missing was a width on this variant and
    /// a suffix at the emitter, not an instruction.
    ///
    /// EIGHT IS THE ONLY VALUE THAT EMITS THE BARE `आहारः`, character for
    /// character as before, which is what keeps every existing image
    /// octet-identical under `measure_corpus_twin_emit`.
    ///
    /// THIS VARIANT AND ITS `.t1` KIND ARRIVED TOGETHER, which is what makes it
    /// unlike the two above it: `LoadGlobal` and `LoadField` were already here
    /// when their `.t1` halves landed, so their margins argue only about WHICH
    /// number. There was no index instruction on either side, and
    /// `every_copy_of_a_kind_table_decodes_what_ir_t1_defines` derives from
    /// `ir.t1` and refuses a partial landing by name — so the pair is mandatory
    /// rather than tidy.
    LoadIndex(ValueId, ValueId, u64),
    /// `W-283` — THE ADDRESS OF A GLOBAL, as a value. `research/28`, ruled
    /// 2026-09-11.
    ///
    /// `LoadGlobal`, `LoadField` and `LoadIndex` differ only in how they form an
    /// address, and each then does one load. Completing that grid for writes
    /// wants eight kinds — three address forms times two directions, with only
    /// the arithmetic varying. **That small a difference between kinds means the
    /// kinds are the wrong unit**, which is what the ruling turned on: form the
    /// address as a value, then load or store AT it. Five kinds replace eight.
    ///
    /// The census keeps its resolution because the three address-formation kinds
    /// stay distinct — `field_base_not_a_name`, `index_declined` and the
    /// `name_global_*` causes are all statements about formation, and a single
    /// undifferentiated address kind would have erased that permanently.
    AddrOfGlobal(SymbolId),
    /// `W-283` — the word at an address held in a value. `आहारः rd, addr त् 0`.
    LoadAt(ValueId),
    /// `W-283` — store a value at an address held in a value.
    /// `निधानम् addr य् 0, src`.
    ///
    /// THE BASE'S KARAKA DIFFERS FROM THE LOAD'S AND IT IS NOT COSMETIC: a load
    /// reads FROM its base (`त्`, अपादान), a store writes TO it (`य्`,
    /// अधिकरण). Copying the load's roles is refused by the assembler — two
    /// operands claiming अपादान — which is loud, and paid once.
    ///
    /// **The silent failure is `is_side_effecting` below.** A store kind missing
    /// from it is deleted as dead code with no diagnostic, so the acceptance
    /// test for this variant asserts a write SURVIVES OPTIMISATION rather than
    /// that it emits.
    StoreAt(ValueId, ValueId),

    /// A RECORD ALLOCATION — `चरः X ॱॱ <a struct> भवति ० ।`, kind २४. `W-284`.
    ///
    /// The `u64` is the SIZE IN OCTETS, `WORD × field count`, fixed at build
    /// time by the owner's data-model ruling: a record is by reference, one
    /// word per field.
    ///
    /// IT EXISTS BECAUSE THE TWO HALVES DISAGREED ABOUT `भवति ०` ON A RECORD.
    /// `nirvahana.rs`'s `zero_at` answers `Value::Record` with every field
    /// zeroed; this file had no record concept at all and lowered the same
    /// source to `ConstInt(0)` — a NULL POINTER. The corpus builds every record
    /// that way, so the compiled half was constructing at address zero, and
    /// only never noticed because the field write was a stub and therefore a
    /// no-op. The moment `AddrOfField` below made the write real it surfaced as
    /// `addr: 0`, which is why the two land together.
    ///
    /// A STATIC BUMP REGION, NOT A FRAME, AND THAT WAS DECIDED BY MEASUREMENT:
    /// 38 corpus routines RETURN a struct type, so a record built in a frame
    /// dangles in every one of them. `riscv64.rs` lays the region and its
    /// cursor once per IMAGE, in the startup — not per object, or the linker
    /// refuses the second definition by name.
    ///
    /// NO OPERANDS: the size is a constant, so `operands()` answers empty and
    /// the liveness pass needs NO arm — the same reason `LoadGlobal` has none.
    /// An absent arm is correct here and a defect for a kind with operands; the
    /// two look identical in that routine, which is why this says so.
    AllocRecord(u64),

    /// THE ADDRESS OF A RECORD FIELD — `(base, offset in OCTETS)`, kind २५.
    /// `W-284`, and the third of the ruled model's three address formers.
    ///
    /// The offset is a BUILD-TIME CONSTANT, as `LoadField`'s is and unlike
    /// `AddrOfIndex`'s: a field's position is known from its declaration and an
    /// index's is not. That asymmetry is the one thing a decoder gets wrong —
    /// reading `ध्रुवमूल्यम्` for the index kind's second operand makes every
    /// index address offset zero, in silence.
    ///
    /// IT IS DELIBERATELY EXCLUDED FROM `count_locals`'s slot-bearing set: it
    /// addresses a record, not the frame, so it contributes no local. That is
    /// why the offset is a separate field rather than a number doing double
    /// duty in `स्थानक्रम`.
    AddrOfField(ValueId, u64),

    /// THE ADDRESS OF AN ARRAY ELEMENT — `(base, index)`, kind २६. `W-284`.
    ///
    /// BOTH operands are values, and the element width is applied here rather
    /// than by the caller: `base + index × WORD`. It is the write half of
    /// `LoadIndex` २०, which already scales the same way.
    ///
    /// **THE WIDTH IS `WORD` AND THAT IS AN ASSUMPTION THIS KIND MAKES, NOT A
    /// FACT IT CHECKS.** `LoadIndex`'s `.t1` builder reads `विस्तार` and refuses
    /// a width it does not emit a load for — it fails CLOSED. Nothing here can:
    /// the element type is not carried on this side. So a narrower element would
    /// be addressed at the wrong stride and no diagnostic would say so. The
    /// refusal that protects it lives in `ir.t1`'s arm, which declines anything
    /// but a word-wide element, and this margin exists so that whoever relaxes
    /// that refusal knows what it was holding up.
    AddrOfIndex(ValueId, ValueId),
    /// `W-254` — A STRING LITERAL'S OCTETS, and the value is their ADDRESS.
    ///
    /// THE VALUE IS ONE WORD, WHICH IS WHY THIS FITS. `artha.t1:679` types a
    /// string literal as a slice of unsigned octets, and a slice is ONE
    /// address-sized word in this ABI (`abi.rs:80-86`, ADR-0026) rather than a
    /// pointer/length pair — so the instruction defines a single `ValueId` like
    /// every other, and no calling convention, register pair or second result
    /// had to be invented for it. A consumer that wants the length reads it
    /// from the type, not from a second register.
    ///
    /// THE OCTETS TRAVEL IN THE INSTRUCTION AND THE POOL IS THE EMITTER'S,
    /// mirroring `ConstInt`. A big integer is not held in a module-level data
    /// table either: `ConstInt` carries its value and `riscv64::lower_constant`
    /// pools what it cannot materialise inline. This is the same arrangement
    /// with a wider payload, and it is why the IR still has no data section —
    /// adding one would put the emitter's business in the IR.
    ///
    /// Its lowering is `lower_constant`'s pooled arm with the LOAD dropped:
    /// `auipc` then `addi` against the pooled label, so what lands in the
    /// register is where the octets are rather than the first eight of them.
    ///
    /// The Rust builder cannot build one — `ast.rs` has no string expression —
    /// so on this side the variant leads the builder, as `Call`, `CondBranch`
    /// and `Load` did before it. `ir.t1`'s `अभिव्यञ्जकरचना` is what lowers a
    /// real one, and the shape is pinned here by hand.
    ConstStr(Vec<u8>),
}

/// The six branch conditions of ADR-0008, in that ADR's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    /// `beq` — `समलङ्घनम्`
    Eq,
    /// `bne` — `विषमलङ्घनम्`
    Ne,
    /// `blt` — `न्यूनलङ्घनम्`
    Lt,
    /// `bge` — `अन्यूनलङ्घनम्`
    Ge,
    /// `bltu` — `अचिह्नन्यूनलङ्घनम्`
    Ltu,
    /// `bgeu` — `अचिह्नान्यूनलङ्घनम्`
    Geu,
}

impl Instruction {
    /// The values an instruction READS, in operand order — one place for every
    /// reader that walks operands (`opt::verify`, DCE, the allocator, the
    /// emitter's use count), so a kind added here is seen by all of them.
    #[must_use]
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Instruction::Call(_, args) => args.clone(),
            Instruction::Add(l, r)
            | Instruction::Sub(l, r)
            | Instruction::Mul(l, r)
            | Instruction::Div(l, r)
            | Instruction::Rem(l, r)
            | Instruction::Shl(l, r)
            | Instruction::Shr(l, r)
            | Instruction::And(l, r)
            | Instruction::Or(l, r)
            | Instruction::Xor(l, r)
            | Instruction::Cmp(_, l, r) => vec![*l, *r],
            Instruction::Store(_, v) => vec![*v],
            // The base is a real use: DCE must not drop what computed the
            // reference this reads through.
            Instruction::LoadField(b, _) => vec![*b],
            // BOTH are real uses, and the offset one is the easy half to get
            // wrong: it is a VALUE here, not a constant, so DCE dropping what
            // computed it would leave a load through an undefined register.
            // The width is a build-time constant, not a value, so it is not a
            // use and DCE has nothing to keep alive for it — the same reason
            // `LoadField`'s offset is `_` one arm up.
            Instruction::LoadIndex(b, o, _) => vec![*b, *o],
            // `W-283`. The address is a real use in both: DCE dropping what
            // formed it leaves a load or a store through an undefined register.
            Instruction::LoadAt(a) => vec![*a],
            // BOTH are uses — the address AND the value written. Listing only
            // the address would let DCE delete whatever computed the value while
            // keeping the store that writes it.
            Instruction::StoreAt(a, v) => vec![*a, *v],
            // `W-284`. The base is a real use in both: DCE dropping what formed
            // it leaves an address computed from an undefined register. The
            // field's offset is a constant and so is not an operand; the
            // index's IS a value and must be listed, which is the same trap
            // `LoadIndex` names two arms up.
            Instruction::AddrOfField(b, _) => vec![*b],
            Instruction::AddrOfIndex(b, i) => vec![*b, *i],
            Instruction::ConstInt(_)
            | Instruction::ConstStr(_)
            | Instruction::Param(_)
            // A global read takes no value operand: its address is a label.
            | Instruction::LoadGlobal(_)
            // Nor does forming that address — same reason, one step earlier.
            | Instruction::AddrOfGlobal(_)
            // Nor an allocation: its size is a build-time constant. `W-284`.
            | Instruction::AllocRecord(_)
            | Instruction::Load(_) => Vec::new(),
        }
    }

    /// Whether the instruction must survive DCE whatever reads its value: a
    /// `Call` for its effects, a `Store` because a later `Load` reads the slot.
    ///
    /// **EVERY STORE KIND MUST APPEAR HERE, AND OMITTING ONE FAILS SILENTLY.**
    /// This is a hand-written list, not an exhaustive `match`, so a new writing
    /// instruction is simply absent from it and DCE deletes every one as dead
    /// code — the program still compiles, assembles and runs, and drops the
    /// writes. There is no diagnostic anywhere on that path.
    ///
    /// So a test that a write EMITS cannot catch it: the emission is real and
    /// the deletion happens afterwards. **The acceptance test for any store kind
    /// must assert the write SURVIVES OPTIMISATION**, and must land in the same
    /// commit as the kind — see `a_store_at_survives_dead_code_elimination`.
    #[must_use]
    pub fn is_side_effecting(&self) -> bool {
        matches!(
            self,
            Instruction::Call(..) | Instruction::Store(..) | Instruction::StoreAt(..)
        )
    }
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Return(Option<ValueId>),
    Branch(BlockId),
    Unreachable,
    /// Conditional branch — `(condition, then_block, else_block)`. `W-198`.
    ///
    /// The one terminator a language with `यदि` and `यावत्` cannot lower
    /// without (`research/22-two-conduits.md` §4.3): `Return` and `Branch`
    /// can express call-and-return, not a marker returning into its own run.
    /// Added here and in `ir.t1` (`शाखावसानभेद`, tag ४ — both twins already
    /// had THREE terminators, not the two the row counted) in the same commit,
    /// so the twins stay twins.
    ///
    /// Rule X3: both targets name blocks of the SAME function; a back-edge
    /// targets a point inside its own span, and there is no jump-to-anywhere.
    /// A `यदि` with no `अन्यथा` is a `CondBranch` whose else-target IS the
    /// join, never a jump out.
    ///
    /// `build_statement` below cannot construct it yet: `ast::Statement` has
    /// no `If` or `While` (`ast.rs:45` defers them), so on this side the
    /// variant leads the builder, exactly as the `.t1` twin now leads this
    /// file. The `.t1` builder is the one that lowers both statements.
    CondBranch(ValueId, BlockId, BlockId),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub insts: Vec<(ValueId, Instruction)>,
    pub terminator: Option<Terminator>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: SymbolId,
    pub blocks: HashMap<BlockId, Block>,
    pub entry_block: BlockId,
}

pub struct IrBuilder {
    next_val: usize,
    next_block: usize,
    pub functions: Vec<Function>,
    /// `W-237` — THE KIND OF A NAME. Every routine the program declares, with
    /// the symbol `build_program` gives it (its 1-based declaration order;
    /// `SymbolId(0)` stays "no symbol", as ० does in the `.t1` twin). This is
    /// the Rust side's reading of `artha.t1`'s `संज्ञाभेदकोश` (`W-228`): a
    /// bare name in value position whose symbol is a ROUTINE is a call with
    /// no arguments — `प्रत्यागमनम् आरम्भः ।` — and a name that is not (a
    /// `चरः`, a parameter) is still the Identifier stub below. The parser
    /// gives a zero-argument call no node of its own on either side, so the
    /// kind of the symbol is the only thing that tells the two apart.
    pub routines: HashMap<String, SymbolId>,
}

/// `W-274`: `new()` takes no arguments, so `Default` is the same
/// constructor under the name the language expects. Written rather than
/// allowed, because `clippy::new_without_default` is asking for an
/// interface and not for silence.
impl Default for IrBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IrBuilder {
    pub fn new() -> Self {
        Self {
            next_val: 0,
            next_block: 0,
            functions: Vec::new(),
            routines: HashMap::new(),
        }
    }

    fn new_value(&mut self) -> ValueId {
        let id = ValueId(self.next_val);
        self.next_val += 1;
        id
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    pub fn build_program(&mut self, program: &Program) {
        // PASS ONE — `W-237`: declare every routine before any body is lowered,
        // as the resolver's pass one does (`artha.t1`, `कार्यक्रमनिर्णयः`), so a
        // body that names a routine declared AFTER it still lowers to a call.
        for decl in &program.declarations {
            if let Declaration::Function { name, .. } = decl {
                let sym = SymbolId(self.routines.len() + 1);
                self.routines.entry(name.clone()).or_insert(sym);
            }
        }
        for decl in &program.declarations {
            if let Declaration::Function {
                name,
                params,
                return_type: _,
                body,
            } = decl
            {
                // The routine's symbol is the one pass one gave it, which is
                // also what a zero-argument call to it names (`W-237`); until
                // then this was `SymbolId(0)` "simulated", and no call could
                // have named it.
                let func_sym = self.routines[name];

                let entry = self.new_block();
                let mut current_block = Block {
                    id: entry,
                    insts: Vec::new(),
                    terminator: None,
                };

                // Generate parameters
                for (i, _) in params.iter().enumerate() {
                    let p_val = self.new_value();
                    current_block.insts.push((p_val, Instruction::Param(i)));
                }

                if let Some(b) = body {
                    let ret_val = self.build_statement(b, &mut current_block);
                    // Add terminator
                    current_block.terminator = Some(Terminator::Return(ret_val));
                } else {
                    current_block.terminator = Some(Terminator::Return(None));
                }

                let mut blocks = HashMap::new();
                blocks.insert(entry, current_block);

                self.functions.push(Function {
                    name: func_sym,
                    blocks,
                    entry_block: entry,
                });
            }
        }
    }

    fn build_statement(&mut self, stmt: &Statement, block: &mut Block) -> Option<ValueId> {
        match stmt {
            Statement::Expression(expr) => Some(self.build_expression(expr, block)),
            Statement::Block(stmts) => {
                let mut last = None;
                for s in stmts {
                    last = self.build_statement(s, block);
                }
                last
            }
        }
    }

    fn build_expression(&mut self, expr: &Expression, block: &mut Block) -> ValueId {
        match expr {
            Expression::Numeral(text) => {
                let val = text.parse::<i64>().unwrap_or(0);
                let v = self.new_value();
                block.insts.push((v, Instruction::ConstInt(val)));
                v
            }
            Expression::Identifier(name) => {
                let v = self.new_value();
                // `W-237`: a ROUTINE's name in value position is a call with no
                // arguments — the parser writes no call node for `आरम्भः ।`
                // alone, so the symbol's kind is what decides (see `routines`).
                // The `.t1` twin reads `अर्थॱसंज्ञाभेदकोश` at the node's symbol
                // and asks for kind १; this side asks the table pass one filled.
                if let Some(sym) = self.routines.get(name) {
                    block.insts.push((v, Instruction::Call(*sym, Vec::new())));
                } else {
                    // Stub: returning 0 for unmapped identifiers
                    block.insts.push((v, Instruction::ConstInt(0)));
                }
                v
            }
            Expression::Group(inner) => self.build_expression(inner, block),
            _ => {
                // Stub everything else to 0
                let v = self.new_value();
                block.insts.push((v, Instruction::ConstInt(0)));
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_function() {
        let mut builder = IrBuilder::new();
        let program = Program {
            declarations: vec![Declaration::Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                body: Some(Statement::Expression(Expression::Numeral("42".into()))),
            }],
        };

        builder.build_program(&program);
        assert_eq!(builder.functions.len(), 1);

        let func = &builder.functions[0];
        let entry = &func.blocks[&func.entry_block];

        // Should have one instruction: ConstInt(42)
        assert_eq!(entry.insts.len(), 1);
        if let Instruction::ConstInt(val) = entry.insts[0].1 {
            assert_eq!(val, 42);
        } else {
            panic!("Expected ConstInt");
        }

        // Should have terminator: Return
        assert!(matches!(
            entry.terminator,
            Some(Terminator::Return(Some(_)))
        ));
    }

    /// `W-237`: `प्रत्यागमनम् आरम्भः ।` — a ROUTINE's bare name in value
    /// position is a `Call` with no arguments, naming the routine's symbol;
    /// `प्रत्यागमनम् क ।` with `क` no routine is not a call. The parser gives a
    /// zero-argument call no node, so the symbol's KIND decides (`W-228`'s
    /// `संज्ञाभेदकोश` on the `.t1` side, `routines` here). The callee is
    /// declared AFTER its caller on purpose: pass one must see it first.
    #[test]
    fn a_routine_name_in_value_position_is_a_call_with_no_arguments_and_a_variable_is_not() {
        let routine = |name: &str, body: Expression| Declaration::Function {
            name: name.into(),
            params: vec![],
            return_type: None,
            body: Some(Statement::Expression(body)),
        };
        let mut builder = IrBuilder::new();
        builder.build_program(&Program {
            declarations: vec![
                routine("परीक्षा", Expression::Identifier("आरम्भः".into())),
                routine("अन्या", Expression::Identifier("क".into())),
                routine("आरम्भः", Expression::Numeral("0".into())),
            ],
        });
        assert_eq!(builder.functions.len(), 3);
        let entry = |i: usize| &builder.functions[i].blocks[&builder.functions[i].entry_block];

        // `प्रत्यागमनम् आरम्भः ।` — one instruction, a Call with no arguments,
        // naming आरम्भः's symbol (the third declaration: SymbolId(3)), and the
        // return carries the call's value.
        let start = builder.routines["आरम्भः"];
        assert_eq!(start, SymbolId(3), "symbols are 1-based declaration order");
        assert_eq!(
            builder.functions[2].name, start,
            "the routine carries its own symbol"
        );
        let calls = entry(0);
        assert_eq!(calls.insts.len(), 1, "{:?}", calls.insts);
        let (v, inst) = &calls.insts[0];
        match inst {
            Instruction::Call(sym, args) => {
                assert_eq!(
                    *sym, start,
                    "the call names the routine, not a fresh symbol"
                );
                assert!(
                    args.is_empty(),
                    "no arguments were written, so none are passed"
                );
            }
            other => panic!("a routine name in value position lowered to {other:?}, not a Call"),
        }
        assert!(matches!(calls.terminator, Some(Terminator::Return(Some(r))) if r == *v));

        // `प्रत्यागमनम् क ।` — क names no routine, so it is the Identifier stub.
        let variable = entry(1);
        assert_eq!(variable.insts.len(), 1);
        assert!(
            matches!(variable.insts[0].1, Instruction::ConstInt(0)),
            "a name that is not a routine is not a call: {:?}",
            variable.insts[0].1
        );
        assert!(!builder.routines.contains_key("क"));
    }

    /// `W-198`: a `CondBranch`'s two targets are blocks of the same function —
    /// Rule X3 stated on the Rust twin, by hand, because this side has no `If`
    /// to lower yet and the variant would otherwise be declared and pinned by
    /// nothing. The shape is `यदि` with no `अन्यथा`: the else-target IS the
    /// join.
    #[test]
    fn a_cond_branch_targets_two_blocks_of_its_own_function() {
        let mut builder = IrBuilder::new();
        let entry = builder.new_block();
        let then = builder.new_block();
        let join = builder.new_block();
        let cond = builder.new_value();
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![(cond, Instruction::ConstInt(1))],
                terminator: Some(Terminator::CondBranch(cond, then, join)),
            },
        );
        blocks.insert(
            then,
            Block {
                id: then,
                insts: Vec::new(),
                terminator: Some(Terminator::Branch(join)),
            },
        );
        blocks.insert(
            join,
            Block {
                id: join,
                insts: Vec::new(),
                terminator: Some(Terminator::Return(None)),
            },
        );
        let func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: entry,
        };

        for block in func.blocks.values() {
            let targets: Vec<BlockId> = match block.terminator {
                Some(Terminator::CondBranch(_, t, e)) => vec![t, e],
                Some(Terminator::Branch(t)) => vec![t],
                _ => Vec::new(),
            };
            for t in targets {
                assert!(
                    func.blocks.contains_key(&t),
                    "block {:?} targets {t:?}, which is not a block of this function",
                    block.id
                );
            }
        }
        let Some(Terminator::CondBranch(_, t, e)) = func.blocks[&entry].terminator else {
            panic!("the entry ends in a CondBranch");
        };
        assert_ne!(t, e, "then and else differ");
        assert_eq!(e, join, "with no else, the else-target is the join");
    }

    /// `W-204`: a `Call` is an instruction INSIDE a block — its arguments are
    /// values defined before it in the same function, it defines a value of
    /// its own at the site, and the block still ends in its own terminator,
    /// because control comes back. Stated on the Rust twin by hand, because
    /// this side has no call expression to lower yet and the variant would
    /// otherwise be declared and pinned by nothing. The shape is the `.t1`
    /// twin's `ग आरभ्य ३ समाप्तम्`: one `ConstInt`, one `Call` with that
    /// value as its one argument, and a `Return` carrying the call's result.
    #[test]
    fn a_call_is_an_instruction_whose_arguments_precede_it_and_whose_result_returns_to_the_site() {
        let mut builder = IrBuilder::new();
        let entry = builder.new_block();
        let three = builder.new_value();
        let result = builder.new_value();
        let callee = SymbolId(7);
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![
                    (three, Instruction::ConstInt(3)),
                    (result, Instruction::Call(callee, vec![three])),
                ],
                terminator: Some(Terminator::Return(Some(result))),
            },
        );
        let func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: entry,
        };

        let block = &func.blocks[&entry];
        // र before ट: every argument of a call is a value the block defined
        // before the call.
        let mut defined: Vec<ValueId> = Vec::new();
        let mut calls = 0;
        for (v, inst) in &block.insts {
            if let Instruction::Call(sym, args) = inst {
                calls += 1;
                assert_eq!(*sym, callee, "the call names its callee's symbol");
                for a in args {
                    assert!(
                        defined.contains(a),
                        "argument {a:?} of the call is not a value defined before it"
                    );
                }
                assert!(
                    !defined.contains(v),
                    "the call defines a NEW value at the site, not one already defined"
                );
            }
            defined.push(*v);
        }
        assert_eq!(calls, 1);
        // व: control returns to the site, so the block still has its own
        // terminator, and it carries the call's result.
        let Some(Terminator::Return(Some(ret))) = block.terminator else {
            panic!("a block holding a call still ends in its own terminator — a call is not one");
        };
        assert_eq!(ret, result, "the return carries the value the call defined");
    }

    /// `W-245`: `यावत् क्रमः न्यूनम् सीमा` as the `.t1` builder lays it down —
    /// the local `क्रमः` is a frame SLOT: `Store` at its `चरः`, `Load` at every
    /// read, and the condition is a `Cmp` that ends the condition block, read
    /// once, by the `CondBranch`. Pinned on the Rust twin by hand, because
    /// this side has no `Let`, no assignment and no operator to lower yet.
    #[test]
    fn a_local_is_a_slot_stored_then_loaded_and_a_compare_used_once_ends_its_block() {
        let (entry, cond, body, exit) = (BlockId(0), BlockId(1), BlockId(2), BlockId(3));
        let (zero, s0, i, lim, c, one, i2, s1) = (
            ValueId(0),
            ValueId(1),
            ValueId(2),
            ValueId(3),
            ValueId(4),
            ValueId(5),
            ValueId(6),
            ValueId(7),
        );
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![
                    (zero, Instruction::ConstInt(0)),
                    (s0, Instruction::Store(0, zero)),
                ],
                terminator: Some(Terminator::Branch(cond)),
            },
        );
        blocks.insert(
            cond,
            Block {
                id: cond,
                insts: vec![
                    (i, Instruction::Load(0)),
                    (lim, Instruction::ConstInt(5)),
                    (c, Instruction::Cmp(CmpOp::Lt, i, lim)),
                ],
                terminator: Some(Terminator::CondBranch(c, body, exit)),
            },
        );
        blocks.insert(
            body,
            Block {
                id: body,
                insts: vec![
                    (one, Instruction::ConstInt(1)),
                    (i2, Instruction::Add(i, one)),
                    (s1, Instruction::Store(0, i2)),
                ],
                terminator: Some(Terminator::Branch(cond)),
            },
        );
        blocks.insert(
            exit,
            Block {
                id: exit,
                insts: Vec::new(),
                terminator: Some(Terminator::Return(Some(i))),
            },
        );
        let func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: entry,
        };

        // Every operand of every instruction is a value defined somewhere in
        // the function — `operands()` is the one list every reader walks.
        let defined: Vec<ValueId> = func
            .blocks
            .values()
            .flat_map(|b| b.insts.iter().map(|(v, _)| *v))
            .collect();
        for b in func.blocks.values() {
            for (_, inst) in &b.insts {
                for o in inst.operands() {
                    assert!(
                        defined.contains(&o),
                        "{inst:?} reads {o:?}, defined by nothing"
                    );
                }
            }
        }
        // A Store is a root; a Load and a Cmp are not.
        assert!(Instruction::Store(0, zero).is_side_effecting());
        assert!(!Instruction::Load(0).is_side_effecting());
        assert!(!Instruction::Cmp(CmpOp::Eq, i, lim).is_side_effecting());
        assert_eq!(Instruction::Store(0, zero).operands(), vec![zero]);
        assert_eq!(Instruction::Load(0).operands(), Vec::<ValueId>::new());
        // The compare is the LAST instruction of its block and the branch reads
        // it — the shape `riscv64::lower_cond_branch` fuses.
        let cb = &func.blocks[&cond];
        let Some(Terminator::CondBranch(read, _, _)) = cb.terminator else {
            panic!("the condition block ends in a CondBranch");
        };
        assert_eq!(cb.insts.last().map(|(v, _)| *v), Some(read));
        assert!(
            matches!(cb.insts.last(), Some((_, Instruction::Cmp(CmpOp::Lt, a, b))) if *a == i && *b == lim)
        );
        // Six conditions, ADR-0008's, and no seventh.
        let six = [
            CmpOp::Eq,
            CmpOp::Ne,
            CmpOp::Lt,
            CmpOp::Ge,
            CmpOp::Ltu,
            CmpOp::Geu,
        ];
        assert_eq!(six.len(), 6);
    }
}
