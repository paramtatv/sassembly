//! THE CHAIN, FROM LIBRARY CODE: a `.t1` source to an emittable module.
//!
//! **Why this module exists.** The T1 front end — the lexer, the parser, the
//! resolver, the checker and the IR builder — is written IN T1 and runs under
//! `nirvahana`, and every path through it lived inside a test:
//! `crates/yantra/tests/paradigm_encode.rs` drives the whole chain and asserts
//! about it. So the project could measure the chain and could not USE it. This
//! module is the driving part of that test, lifted to library code and with the
//! census's counters left behind: what remains is what a compiler needs.
//!
//! **Why not the Rust front end.** `t1::parse`, `t1::resolve`, `t1::typecheck`
//! and `t1::ir` are the Rust twins, and they are a skeleton: `ast::Statement`
//! has two variants, `Expression` and `Block`, with the margin "Variable,
//! return, if, etc. deferred for now", so `प्रत्यागमनम् २ योगः ३ ।` builds an
//! EMPTY routine — measured, not assumed: the Rust chain compiles that source
//! to zero IR instructions and the image halts 0. The `.t1` sources are the
//! implementation (`ir.t1:546` lowers `योगः` and `वियोगः`), which is why the
//! census drives them and why this does.
//!
//! **What is carried and what is not.** The front end is EMBEDDED — seven `.t1`
//! sources, ~700 KiB, compiled in — so the binary is one file and needs no
//! `--t1-root`. And it is self-contained in the one respect this margin denied
//! until 2026-10-01: NO front-end source EXECUTES `समावेशः` — all 14 mentions
//! across the seven are comments or string literals (`lex.t1:682` and kin
//! IMPLEMENT the construct for the programs they compile) — so the root the
//! caller states is never read on this path and the chain loads and emits with
//! no `spec/` on disk at all. Measured corpus-wide, not argued:
//! `crates/sadhana-t1/tests/d003_t1_emit_is_path_blind.rs` pins it, and the
//! tables a compiled source embeds travel inside `sarani.t1`'s store (the
//! margin on `CHAIN` below). The sentence that stood here — *"four of the
//! seven embed `spec/*.tsv`, resolved at LOAD time"* — was stale.
//!
//! **One source.** [`Front::module`] reads ONE program's arenas. Nothing here
//! collects a compilation set or links another module's object; a call to a
//! routine the source does not declare becomes an undefined symbol that the
//! LINKER refuses by name, which is the honest stop.

use crate::t1::ast::SymbolId;
use crate::t1::ir::{Block, BlockId, CmpOp, FloatOp, Function, Instruction, Terminator, ValueId};
use crate::t1::nirvahana::{Interpreter, Octets, Value, kernel_member_of, kernel_name_reserved};
use crate::t1::riscv64::{self, Module, Names};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// The T1 front end, in load order-free order (`W-224`): the lexer, the AST,
/// the parser, the text helper, the declaration store, the resolver and the IR
/// builder. The two emitter sources (`utsarjana.t1`, `yantrotsarjana.t1`) are
/// NOT here: this drives the T1 front end and then hands the IR to the RUST
/// emitter, whose twin agrees with the T1 one octet for octet (`W-236`).
const FRONT_END: &[(&str, &str)] = &[
    ("lex.t1", include_str!("../../../sadhana-t1/src/lex.t1")),
    ("ast.t1", include_str!("../../../sadhana-t1/src/ast.t1")),
    ("parse.t1", include_str!("../../../sadhana-t1/src/parse.t1")),
    (
        "sanskrit_text.t1",
        include_str!("../../../sadhana-t1/src/sanskrit_text.t1"),
    ),
    (
        "sanchaya.t1",
        include_str!("../../../sadhana-t1/src/sanchaya.t1"),
    ),
    ("artha.t1", include_str!("../../../sadhana-t1/src/artha.t1")),
    ("ir.t1", include_str!("../../../sadhana-t1/src/ir.t1")),
];

/// **THE WHOLE CHAIN, AND THE REASON THIS LIST IS PART OF THE DRIVER'S UNIT
/// RATHER THAN PAPERWORK AFTER IT.** A `.t1` module exists only if a Rust
/// loader lists it: `artha.t1` and `sanchaya.t1` were once never in one image,
/// and an API audit over each of them passed while saying nothing about
/// whether they could co-reside.
///
/// `शृङ्खला` is the TOP of the dependency graph — it calls `पदविभाग`,
/// `व्याकर`, `अर्थ`, `मध्यरूप` and `यन्त्रोत्सर्जन` — so it cannot live in
/// [`FRONT_END`], which is deliberately the front HALF and which every
/// stage-at-a-time caller loads. Putting it there both inverted the meaning of
/// that list and made every `Front::load` pay for an emitter it never calls.
///
/// **The absence is not hypothetical and the failure is not the one you would
/// expect.** With `yantrotsarjana.t1` off this list the driver does not fail to
/// find a routine at run time — `व्याकर` cannot resolve the qualified name at
/// PARSE time, falls back to the one-argument index form, and refuses
/// `shrinkhala.t1:105` with *expected `समाप्तम्`, found `ऽ`*. A missing module
/// reads as a syntax error in the module that called it, which is why the test
/// below drives the chain instead of asserting this list contains some strings.
///
/// `pub` because it is a MANIFEST, not an implementation detail: `sadhana-t1`'s
/// own driver tests load from it, so there is exactly one answer in the tree to
/// "which modules make a chain". A second hand-written list beside this one
/// would be a list that can drift from the shipped one and still be green.
pub const CHAIN: &[(&str, &str)] = &[
    ("lex.t1", include_str!("../../../sadhana-t1/src/lex.t1")),
    ("ast.t1", include_str!("../../../sadhana-t1/src/ast.t1")),
    ("parse.t1", include_str!("../../../sadhana-t1/src/parse.t1")),
    (
        "sanskrit_text.t1",
        include_str!("../../../sadhana-t1/src/sanskrit_text.t1"),
    ),
    (
        "sanchaya.t1",
        include_str!("../../../sadhana-t1/src/sanchaya.t1"),
    ),
    ("artha.t1", include_str!("../../../sadhana-t1/src/artha.t1")),
    ("vastu.t1", include_str!("../../../sadhana-t1/src/vastu.t1")),
    ("ir.t1", include_str!("../../../sadhana-t1/src/ir.t1")),
    (
        "utsarjana.t1",
        include_str!("../../../sadhana-t1/src/utsarjana.t1"),
    ),
    (
        "yantrotsarjana.t1",
        include_str!("../../../sadhana-t1/src/yantrotsarjana.t1"),
    ),
    (
        "vishlesana.t1",
        include_str!("../../../sadhana-t1/src/vishlesana.t1"),
    ),
    (
        "ashtaka.t1",
        include_str!("../../../sadhana-t1/src/ashtaka.t1"),
    ),
    (
        "encode.t1",
        include_str!("../../../sadhana-t1/src/encode.t1"),
    ),
    (
        "vakyavibhaga.t1",
        include_str!("../../../sadhana-t1/src/vakyavibhaga.t1"),
    ),
    // `निदान` — THE DIAGNOSTIC MODULE, AND THE FAILURE MODE WAS THAT THE FAILURE
    // MODE WAS BROKEN. `encode.t1:818` imports it and `encode.t1:1495` calls
    // `निदानॱविवरणम्` from `सङ्केतनदोषवचनम्`, which has eight callers in
    // `encode.t1` and one in `utsarjana.t1` — all loaded. This list did not carry
    // `nidana.t1`, so the production chain held a REACHABLE call into a module
    // that was not there.
    //
    // IT NEVER BIT BECAUSE IT IS THE ERROR PATH. That routine builds the
    // encoder's diagnostic, so a program that encodes never reaches it; only a
    // program that FAILS to encode would, and it would meet an unresolved call
    // instead of the message explaining what it got wrong. Nothing in the corpus
    // exercises it, which is why no test went red for as long as this was true.
    //
    // AND NO GUARD COULD HAVE CAUGHT IT, WHICH IS THE LARGER POINT.
    // `no_module_is_reached_without_being_imported` checks REACH ⇒ IMPORT, and
    // `encode.t1` imports `निदान` correctly, so it passes — truthfully. The
    // missing invariant is IMPORT ⇒ LOADED, and it is the first one that would
    // cross the `.t1`/Rust boundary: every guard in the tree lives on one side,
    // comparing `.t1` against `.t1` or Rust against Rust. That is why a hole
    // sitting in the open for this long was invisible from both sides at once.
    //
    // The closure terminates here: `nidana.t1` imports `वाक्यविभाग` and
    // `सङ्केतन`, both already above. Adding it pulls in nothing further, and the
    // `encode` ↔ `nidana` cycle is not a loader problem because names resolve at
    // CALL time, not at load.
    (
        "nidana.t1",
        include_str!("../../../sadhana-t1/src/nidana.t1"),
    ),
    // THE BACK HALF, added for `W-279`. Until now this list ended at the
    // EMITTER, so `शृङ्खला` could produce Sassembly text and nothing in the
    // image it was loaded into could turn that text into an object, link it,
    // or write it out. `कोश` writes the ELF image and `संयोजन` links objects;
    // both were landed, tested and unreachable from the chain the driver walks.
    //
    // THIS IS THE SMALLER HALF OF THE ROW AND IT IS FIRST FOR A REASON. Loading
    // a module makes it REACHABLE; it does not make the driver CALL it. But the
    // order matters: name `कोशॱ…` in `शृङ्खला` before this list carries
    // `kosha.t1`, and `व्याकर` cannot resolve the qualified name, falls back to
    // the one-argument index form, and refuses a line that is CORRECT — the
    // trap the margin above describes. List first, call second, and it never
    // arms.
    // `समावेशसारणी` — THE SPEC-TABLE STORE, and it is listed HERE for the reason the
    // margin above gives: list first, call second. `शृङ्खलाॱसङ्कलनारम्भः` calls
    // `समावेशसारणीॱसारणीपूरणम्` as its first statement, and this list is what both the
    // interpreted driver and `t1_image` walk — the native image's "20 sources
    // loaded" is this array's length, not a directory listing. Unlisted, the call
    // would resolve nowhere and the self-image would lose its tables again, which
    // is the defect the store lookup landed to fix.
    //
    // It imports `पदविभाग` alone, which is above it, and nothing imports it but
    // the driver, so it pulls in nothing and closes nothing.
    //
    // IT IS 866 KB OF GENERATED SOURCE — by far the largest member, and every
    // consumer of this list now lexes it. `Interpreter::load` is NOT lazy: pass
    // one lexes and reads the head of every source here. So the cost is real and
    // the row filed the chain half as "a second unit because it puts 1.27 MB of
    // source into every chain load".
    //
    // MEASURED 2026-09-17, AND THE OBJECTION DOES NOT HOLD. The figure is 866 KB,
    // not 1.27 MB, and bytes are the wrong unit anyway — load cost tracks TOKENS,
    // and this file's bulk is base-64 literals that lex to ONE token each:
    //
    //     source          bytes    tokens   bytes/token
    //     sarani.t1     866,293     5,843       148
    //     encode.t1     562,932    16,755        33
    //     ir.t1         457,635    13,850        33
    //     ast.t1         21,663       446        48   (the next highest ratio)
    //     ── whole chain (19 sources) ──
    //                 3,989,630   111,200        36
    //
    // `sarani.t1` is 21.7% of the chain's bytes and 5.3% of its tokens, at 4.5x
    // the ratio of any other member. A stopwatch cannot see it at all: nine
    // alternating `Interpreter::load` runs against an empty spec root read
    // 596-1,744 ms WITH this file and 850-1,934 ms WITHOUT it — the minimum
    // reading is LOWER with it than without, so the run-to-run spread on this
    // volume swamps whatever it costs. The token count is the honest instrument
    // and the wall clock is reported here only to say that it is not one.
    (
        "sarani.t1",
        include_str!("../../../sadhana-t1/src/sarani.t1"),
    ),
    ("kosha.t1", include_str!("../../../sadhana-t1/src/kosha.t1")),
    (
        "samyojana.t1",
        include_str!("../../../sadhana-t1/src/samyojana.t1"),
    ),
    (
        "shrinkhala.t1",
        include_str!("../../../sadhana-t1/src/shrinkhala.t1"),
    ),
];

/// Fuel: a bound on interpreter steps, not a timeout. The corpus's own censuses
/// use these numbers for these calls.
/// THE ROUTINES THE IR BUILDS, by symbol: `(the ir.t1 global that holds the
/// symbol, the symbol, the routine's name)`. `W-295`, generalised by `N-004`.
///
/// A built routine has no declaration, so `वृत्तिनामचिह्नककोश` holds ० for it
/// and there is no token to read the name from — both emitters spell it as a
/// literal instead. The `.t1` half's is at `shrinkhala.t1:440`; **these are the
/// same strings and the twin check is what keeps them so**, since the two
/// halves meet as a LABEL and a disagreement here would link a call in one
/// object against a definition in another that does not answer to the same
/// name.
///
/// IT WAS ONE CONSTANT, AND EVERY TOKEN-० ROUTINE GOT IT. With one built
/// routine per module (`खण्डवृद्धिः`) that was the same thing as naming it by
/// its symbol; a second built routine (`N-004`'s shared run compare,
/// `खण्डसाम्यम्`) would have been labelled `खण्डवृद्धिः` too and refused by the
/// emitter as a `LabelCollision`. So the name is now looked up by the symbol
/// the IR already wrote into the routine's `नाम`, and a token-० routine whose
/// symbol is in no row here is REFUSED by name rather than guessed.
///
/// The symbols are written twice — here and as the `ir.t1` globals the first
/// column names — and
/// `every_built_routine_symbol_agrees_with_the_global_ir_t1_declares` is what
/// keeps the two equal.
///
/// `V-009` (ii) added the module's two matrix kernel routines the same way,
/// named by their built-ins' members (`nirvahana.rs`'s `MATRIX_PRODUCT_MEMBER`
/// and `MATRIX_TRANSPOSE_MEMBER`) and numbered `१००००००६`/`१००००००७` — which is
/// why the run compare, approved at `१००००००६` against a tree that predated
/// them, takes `१००००००८`, the first number neither tree uses.
#[allow(clippy::cast_possible_wrap)]
pub const BUILT_ROUTINES: &[(&str, i128, &str)] = &[
    ("वृद्धिवृत्तिसंज्ञा", 10_000_005, "खण्डवृद्धिः"),
    (
        "आव्यूहवृत्तिसंज्ञा",
        riscv64::MATRIX_PRODUCT_SYMBOL.0 as i128,
        crate::t1::nirvahana::MATRIX_PRODUCT_MEMBER,
    ),
    (
        "व्युत्क्रमवृत्तिसंज्ञा",
        riscv64::MATRIX_TRANSPOSE_SYMBOL.0 as i128,
        crate::t1::nirvahana::MATRIX_TRANSPOSE_MEMBER,
    ),
    ("साम्यवृत्तिसंज्ञा", 10_000_008, "खण्डसाम्यम्"),
];

/// The name of the built routine `symbol` stands for, if any (`BUILT_ROUTINES`).
/// `paradigm_encode`'s own reader of the IR arenas names built routines by
/// this too, so the two readers cannot disagree on the rule.
#[must_use]
pub fn built_routine_name(symbol: i128) -> Option<&'static str> {
    BUILT_ROUTINES
        .iter()
        .find(|(_, s, _)| *s == symbol)
        .map(|(_, _, name)| *name)
}

const FUEL_LEX: u64 = 2_000_000_000;
const FUEL_PARSE: u64 = 4_000_000_000;
const FUEL_RESOLVE: u64 = 4_000_000_000;
const FUEL_SMALL: u64 = 5_000_000;

/// The front end, loaded once, driven one stage at a time so a caller can
/// report each stage's result and name the stage that stopped.
pub struct Front {
    it: Interpreter,
    tokens: i128,
    declarations: i128,
    resolver: Option<Value>,
}

impl Front {
    /// Load the seven sources, resolving their embeds against `spec_root`.
    ///
    /// # Errors
    /// The loader's own message — a source that does not parse, a global whose
    /// initialiser does not evaluate, or an embed the root does not hold.
    pub fn load(spec_root: &Path) -> Result<Self, String> {
        let it = Interpreter::load(FRONT_END, spec_root).map_err(|e| {
            format!(
                "the T1 front end does not load against `{}`: {}",
                spec_root.display(),
                e.reason
            )
        })?;
        Ok(Self {
            it,
            tokens: 0,
            declarations: 0,
            resolver: None,
        })
    }

    /// `अक्षरकोशॱपरिधिपदविभाग` — the source to tokens; how many. That routine
    /// is the ONE gated source entry (W-304): R-15-1 over the octets, then
    /// `पदविभागॱपदविभाग`. A refusal answers ० tokens and [`refusal_site`] names
    /// the octet. The gate is `.t1`, not Rust: this names the routine and nothing
    /// else, so the product needs no Rust to refuse a foreign octet.
    ///
    /// # Errors
    /// The interpreter's reason.
    pub fn lex(&mut self, source: &str) -> Result<usize, String> {
        let v = self
            .it
            .call(
                "अक्षरकोशॱपरिधिपदविभाग",
                vec![Value::Octets(Octets::new(source.as_bytes()))],
                FUEL_LEX,
            )
            .map_err(|e| e.reason)?;
        self.tokens = v.as_int().unwrap_or(0);
        usize::try_from(self.tokens).map_err(|_| "the token count does not fit".to_string())
    }

    /// **`V-005` — COLLECT FIRST, COMPILE SECOND, for a set of modules.** Lex
    /// and parse `source` so its declarations enter the shared store, and
    /// nothing more. A caller lowering several modules gathers EVERY one of
    /// them first and only then lowers each (`lex` / `parse` / … / `build_ir`
    /// again per module), exactly as the `.t1` product's
    /// `मण्डलानिप्रतिबिम्बम्` does. Without it a call into a module not yet
    /// gathered has no signature to read, so a float parameter or result in
    /// it is invisible — measured: caller-first lowering answered `3ff8…`
    /// twice where callee-first answered `4025…` and `4024c…`. Re-gathering a
    /// module on its second parse adds identical entries, and lookups take the
    /// first.
    ///
    /// # Errors
    /// The lexer's or the parser's refusal.
    pub fn gather(&mut self, source: &str) -> Result<usize, String> {
        self.lex(source)?;
        self.parse()
    }

    /// `व्याकरॱकार्यक्रमपठनम्` — the tokens to declarations; how many.
    ///
    /// # Errors
    /// The interpreter's reason, or the parser's own diagnostic count.
    pub fn parse(&mut self) -> Result<usize, String> {
        let v = self
            .it
            .call(
                "व्याकरॱकार्यक्रमपठनम्",
                vec![Value::Int(self.tokens)],
                FUEL_PARSE,
            )
            .map_err(|e| e.reason)?;
        self.declarations = v.as_int().unwrap_or(0);
        let errors = self.global_int("दोषसूचकाङ्क");
        if errors > 0 {
            return Err(format!(
                "{errors} diagnostic(s); the parser refused the source"
            ));
        }
        if self.declarations == 0 {
            return Err("the source declares nothing".to_string());
        }
        // **FILL THE SHARED DECLARATION STORE, or no later source can name this
        // one.** `सङ्ग्रहः` copies this program out of `व्याकर`'s arena into
        // `घोषणासञ्चय`; until it runs the store sits empty and every qualified
        // cross-module reference refuses at RESOLVE with "has no declaration".
        //
        // `FRONT_END` has listed `sanchaya.t1` all along, so the store was in the
        // image and inert — **listed in a loader is necessary and not sufficient.**
        // `paradigm_encode.rs`'s `load_chain_collected` has had this call since
        // W-253, whose own margin records the consequence of its absence: *"every
        // qualified use was taken on trust"*. That fix landed in a test-local
        // driver and not here, so `Front` — and with it `t1_build`, the product
        // build path — could not compile a multi-module program at all.
        //
        // It runs AFTER the refusal checks above: a program the parser refused
        // must not have its wreckage copied into a store other sources consult.
        self.it
            .call("घोषणासञ्चयॱसङ्ग्रहः", vec![], FUEL_RESOLVE)
            .map_err(|e| format!("the declaration store does not gather: {}", e.reason))?;
        usize::try_from(self.declarations).map_err(|_| "the count does not fit".to_string())
    }

    /// `अर्थॱकार्यक्रमनिर्णयः` — every name bound to a symbol.
    ///
    /// # Errors
    /// The undeclared name and its line, or the redeclared name with BOTH of
    /// its declaration lines (`W-247`) — the resolver records each by name, and
    /// this reports what it recorded rather than a summary of it.
    pub fn resolve(&mut self) -> Result<(), String> {
        let r = self
            .it
            .call("अर्थॱनिर्णायकारम्भः", vec![], FUEL_SMALL)
            .map_err(|e| e.reason)?;
        self.resolver = Some(r.clone());
        // **TELL THE RESOLVER IT MAY CONSULT THE STORE.** Without this the gather
        // above is wasted: the store is full and `निर्णयः` does not read it.
        //
        // Calling it on every resolve is deliberate and safe, not laziness.
        // `artha.t1`'s own margin states that `सञ्चयसिद्धमस्ति` **is not cleared**
        // by `निर्णायकारम्भः` — "the flag records what the CALLER did, which a new
        // resolver does not undo" — and that there is no lowering routine at all.
        // So the flag is one-way and idempotent, and `Front` needs no state of its
        // own to track whether it has been raised.
        self.it
            .call("अर्थॱसञ्चयसिद्धिः", vec![], FUEL_SMALL)
            .map_err(|e| format!("the resolver will not consult the store: {}", e.reason))?;
        let verdict = self
            .it
            .call(
                "अर्थॱकार्यक्रमनिर्णयः",
                vec![r, Value::Int(self.declarations)],
                FUEL_RESOLVE,
            )
            .map_err(|e| e.reason)?;
        if matches!(verdict, Value::Bool(true)) {
            return Ok(());
        }
        // THE SAME TWO RECORDS A DRIVER READS, and this is now the one place
        // the sentence is written. `t1_boot` and `t1_image` never construct a
        // `Front`, so before SAS-013 this rendering stood here, in
        // `frontend/src/lib.rs:634` and in `yantra/tests/paradigm_encode.rs`
        // — three copies — and in NEITHER driver, which is where a builder was
        // actually reading. Those two other copies are left standing: each
        // reads a different stage's globals for its own report and neither is
        // on the landing path of this row, so folding them in would buy a
        // whole-crate rebuild of `frontend` and `yantra` for a string.
        if let Some(site) = resolve_site(&self.it) {
            return Err(site);
        }
        Err("the resolver refused the program and recorded no name".to_string())
    }

    /// `अर्थॱकार्यक्रमप्रकारपरीक्षा` — every statement typed.
    ///
    /// # Errors
    /// The routine whose body and return kind disagree, as the checker records
    /// it; or a call made before [`Front::resolve`], whose resolver the reset
    /// below is handed.
    pub fn typecheck(&mut self) -> Result<(), String> {
        // ZERO THE CHECKER'S STATE FOR THIS SOURCE — `W-270`, the third of
        // three. `resolve` above opens with `निर्णायकारम्भः` and `build_ir`
        // below opens with `मध्यरूपॱआरम्भः`; this stage opened with nothing,
        // while `yantra/tests/paradigm_encode.rs` already called
        // `प्रकारपरीक्षकारम्भः` on its own copy of this same walk. The three
        // stages agree now, and so do the driver and the census.
        //
        // WHAT THE WALK CLEARS AND WHAT IT DOES NOT — the reason the gap was
        // latent rather than visible. `कार्यक्रमप्रकारपरीक्षा` opens by clearing
        // five globals of its own (`प्रकारदोषमस्ति`, `प्रकारदोषपङ्क्ति`,
        // `परिधिदोषमस्ति`, `परिधिदोषभेद`, `परिधिअन्तः`), and those five are
        // EXACTLY the ones this function goes on to read. `प्रकारपरीक्षकारम्भः`
        // owns a DISJOINT ten: the four blind-spot counters, the two
        // first-poison records with their flags and line, and `पुच्छे`. Nothing
        // in the chain cleared those, so they accumulated across sources.
        //
        // MEASURED, NOT INFERRED (`the_checkers_state_does_not_carry_…` below):
        // two sources through ONE `Front`, each with one mid-body `यदि` whose
        // arms disagree, left `अयुग्मशाखासंख्या` at 2 where either source alone
        // reads 1 — and a source the checker REFUSES left its `अबूलशर्तसंख्या`
        // 1 and `दुष्टकारण` 1 standing over the next source's own 0 and 0, so a
        // later poison's recorded CAUSE was the earlier program's.
        //
        // THE VERDICT DOES NOT MOVE, and `W-253` measured that before this row
        // did (`sadhana-t1/tests/t1_execution.rs`). So this is not a fix for a
        // wrong answer `Front` gives today — `Front` reads none of the ten. It
        // is the condition under which `Front`'s answers stay right for a
        // caller that reads a counter or a poison SITE, and `Front` is library
        // code precisely so that a caller other than `t1_build` can exist.
        //
        // ONE CAVEAT THIS ROW MEASURED AND `W-253` DID NOT. That test asserts a
        // completed typecheck leaves `पुच्छे` TRUE and says that if it ever
        // reads false its reasoning is void and the reset becomes load-bearing.
        // IT READS FALSE for a source the checker refuses — the test drove only
        // accepting ones. It still moves no verdict, because every block arm of
        // `वाक्यप्रकारः` writes `पुच्छे` before any rule reads it: the flag is
        // carried, never consulted. The claim that survives is narrower than
        // `W-253`'s — "a completed typecheck ends true" is false; "no rule reads
        // the incoming value" is what holds.
        let Some(resolver) = self.resolver.clone() else {
            return Err("typecheck before resolve: the checker reads the \
                        resolver's symbol-keyed arenas, and its reset is handed \
                        the resolver this program bound"
                .to_string());
        };
        self.it
            .call("अर्थॱप्रकारपरीक्षकारम्भः", vec![resolver], FUEL_SMALL)
            .map_err(|e| e.reason)?;
        let verdict = self
            .it
            .call(
                "अर्थॱकार्यक्रमप्रकारपरीक्षा",
                vec![Value::Int(self.declarations)],
                FUEL_RESOLVE,
            )
            .map_err(|e| e.reason)?;
        if matches!(verdict, Value::Bool(true)) {
            return Ok(());
        }
        // As `resolve` above: one rendering, read by the drivers too. The two
        // records `typecheck_site` adds beyond `प्रकारदोष…` fire only where
        // this used to answer "recorded no cause", so no caller loses a message.
        Err(typecheck_site(&self.it)
            .unwrap_or_else(|| "the checker refused the program and recorded no cause".to_string()))
    }

    /// `मध्यरूपॱकार्यक्रमरचना` — the declarations to IR; the routines built.
    ///
    /// # Errors
    /// The interpreter's reason, or the call whose callee carries no symbol —
    /// the builder's own refusal, with the line it recorded.
    pub fn build_ir(&mut self) -> Result<usize, String> {
        self.it
            .call("मध्यरूपॱआरम्भः", vec![], FUEL_SMALL)
            .map_err(|e| e.reason)?;
        self.it
            .call(
                "मध्यरूपॱकार्यक्रमरचना",
                vec![Value::Int(self.declarations)],
                FUEL_RESOLVE,
            )
            .map_err(|e| e.reason)?;
        if self.global_bool("अनिर्णीताह्वानमस्ति") {
            let tok = self.global_int("अनिर्णीताह्वानचिह्नकाङ्क");
            return Err(format!(
                "a call's callee carries no symbol, at line {}",
                self.token_line(tok)
            ));
        }
        // `V-005`: a value crossing into a declared file it is not in — a local,
        // an argument, a returned value — refuses the program by name.
        if let Some(why) = file_mismatch_site(&self.it) {
            return Err(why);
        }
        let count = usize::try_from(self.global_int("वृत्तिसूचकाङ्क")).unwrap_or(0);
        // A SOURCE WITH NO ROUTINES BUT WITH GLOBALS HAS SOMETHING TO BUILD: its
        // DATA. `वास्तु` declares 45 globals and not one routine, and the six
        // modules that read them cannot resolve a symbol no object defines — so
        // `ir.t1`'s cause-44 arm refuses those reads ON PURPOSE.
        //
        // **THIS GUARD WAS THE ONLY THING REFUSING.** `मध्यरूपॱकार्यक्रमरचना` above
        // builds the globals whatever the routine count, and
        // `यन्त्रमण्डलोत्सर्जनम्` calls `यन्त्रदत्तोत्सर्जनम्` AFTER its routine loop
        // and unconditionally — so the `.t1` half has always been able to emit a
        // data-only object and has never been asked. Measured: with this guard in
        // place `Front::build_ir` answers "the source declares no routine", and the
        // emitter is never reached.
        let globals = usize::try_from(self.global_int("वैश्विकसञ्चयसूचकाङ्क")).unwrap_or(0);
        if count == 0 && globals == 0 {
            return Err(
                "the source declares no routine and no global, so there is nothing to build"
                    .to_string(),
            );
        }
        Ok(count)
    }

    /// The IR arenas, read into the emitter's [`Module`].
    ///
    /// `module` names the half of every label that is not the routine's
    /// (research/25 §2.2). `entry` is the routine whose result becomes the exit
    /// status; `None` emits a stub that calls nothing.
    ///
    /// # Errors
    /// A value the IR names as absent (`०`) where one is required — the shape a
    /// refused call leaves behind — with the instruction that names it.
    pub fn module(&mut self, module: &str, entry: Option<&str>) -> Result<Module, String> {
        let mut built = self.read_module(module)?;
        built.module.entry = match entry {
            None => built.first_zero_parameter(),
            Some(name) => match built.symbol_of(name) {
                Some(sym) => Some(sym),
                None => {
                    return Err(format!("`{name}` is not a routine this source declares"));
                }
            },
        };
        Ok(built.module)
    }

    /// Every routine's name, in declaration order — what `--entry` may name.
    #[must_use]
    pub fn routine_names(&mut self) -> Vec<String> {
        let names = self.arena("वृत्तिनामचिह्नककोश");
        let count = usize::try_from(self.global_int("वृत्तिसूचकाङ्क")).unwrap_or(0);
        (1..=count)
            .map(|i| {
                let idx = arena_int(&names, i);
                self.token_text(idx)
            })
            .collect()
    }

    // ── the arenas ───────────────────────────────────────────────────────

    // The free readers below are the one implementation; these two are the
    // `&self` spellings the rest of this file already reads through. The third,
    // `global_text`, went with the three copies of the refusal rendering it
    // existed to serve — `resolve_site` and `typecheck_site` own that text now.
    fn global_int(&self, name: &str) -> i128 {
        g_int(&self.it, name)
    }

    fn global_bool(&self, name: &str) -> bool {
        g_bool(&self.it, name)
    }

    fn arena(&self, name: &str) -> Rc<RefCell<Vec<Value>>> {
        match self.it.global(name) {
            Some(Value::Arena(a)) => Rc::clone(a),
            _ => Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// The text of token `idx`, through the lexer's own reader.
    fn token_text(&mut self, idx: i128) -> String {
        if idx <= 0 {
            return String::new();
        }
        let tokens = self.arena("चिह्नककोश");
        let Some(tok) = tokens.borrow().get(idx as usize).cloned() else {
            return String::new();
        };
        match self.it.call("पदविभागॱचिह्नकपाठः", vec![tok], FUEL_SMALL)
        {
            Ok(v) => text_of(&v),
            Err(_) => String::new(),
        }
    }

    fn token_line(&self, idx: i128) -> i128 {
        if idx <= 0 {
            return 0;
        }
        self.arena("चिह्नककोश")
            .borrow()
            .get(idx as usize)
            .map(|t| int_of(t, "पङ्क्ति"))
            .unwrap_or(0)
    }

    /// The `(module, member)` a callee NODE names — a one-token qualified name
    /// split at `ॱ`, `W-228` (b)'s folded spaced qualifier, or a `क्षेत्र` whose
    /// base is the module.
    fn callee_of_node(&mut self, node: i128) -> Option<(String, String)> {
        if node <= 0 {
            return None;
        }
        let nodes = self.arena("अभिव्यञ्जककोश");
        let n = nodes.borrow().get(node as usize).cloned()?;
        let kind = int_of(&n, "भेद");
        let name_kind = self.global_int("नामाभिव्यञ्जकभेद");
        let field_kind = self.global_int("क्षेत्राभिव्यञ्जकभेद");
        if kind == name_kind {
            let text = self.token_text(int_of(&n, "मूल्यसूचकाङ्क"));
            let folded = int_of(&n, "दक्षिणसूचकाङ्क");
            if folded > 0 {
                return Some((text, self.token_text(folded)));
            }
            if let Some((m, r)) = text.split_once('\u{971}') {
                return Some((m.to_string(), r.to_string()));
            }
            return Some((String::new(), text));
        }
        if kind == field_kind {
            let base = int_of(&n, "वामसूचकाङ्क");
            let base_text = {
                let b = nodes.borrow().get(base as usize).cloned();
                match b {
                    Some(b) if int_of(&b, "भेद") == name_kind => {
                        let idx = int_of(&b, "मूल्यसूचकाङ्क");
                        self.token_text(idx)
                    }
                    _ => String::new(),
                }
            };
            let member = self.token_text(int_of(&n, "मूल्यसूचकाङ्क"));
            return Some((base_text, member));
        }
        None
    }

    /// The reading itself: `मध्यरूप`'s five arenas into the emitter's module.
    #[allow(clippy::too_many_lines)]
    fn read_module(&mut self, module: &str) -> Result<Built, String> {
        let need = |v: &Value, name: &str, at: &str| -> Result<usize, String> {
            id_of(v, name).ok_or_else(|| format!("{at} names the absent value ० as its `{name}`"))
        };
        // The resolver's scopes: a routine's name to the symbol it was given,
        // so the emitter's labels are the resolver's names and a `Call` that
        // carries a symbol needs no invention.
        let mut entry_symbol: HashMap<String, i128> = HashMap::new();
        let mut symbol_name: HashMap<i128, String> = HashMap::new();
        if let Some(resolver) = self.resolver.clone()
            && let Value::Arena(scopes) = member(&resolver, "परिसराः")
        {
            for scope in scopes.borrow().iter() {
                if let Value::Arena(entries) = member(scope, "प्रविष्टयः") {
                    for e in entries.borrow().iter() {
                        if matches!(e, Value::Record(_)) {
                            let name = text_of(&member(e, "नाम"));
                            let sym = int_of(e, "संज्ञा") + 1;
                            entry_symbol.entry(name.clone()).or_insert(sym);
                            symbol_name.entry(sym).or_insert(name);
                        }
                    }
                }
            }
        }
        let kinds = self.arena("संज्ञाभेदकोश");
        let routine_kind = self.global_int("वृत्तिघोषणाभेद");
        let functions = self.arena("वृत्तिकोश");
        let blocks = self.arena("पर्वकोश");
        let insts = self.arena("आज्ञाकोश");
        let args = self.arena("आदानकोश");
        // `W-254`: a string literal's octets, one contiguous run per literal.
        // A RUN OF OCTETS, NOT AN ARENA — `अङ्कः अन्तः अ८` is a `Value::Octets`
        // where `अङ्कः अन्तः मूल्याङ्क` is a `Value::Arena`, so `आदानकोश` above
        // is `Rc`-shared and live and this is a COPY taken here.
        let string_octets: Vec<u8> = self
            .it
            .global("पाठाक्षरकोश")
            .and_then(Value::octets)
            .map(|o| o.as_slice().to_vec())
            .unwrap_or_default();
        let callee_nodes = self.arena("आह्वेयकोश");
        let routine_names = self.arena("वृत्तिनामचिह्नककोश");
        let count = usize::try_from(self.global_int("वृत्तिसूचकाङ्क")).unwrap_or(0);

        let mut names = Names::new();
        let mut routine_symbol: HashMap<String, SymbolId> = HashMap::new();
        let mut routine_syms: Vec<SymbolId> = Vec::new();
        let mut declared: Vec<(String, SymbolId)> = Vec::new();
        for i in 1..=count {
            let idx = arena_int(&routine_names, i);
            // `W-295` — **A NAME TOKEN OF ० IS NOT A MISSING NAME. IT IS THE
            // MODULE'S SYNTHESISED GROWTH ROUTINE**, and reading it as the
            // former fabricated BOTH halves of its identity.
            //
            // `ir.t1:4287` writes `वृत्तिनामचिह्नककोश[वृद्धिवृत्त्यङ्कः] = ०`
            // deliberately: `खण्डवृद्धिः` has no declaration and so no token to
            // point at, and its symbol — `वृद्धिवृत्तिसंज्ञा`, `१००००००५` — was
            // already put in the function record's `नाम` by `वृत्तियोजनम्`.
            // `shrinkhala.t1:334-338`, the `.t1` driver, states the rule in two
            // lines: *"Token ० is the module's synthesised growth routine …
            // its symbol is already in `कार्यम् ॱ नाम`; name it (module,
            // "खण्डवृद्धिः")"*.
            //
            // **THIS HALF DID NEITHER, AND THE TWO WRONGS COMPOUNDED.** An empty
            // token text fell to the `वृत्ति{i}` fallback, the fabricated name
            // missed `entry_symbol`, the symbol fell to `1_000_000 + i` — and
            // the `set_int(&f, "नाम", …)` below then OVERWROTE the real
            // `१००००००५` with it. The call sites `ir.t1:3938` emits still carry
            // `१००००००५`, which no longer matched any definition, so every run
            // that grows linked against a label nothing defines:
            //
            // ```text
            //   the linker refused: `परीक्षासंज्ञा१००००००५` is not defined by any object
            // ```
            //
            // **THE FAILURE MOVED BUT DID NOT GET WORSE, AND THAT IS WHY IT WAS
            // NOT SEEN.** Until the `AddrOfGlobal` repair above, the same seven
            // fixtures died EARLIER, at emit, with `UnnamedSymbol` — one defect
            // masking the next in the same pipeline. Measured 2026-09-17 on this
            // tree: 13 red → 7 red (the record half green, the run half now at
            // `link`) → 0. A repair that only moves a count is not evidence the
            // first repair was wrong.
            //
            // The `1_000_000 + i` fallback is KEPT for a genuinely unnamed
            // routine, because it still numbers apart from the resolver's
            // symbols; what it must not do is claim this one.
            //
            // `N-004`: NAMED BY THE SYMBOL, so a module may hold more than one
            // built routine (`BUILT_ROUTINES`).
            let built = if idx == 0 && i < functions.borrow().len() {
                let symbol = int_of(&functions.borrow()[i], "नाम");
                match built_routine_name(symbol) {
                    Some(name) => Some((name, symbol)),
                    None => {
                        return Err(format!(
                            "routine {i} of `{module}` has no declaration and its symbol {symbol} \
                             names no built routine (chain.rs `BUILT_ROUTINES`)"
                        ));
                    }
                }
            } else {
                None
            };
            let name = if let Some((name, _)) = built {
                name.to_string()
            } else {
                let name = self.token_text(idx);
                if name.is_empty() {
                    format!("वृत्ति{}", riscv64::devanagari(i as i64))
                } else {
                    name
                }
            };
            let sym = if let Some((_, symbol)) = built {
                // THE SYMBOL THE ARENA ALREADY HOLDS: `ir.t1` decides it and
                // this reads it back; the table above only NAMES it.
                SymbolId(usize::try_from(symbol).unwrap_or(1_000_000 + i))
            } else {
                match entry_symbol.get(&name) {
                    Some(s) if arena_int(&kinds, *s as usize) == routine_kind => {
                        SymbolId(*s as usize)
                    }
                    // A routine the resolver did not name: numbered apart so it
                    // cannot collide with a symbol the resolver minted.
                    _ => SymbolId(1_000_000 + i),
                }
            };
            names.insert(sym, (module.to_string(), name.clone()));
            routine_symbol.insert(name.clone(), sym);
            routine_syms.push(sym);
            declared.push((name, sym));
        }

        let mut next_synth = 2_000_000usize;
        let mut synth: HashMap<(String, String), SymbolId> = HashMap::new();
        let mut out_functions: Vec<Function> = Vec::new();
        let mut parameters: HashMap<SymbolId, usize> = HashMap::new();

        for i in 1..=count {
            let f = functions.borrow()[i].clone();
            let sym = routine_syms[i - 1];
            // One IR for both twins: the symbol chosen here is written back
            // where the T1 emitter reads it.
            set_int(&f, "नाम", sym.0 as i128);
            let start = usize::try_from(int_of(&f, "पर्वारम्भ")).unwrap_or(0);
            let n = usize::try_from(int_of(&f, "पर्वसंख्यान")).unwrap_or(0);
            let mut out_blocks = HashMap::new();
            let mut params = 0usize;
            for b in start..start + n {
                let blk = blocks.borrow()[b].clone();
                let first = usize::try_from(int_of(&blk, "आज्ञारम्भ")).unwrap_or(0);
                let k = usize::try_from(int_of(&blk, "आज्ञासंख्यान")).unwrap_or(0);
                let mut out_insts = Vec::new();
                for j in first..first + k {
                    let ins = insts.borrow()[j].clone();
                    let at = format!("instruction {j} of block {b}");
                    let v = ValueId(need(&ins, "फलम्", &at)?);
                    let inst = match int_of(&ins, "भेद") {
                        1 => Instruction::ConstInt(int_of(&ins, "ध्रुवमूल्यम्") as i64),
                        2 => {
                            let s = int_of(&ins, "संज्ञा");
                            let sym = if s > 0 && names.contains_key(&SymbolId(s as usize)) {
                                SymbolId(s as usize)
                            } else if s > 0 {
                                // **A RESOLVED SYMBOL IS NOT EVIDENCE THAT THIS MODULE
                                // OWNS IT.** This arm used to attribute any symbol the
                                // resolver had assigned to `module`, producing the label
                                // `<caller>संज्ञा<N>` — which no object defines, because
                                // the callee's object defines `<callee>संज्ञा<N>`.
                                // Measured: `परीक्षासंज्ञा४` undefined against `अन्यत्`'s
                                // `अन्यत्संज्ञा४`.
                                //
                                // `artha.t1:2610` states the authority this now consults:
                                // a cross-module label comes "from the callee NODE rather
                                // than from a SymbolId that means nothing outside the
                                // program that minted it". The node was already in scope
                                // and read twenty lines below, in the arm taken when the
                                // resolver assigned NOTHING — so the two arms disagreed
                                // about who owns a foreign symbol, and which one ran was
                                // decided by whether the declaration store had been
                                // gathered.
                                //
                                // **Only a genuinely foreign module diverts.** An empty
                                // module, or this one, falls through to the original
                                // local attribution unchanged, so a local call keeps the
                                // symbol the resolver gave it.
                                let foreign = self
                                    .callee_of_node(arena_int(&callee_nodes, j))
                                    .filter(|(m, _)| !m.is_empty() && m != module);
                                if let Some((m, r)) = foreign {
                                    *synth.entry((m.clone(), r.clone())).or_insert_with(|| {
                                        next_synth += 1;
                                        let sym = SymbolId(next_synth);
                                        names.insert(sym, (m, r));
                                        sym
                                    })
                                } else {
                                    let sym = SymbolId(s as usize);
                                    let nm = symbol_name.get(&s).cloned().unwrap_or_else(|| {
                                        format!("संज्ञा{}", riscv64::devanagari(s as i64))
                                    });
                                    names.entry(sym).or_insert((module.to_string(), nm));
                                    sym
                                }
                            } else {
                                // A qualified callee the resolver took on trust:
                                // the callee NODE says which module and member.
                                let node = arena_int(&callee_nodes, j);
                                let (m, r) = self
                                    .callee_of_node(node)
                                    .unwrap_or_else(|| (String::new(), "बाह्यम्".to_string()));
                                let (m, r) = if m.is_empty() {
                                    (module.to_string(), r)
                                } else {
                                    (m, r)
                                };
                                if m == module && routine_symbol.contains_key(&r) {
                                    routine_symbol[&r]
                                } else {
                                    *synth.entry((m.clone(), r.clone())).or_insert_with(|| {
                                        next_synth += 1;
                                        let sym = SymbolId(next_synth);
                                        names.insert(sym, (m, r));
                                        sym
                                    })
                                }
                            };
                            set_int(&ins, "संज्ञा", sym.0 as i128);
                            let a0 = usize::try_from(int_of(&ins, "आदानारम्भ")).unwrap_or(0);
                            let an = usize::try_from(int_of(&ins, "आदानसंख्यान")).unwrap_or(0);
                            let mut list = Vec::new();
                            for q in a0..a0 + an {
                                let arg = args.borrow()[q].clone();
                                let k = int_of(&arg, "क्रमाङ्क");
                                let k = usize::try_from(k).ok().filter(|k| *k > 0).ok_or_else(
                                    || format!("{at} passes the absent value ० as argument {q}"),
                                )?;
                                list.push(ValueId(k - 1));
                            }
                            // `V-005`: `उपभेद` १ is a callee answering `प६४` — `Call`
                            // first, as kind २'s own (the transcription guard reads
                            // the first variant an arm names).
                            if int_of(&ins, "उपभेद") != 1 {
                                Instruction::Call(sym, list)
                            } else {
                                Instruction::CallFloat(sym, list)
                            }
                        }
                        5 => {
                            params += 1;
                            let k = usize::try_from(int_of(&ins, "प्राचलक्रम")).unwrap_or(0);
                            // `V-005`: `उपभेद` १ is a `प६४` parameter, in the float file.
                            if int_of(&ins, "उपभेद") != 1 {
                                Instruction::Param(k)
                            } else {
                                Instruction::ParamFloat(k)
                            }
                        }
                        // `W-245`: a local's slot, as the builder numbers it (from ०).
                        //
                        // `V-005`: `उपभेद` १ marks a `प६४` local's read, which loads
                        // into the FLOAT file. Reading the kind and dropping the
                        // field would decode it as `Load` and put a float in an `x`
                        // register — so the field is read here, not defaulted.
                        15 => {
                            // An `if` and not a `match` on the mark: a numeric arm here
                            // would read as a KIND row to `t1_transcriptions.rs`'s
                            // guard, which finds the kind tables by their arms and
                            // reads the FIRST variant an arm names — so `Load`,
                            // kind १५'s own, is written first.
                            let k = usize::try_from(int_of(&ins, "स्थानक्रम")).unwrap_or(0);
                            let mark = int_of(&ins, "उपभेद");
                            if mark == 0 {
                                Instruction::Load(k)
                            } else if mark == 1 {
                                Instruction::LoadFloat(k)
                            } else {
                                return Err(format!(
                                    "{at}: a local read marked {mark}, which is neither \
                                     ० (an integer slot) nor १ (a float slot)"
                                ));
                            }
                        }
                        // `V-005` — a float op: `उपभेद` is the op (from १), `वाम` and
                        // `दक्षिण` its first two operands, and `fmadd`'s third the
                        // one entry of its `आदानकोश` run.
                        27 => {
                            let code = int_of(&ins, "उपभेद");
                            let op = FloatOp::from_code(code).ok_or_else(|| {
                                format!("{at}: float op {code} is not one of the thirteen")
                            })?;
                            let mut list = vec![ValueId(need(&ins, "वाम", &at)?)];
                            if op.arity() >= 2 {
                                list.push(ValueId(need(&ins, "दक्षिण", &at)?));
                            }
                            if op.arity() == 3 {
                                let a0 = usize::try_from(int_of(&ins, "आदानारम्भ")).unwrap_or(0);
                                let arg = args.borrow().get(a0).cloned().ok_or_else(|| {
                                    format!("{at}: fmadd's third operand is past आदानकोश")
                                })?;
                                let k = int_of(&arg, "क्रमाङ्क");
                                let k = usize::try_from(k).ok().filter(|k| *k > 0).ok_or_else(
                                    || format!("{at}: fmadd's third operand is the absent value ०"),
                                )?;
                                list.push(ValueId(k - 1));
                            }
                            Instruction::Float(op, list)
                        }
                        // `V-008` part 2 — a vector op: `उपभेद` is the op (१..४, the float
                        // sub-kinds add/sub/mul/div), `वाम` the result run, `दक्षिण` the
                        // first operand run and the second the one entry of its
                        // `आदानकोश` run.
                        28 => {
                            let code = int_of(&ins, "उपभेद");
                            let op = FloatOp::from_code(code)
                                .filter(|op| {
                                    matches!(
                                        op,
                                        FloatOp::Add | FloatOp::Sub | FloatOp::Mul | FloatOp::Div
                                    )
                                })
                                .ok_or_else(|| {
                                    format!("{at}: vector op {code} is not add, sub, mul or div")
                                })?;
                            let a0 = usize::try_from(int_of(&ins, "आदानारम्भ")).unwrap_or(0);
                            let arg = args.borrow().get(a0).cloned().ok_or_else(|| {
                                format!("{at}: a vector op's second operand run is past आदानकोश")
                            })?;
                            let k = int_of(&arg, "क्रमाङ्क");
                            let k =
                                usize::try_from(k).ok().filter(|k| *k > 0).ok_or_else(|| {
                                    format!(
                                        "{at}: a vector op's second operand is the absent value ०"
                                    )
                                })?;
                            Instruction::Vector(
                                op,
                                vec![
                                    ValueId(need(&ins, "वाम", &at)?),
                                    ValueId(need(&ins, "दक्षिण", &at)?),
                                    ValueId(k - 1),
                                ],
                            )
                        }
                        // `W-278`: a module-level global read — the word at the
                        // global's own exported label, named by its symbol.
                        // `W-field`: a record field read — one word at a byte offset from
                        // a RUNTIME base. The offset is carried in `ध्रुवमूल्यम्` and never
                        // in `स्थानक्रम`, which is a FRAME INDEX the emitter scales by ८
                        // and the frame-sizing pass maxes over — a byte offset there is
                        // wrong twice and loud neither time.
                        // `W-381` stage 4: the offset may be NEGATIVE — the bound
                        // check reads a run's length word at base − 8 — so it is
                        // carried as the i64's two's-complement bits. The old
                        // `u64::try_from(..).unwrap_or(0)` turned −8 into a SILENT 0.
                        19 => Instruction::LoadField(
                            ValueId(need(&ins, "वाम", &at)?),
                            i64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).map_or(0, i64::cast_unsigned),
                        ),
                        // BOTH operands are values here, where the kind above takes one
                        // value and a constant. A decoder that read `ध्रुवमूल्यम्`
                        // for the second would silently decode every index read as an
                        // offset of zero.
                        //
                        // **`W-294`: IT NOW CARRIES A CONSTANT TOO, AND THAT IS NOT A
                        // CONTRADICTION OF THE SENTENCE ABOVE.** Both OPERANDS are still
                        // values; `ध्रुवमूल्यम्` is a third thing beside them, holding the
                        // element's width in OCTETS so the emitter can pick between
                        // `आहारः` and `आहारःॱअ८`. The warning stands exactly as written —
                        // reading `ध्रुवमूल्यम्` as the second OPERAND is still the bug it
                        // describes.
                        //
                        // A `.t1` half that has not learned to write the width answers ०
                        // here, and ० is not a width. It is mapped to ८ rather than
                        // carried, because a zero would scale every index to the same
                        // address — the `unwrap_or(0)` default is safe for an OFFSET and
                        // is not safe for a STRIDE.
                        20 => Instruction::LoadIndex(
                            ValueId(need(&ins, "वाम", &at)?),
                            ValueId(need(&ins, "दक्षिण", &at)?),
                            match u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0)
                            {
                                0 => 8,
                                w => w,
                            },
                        ),
                        18 => Instruction::LoadGlobal(SymbolId(
                            usize::try_from(int_of(&ins, "संज्ञा")).unwrap_or(0),
                        )),
                        // `W-283`, the ruled storage model: an address is a VALUE.
                        // 21 forms one from a label and 22/23 consume it, so the
                        // three above (19, 20, 18) are the forms this pair replaces
                        // — each of them builds an address and immediately spends it.
                        21 => Instruction::AddrOfGlobal(SymbolId(
                            usize::try_from(int_of(&ins, "संज्ञा")).unwrap_or(0),
                        )),
                        // `V-008`: `उपभेद` १ is a `प६४` slot's read (a run element, a field
                        // or a global declared `प६४`), into the FLOAT file — `LoadAt` first,
                        // as kind २२'s own (the transcription guard reads the first variant).
                        22 => {
                            let a = ValueId(need(&ins, "वाम", &at)?);
                            if int_of(&ins, "उपभेद") != 1 {
                                Instruction::LoadAt(a)
                            } else {
                                Instruction::LoadAtFloat(a)
                            }
                        }
                        // `वाम` is the ADDRESS and `दक्षिण` is the value stored. The
                        // order matters and is not recoverable from the decode: a
                        // decoder that swapped them would write the address into the
                        // value's storage and be silent about it, since both are
                        // words and both are live.
                        // `W-306c` — AND `ध्रुवमूल्यम्` IS THE WIDTH IN OCTETS, a third
                        // thing beside the two values, exactly as it is on kind 20. A
                        // decoder that read it as an OPERAND would store at the
                        // address held in value ०.
                        //
                        // THE ० -> ८ MAPPING IS COPIED FROM KIND 20 DELIBERATELY, not
                        // re-derived: two decoders that default differently are worse
                        // than two that default wrongly together, because only the
                        // first kind of disagreement is invisible to a twin
                        // comparison. `ir.t1` does not write this field yet, so every
                        // instruction in the corpus arrives here as ० and leaves as ८
                        // — a whole word, today's behaviour, and the bare `निधानम्`.
                        23 => Instruction::StoreAt(
                            ValueId(need(&ins, "वाम", &at)?),
                            ValueId(need(&ins, "दक्षिण", &at)?),
                            match u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0)
                            {
                                0 => 8,
                                w => w,
                            },
                        ),
                        // `W-284` — THE STORAGE THE OTHER FIVE KINDS ADDRESS. 24
                        // allocates it, 25 and 26 form addresses INTO it, and 22/23
                        // spend them. The size is a build-time constant in
                        // `ध्रुवमूल्यम्` and there are NO value operands, which is why
                        // this kind is absent from `operands()` — read `ir.rs` there
                        // rather than the omission.
                        24 => Instruction::AllocRecord(
                            u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0),
                        ),
                        // 25 takes a value and a CONSTANT where 26 takes two values —
                        // the same asymmetry as 19 against 20, and for the same
                        // reason: a field's offset is known at build time and an
                        // index's is not. A decoder reading `ध्रुवमूल्यम्` for 26's
                        // second operand would make every index address offset zero,
                        // silently.
                        25 => Instruction::AddrOfField(
                            ValueId(need(&ins, "वाम", &at)?),
                            u64::try_from(int_of(&ins, "ध्रुवमूल्यम्")).unwrap_or(0),
                        ),
                        26 => Instruction::AddrOfIndex(
                            ValueId(need(&ins, "वाम", &at)?),
                            ValueId(need(&ins, "दक्षिण", &at)?),
                        ),
                        16 => Instruction::Store(
                            usize::try_from(int_of(&ins, "स्थानक्रम")).unwrap_or(0),
                            ValueId(need(&ins, "वाम", &at)?),
                        ),
                        // `W-254`: the octets of a string literal, by the run
                        // the instruction names. Past the end is an error and
                        // not a truncation — a short string is a wrong image.
                        17 => {
                            let start = usize::try_from(int_of(&ins, "पाठारम्भ")).unwrap_or(0);
                            let count = usize::try_from(int_of(&ins, "पाठसंख्यान")).unwrap_or(0);
                            let end = start + count;
                            let bytes = string_octets.get(start..end).ok_or_else(|| {
                                format!(
                                    "{at}: octets {start}..{end} and पाठाक्षरकोश holds {}",
                                    string_octets.len()
                                )
                            })?;
                            Instruction::ConstStr(bytes.to_vec())
                        }
                        kind => binary_kind(
                            kind,
                            ValueId(need(&ins, "वाम", &at)?),
                            ValueId(need(&ins, "दक्षिण", &at)?),
                            int_of(&ins, "उपभेद"),
                            int_of(&ins, "ध्रुवमूल्यम्"),
                        )
                        .ok_or_else(|| {
                            format!("{at}: instruction kind {kind} is not one of the seventeen")
                        })?,
                    };
                    out_insts.push((v, inst));
                }
                let term = member(&blk, "अवसानम्");
                let at = format!("the terminator of block {b}");
                let terminator = match int_of(&term, "भेद") {
                    0 => None,
                    1 => {
                        let k = int_of(&member(&term, "मूल्यम्"), "क्रमाङ्क");
                        Some(Terminator::Return(if k > 0 {
                            Some(ValueId(k as usize - 1))
                        } else {
                            None
                        }))
                    }
                    2 => Some(Terminator::Branch(BlockId(need(&term, "लक्ष्यम्", &at)?))),
                    3 => Some(Terminator::Unreachable),
                    4 => Some(Terminator::CondBranch(
                        ValueId(need(&term, "मूल्यम्", &at)?),
                        BlockId(need(&term, "लक्ष्यम्", &at)?),
                        BlockId(need(&term, "अन्यलक्ष्यम्", &at)?),
                    )),
                    other => {
                        return Err(format!(
                            "{at}: terminator kind {other} is not one of the four"
                        ));
                    }
                };
                let id = BlockId(b - 1);
                out_blocks.insert(
                    id,
                    Block {
                        id,
                        insts: out_insts,
                        terminator,
                    },
                );
            }
            parameters.insert(sym, params);
            out_functions.push(Function {
                name: sym,
                blocks: out_blocks,
                entry_block: BlockId(need(&f, "प्रवेशपर्व", &format!("routine {i}"))?),
            });
        }
        // `W-279` — THE MODULE'S GLOBALS, READ FROM THE THREE ARENAS `ir.t1`
        // FILLS. Until now this was `Vec::new()` and `names` carried routines
        // only, so `riscv64.rs`'s `LoadGlobal` arm called `routine_label` on a
        // symbol that was never inserted and EVERY module-level global read
        // refused with `UnnamedSymbol`. Measured: 16 of 16 corpus sources that
        // reach emit failed there, and `t1_build` produced ZERO ELFs.
        //
        // BOTH HALVES ARE NEEDED AND REPAIRING ONE ALONE IS WORSE THAN NEITHER.
        // `emit_data` iterates `globals` to write `॥ वैश्विकम् {label} ॥`, the
        // DEFINITION; `names` is what `routine_label` reads for the REFERENCE.
        // Filling `names` alone would emit a module referencing a global it
        // never defines — moving the failure from `emit` to `link` and looking
        // like a new defect.
        //
        // `paradigm_encode.rs` HAS DONE THIS SINCE `W-278` and this file did
        // not: the census's own decode inserts these entries, under a margin
        // saying exactly why they are required. One requirement, two pipelines,
        // satisfied in the one the instrument used — which is why the census
        // reached `run` for seventeen sources while the production loader built
        // none at all.
        //
        // The `.t1` emitter needs nothing: `यन्त्रदत्तोत्सर्जनम्` reads the same
        // arenas directly and emits each `॥ वैश्विकम् ॥` itself. There is no
        // `Module` on that side to leave empty, so this is scaffolding catching
        // up to a port that already compiles the construct.
        let g_modules = self.arena("वैश्विकमण्डलकोश");
        let g_names = self.arena("वैश्विकनामकोश");
        let g_values = self.arena("वैश्विकमूल्यकोश");
        // `W-293` — THE OCTETS EACH GLOBAL RESERVES, CARRIED AND NOT DECIDED.
        // `मध्यरूप` computes it (`ir.t1:2793`): `८` for a scalar, which is what
        // this side has always emitted, and `खण्डसामर्थ्यम् गुणनम् ८` for a run.
        // **Nothing here derives it.** This half is the emitter TWIN and its job is
        // to say what the `.t1` emitter says, octet for octet; a global's storage
        // is the product's decision and reading the arena is how it arrives.
        let g_octets = self.arena("वैश्विकसामर्थ्यकोश");
        let g_count = usize::try_from(self.global_int("वैश्विकसञ्चयसूचकाङ्क")).unwrap_or(0);
        let mut globals: Vec<(String, i64, i64)> = Vec::new();
        for i in 1..=g_count {
            let m = {
                let b = g_modules.borrow();
                match b.get(i) {
                    Some(v) => text_of(v),
                    None => continue,
                }
            };
            let n = {
                let b = g_names.borrow();
                match b.get(i) {
                    Some(v) => text_of(v),
                    None => continue,
                }
            };
            if n.is_empty() {
                continue;
            }
            // A global the `.t1` side recorded before this arena existed answers
            // ० here; `WORD` is the floor rather than the answer, so an older
            // interpreter state emits the scalar quad it always did instead of
            // reserving nothing and pointing at it.
            let octets = arena_int(&g_octets, i) as i64;
            globals.push((
                format!("{m}{n}"),
                arena_int(&g_values, i) as i64,
                if octets < 8 { 8 } else { octets },
            ));
            if let Some(sym) = entry_symbol.get(&n) {
                names.insert(SymbolId(*sym as usize), (m, n));
            }
        }

        // `W-296` — THE REGISTRY-RESOLVED GLOBAL, WHICH THE LOOP ABOVE CANNOT
        // REACH BECAUSE IT IS NOT THIS MODULE'S GLOBAL.
        //
        // `6e3e03cc` (2026-09-13, "the lowering half of वास्तु") stopped `ir.t1`
        // refusing a cross-module global read and lowered it to
        // `वैश्विकाज्ञाभेद` carrying the RESOLVER's symbol. The emitter turns that
        // symbol into a label only if the symbol is NAMED, and the loop above
        // walks `वैश्विकमण्डलकोश`/`वैश्विकनामकोश` — **the globals THIS module
        // declares**. A foreign one is in neither arena, so there is no entry to
        // miss: it has to be spelled, exactly as `W-295`'s three minted symbols
        // did. `AddrOfGlobal`/`LoadGlobal` then called `routine_label` on a
        // symbol `names` did not hold and the emitter refused the whole module:
        //
        // ```text
        //   emit परीक्षा: UnnamedSymbol { symbol: SymbolId(4) }
        // ```
        //
        // **THAT COMMIT WROTE THIS LOOP TWICE AND THIS FILE WAS NEITHER COPY.**
        // `shrinkhala.t1:449-470` (the `.t1` driver's `नामसञ्चयः`) and
        // `crates/yantra/tests/paradigm_encode.rs:1164-1210` (the census twin)
        // both walk the registry; `chain.rs` — the `Front` that `t1_image`,
        // `t1_boot` and every probe drive — did not, so the census read
        // `twin AGREE` while the product path refused. **A pair of twins that
        // agree is not a statement about a third reader.**
        //
        // Mirrored from the census twin rather than from the driver, because the
        // census is what measures the two emitters octet for octet: the
        // `चरघोषणाभेद` filter and `or_insert` are its, not a reconstruction.
        // `or_insert` and not `insert`: a symbol this module's own arenas already
        // named keeps that name, so a local global cannot be relabelled by a
        // store entry that happens to share its id.
        {
            let n = usize::try_from(self.global_int("सञ्चयसंज्ञासूचकाङ्क")).unwrap_or(0);
            let entries = self.arena("सञ्चयसंज्ञाप्रविष्टयः");
            let symbols = self.arena("सञ्चयसंज्ञामूल्यानि");
            let global_kind = self.global_int("चरघोषणाभेद");
            for k in 1..=n {
                let e = arena_int(&entries, k);
                let sym = arena_int(&symbols, k);
                if e <= 0 || sym <= 0 {
                    continue;
                }
                let int_call = |it: &mut Interpreter, r: &str| -> i128 {
                    it.call(r, vec![Value::Int(e)], FUEL_SMALL)
                        .ok()
                        .and_then(|v| v.as_int())
                        .unwrap_or(0)
                };
                if int_call(&mut self.it, "घोषणासञ्चयॱप्रविष्टिभेदः") != global_kind
                {
                    continue;
                }
                let mi = int_call(&mut self.it, "घोषणासञ्चयॱप्रविष्टिमण्डलम्");
                let m = self
                    .it
                    .call("घोषणासञ्चयॱमण्डलनाम", vec![Value::Int(mi)], FUEL_SMALL)
                    .ok()
                    .map(|v| text_of(&v))
                    .unwrap_or_default();
                let member = self
                    .it
                    .call("घोषणासञ्चयॱप्रविष्टिनाम", vec![Value::Int(e)], FUEL_SMALL)
                    .ok()
                    .map(|v| text_of(&v))
                    .unwrap_or_default();
                if m.is_empty() || member.is_empty() {
                    continue;
                }
                names
                    .entry(SymbolId(usize::try_from(sym).unwrap_or(0)))
                    .or_insert((m, member));
            }
        }

        // `W-295` — THE TWO NAMES THE EMITTERS OWN, AND THE DRIVER'S TWIN
        // REGISTERS THEM WHILE THIS SIDE DID NOT.
        //
        // `ir.t1:544` states the contract: the record cursor and its region are
        // addressed from the IR as globals, their labels belong to the emitter,
        // and being module-less "the pair ("", "रचनासूचकः") spells it". The `.t1`
        // driver does exactly that in two lines — `shrinkhala.t1:473-474` calls
        // `यन्त्रनामयोजनम्` with `रिक्तमण्डलनाम`, the EMPTY module text — so the
        // `.t1` half has carried these names since `W-284`.
        //
        // **THIS HALF CARRIED NEITHER, AND THE SHAPE OF THE FAILURE IS WHY IT
        // SURVIVED A LANDING.** The two symbols are minted by `ir.t1` as
        // constants; they are in no arena this function reads — not
        // `वृत्तिनामचिह्नककोश`, not `वैश्विकनामकोश` — so the loops above cannot
        // reach them however carefully they are written. `AddrOfGlobal` then
        // called `routine_label` on `SymbolId(10000003)` and the emitter refused
        // the whole module with `UnnamedSymbol`. Measured 2026-09-17: all SEVEN
        // run/slice fixtures in `t1_storage_witness` died there, and the six
        // record ones did not — an allocation lowers through `AllocRecord`,
        // which writes `RECORD_CURSOR` as a LITERAL label and never asks `names`.
        // So the defect was invisible to exactly the construct it was added for.
        //
        // The same symbols already appear by name in `module_allocates` below
        // `riscv64.rs:1343`, which is the clearest evidence this was an omission
        // and not a scope line: the emitter knew to expect an `AddrOfGlobal` of
        // them and had no way to label one.
        //
        // `insert` and not `entry().or_insert()`: nothing else may claim these
        // ids, and if the resolver ever mints one the emitter's own label must
        // win — a silent second meaning for `रचनासूचकः` is the failure that
        // `check_labels` exists to make loud.
        names.insert(
            riscv64::RECORD_CURSOR_SYMBOL,
            (String::new(), riscv64::RECORD_CURSOR.to_string()),
        );
        names.insert(
            riscv64::RECORD_REGION_SYMBOL,
            (String::new(), riscv64::RECORD_REGION.to_string()),
        );

        Ok(Built {
            module: Module {
                globals,
                name: module.to_string(),
                functions: out_functions,
                names,
                entry: None,
            },
            declared,
            parameters,
        })
    }
}

/// A global as a number, ० when the corpus does not declare it.
fn g_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name).and_then(Value::as_int).unwrap_or(0)
}

/// A guard: TRUE only when the corpus declares it AND it is raised. A missing
/// global reads FALSE, so a reader of an older corpus is told "no record"
/// rather than handed a site out of an arena nobody filled.
fn g_bool(it: &Interpreter, name: &str) -> bool {
    matches!(it.global(name), Some(Value::Bool(true)))
}

/// A `अङ्कः अन्तः अ८` global as text. Every one of these is declared `भवति ०`
/// — type-directed for the empty run — so the caller MUST test the guard
/// beside it: an empty string here is "not recorded", not "the empty name".
fn g_text(it: &Interpreter, name: &str) -> String {
    match it.global(name) {
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        _ => String::new(),
    }
}

/// **WHERE THE RESOLVER REFUSED**, as `अर्थ` recorded it — the name and the
/// line, never a bare "something is undeclared".
///
/// READ ONLY AFTER A REFUSAL OF THE STAGE THAT WROTE IT. Both guards are
/// cleared by `अर्थॱनिर्णायकारम्भः`, which `शृङ्खलाॱनिर्णयः` and
/// [`Front::resolve`] both call on entry, so the record belongs to the last
/// program resolved and to no earlier one. A caller that asks after a
/// SUCCESSFUL resolve is answered `None`, and that is the case the test drives.
#[must_use]
pub fn resolve_site(it: &Interpreter) -> Option<String> {
    if g_bool(it, "पुनर्घोषणामस्ति") {
        return Some(format!(
            "`{}` is declared twice in this module, at line {} and again at line {}",
            g_text(it, "पुनर्घोषणानाम"),
            g_int(it, "पूर्वघोषणापङ्क्ति"),
            g_int(it, "पुनर्घोषणापङ्क्ति")
        ));
    }
    if g_bool(it, "अनिर्णीतमस्ति") {
        return Some(format!(
            "`{}` at line {} has no declaration",
            g_text(it, "अनिर्णीतनाम"),
            g_int(it, "अनिर्णीतपङ्क्ति")
        ));
    }
    None
}

/// **WHERE THE TYPE CHECKER REFUSED.** Three records in the order the checker
/// reaches them: the routine whose body and return kind disagree, the field a
/// struct does not declare, and the first statement that poisoned a block.
///
/// `प्रकारदोष…` is cleared by `कार्यक्रमप्रकारपरीक्षा` itself; the other two by
/// `अर्थॱप्रकारपरीक्षकारम्भः`, which `शृङ्खलाॱनिर्णयः` and [`Front::typecheck`]
/// both call on entry. Same rule as [`resolve_site`]: ask after the refusal.
#[must_use]
pub fn typecheck_site(it: &Interpreter) -> Option<String> {
    if g_bool(it, "प्रकारदोषमस्ति") {
        return Some(format!(
            "`{}`: body kind {} against return kind {}",
            g_text(it, "प्रकारदोषनाम"),
            g_int(it, "प्रकारदोषशरीरभेद"),
            g_int(it, "प्रकारदोषप्रत्यागमनभेद")
        ));
    }
    if g_bool(it, "असत्क्षेत्रमस्ति") {
        return Some(format!(
            "`{}` is no field of `{}`, at line {}",
            g_text(it, "असत्क्षेत्रनाम"),
            g_text(it, "असत्क्षेत्रवस्तु"),
            g_int(it, "असत्क्षेत्रपङ्क्ति")
        ));
    }
    // **THE REGISTRY HELD NO FIELD LIST AT ALL**, which is `घोषणासञ्चय`'s defect
    // and not the program's — named apart so a reader is sent to the right file.
    if g_bool(it, "असञ्चितक्षेत्रमस्ति") {
        return Some(format!(
            "`{}` at line {}: the collected registry holds no field list for that \
             struct — `घोषणासञ्चय` exported the type without its fields",
            g_text(it, "असञ्चितक्षेत्रनाम"),
            g_int(it, "असञ्चितक्षेत्रपङ्क्ति")
        ));
    }
    // `ऋण` lowers to `० − x`, so its operand has to be a number.
    if g_bool(it, "अऋण्यमस्ति") {
        return Some(format!(
            "`ऋण` on a non-numeric operand: its type reads kind {}",
            g_int(it, "अऋण्यभेद")
        ));
    }
    if g_bool(it, "दुष्टवाक्यमस्ति") {
        return Some(format!(
            "a statement of kind {} poisoned its block (cause {}, first token {})",
            g_int(it, "दुष्टवाक्यभेद"),
            g_int(it, "दुष्टकारण"),
            g_int(it, "दुष्टवाक्यस्थान")
        ));
    }
    None
}

/// **THE SITE OF THE `.t1` CHAIN'S REFUSAL, FOR A DRIVER THAT DID NOT USE
/// [`Front`].** `SAS-013`: `t1_boot --object` and `t1_image` call
/// `शृङ्खलाॱमण्डलसङ्कलनम्` directly, so neither ever saw the messages
/// [`Front::resolve`] and [`Front::typecheck`] build — a refused module came
/// back as `सङ्कलनविरामभेद = 2` and no more, and a builder bisected instrument
/// variants for an hour to find a call to a routine nobody declared.
///
/// **THE STAGE DECIDES WHICH RECORD IS ASKED, AND THAT IS NOT FUSSINESS.**
/// `निर्णयविरामभेद` is `१` for resolve and `२` for typecheck. Asking all five
/// records at once would report a STALE typecheck site on a resolve refusal:
/// one interpreter compiles the whole corpus, `कार्यक्रमप्रकारपरीक्षा` is not
/// even entered when resolve refuses, and `प्रकारदोषमस्ति` would still stand
/// from an earlier source. The ledger has paid for that shape twice
/// (`अयुग्मशाखासंख्या` 337 against a true 53) and does not pay for it here.
///
/// A stage this does not know — `०` (both passed), `३` (never entered, or an
/// exit `निर्णयः` did not record) — is answered `None`. There is no fallback
/// sweep: naming a site the refusal cannot own is the defect, not the cure.
#[must_use]
pub fn refusal_site(it: &Interpreter) -> Option<String> {
    // R-15-1 FIRST, AND THE STAGE GATE'S OWN ARGUMENT IS WHY (W-304).
    // `अक्षरकोशॱपरिधिपदविभाग` refuses a source whose octets leave the
    // repertoire and `पठनम्` returns BEFORE `निर्णयः` runs, so
    // `निर्णयविरामभेद` still holds the PREVIOUS source's verdict on exactly
    // that path — the stale-record failure this routine's contract test names,
    // arriving from the other direction. `परिधिदोषस्थितम्` cannot go stale the
    // same way: every source is lexed through that one entry (both `पठनम्` and
    // [`Front::lex`]) and it resets the record on entry.
    if g_bool(it, "परिधिदोषस्थितम्") {
        return Some(format!(
            "repertoire: octet {} is outside R-15-1",
            g_int(it, "परिधिदोषस्थानम्")
        ));
    }
    // THE PARSER SECOND, FOR THE GATE'S OWN REASON (W-342). A source the parser
    // refused returns from `पठनम्` before `निर्णयः` runs too, so everything
    // below this line would answer about the PREVIOUS source — and on
    // `t1_image`'s isolated re-compile that answer was "RESOLVED AND
    // TYPECHECKED when re-compiled ALONE … LOAD-DEPENDENT", a confident
    // sentence about a source that never parsed.
    if let Some(s) = parse_site(it) {
        return Some(format!("parse: {s}"));
    }
    if let Some(s) = emit_site(it) {
        return Some(s);
    }
    match g_int(it, "निर्णयविरामभेद") {
        1 => resolve_site(it).map(|s| format!("resolve: {s}")),
        2 => typecheck_site(it).map(|s| format!("typecheck: {s}")),
        // Both decide stages passed, so the IR stage ran on THIS source and
        // its record is this source's (`मध्यरूपॱआरम्भः` clears it per program).
        0 => file_mismatch_site(it).map(|s| format!("ir: {s}")),
        _ => None,
    }
}

/// **`V-005` — THE ONE NAMED CAUSE A FILE MISMATCH IS REFUSED BY**, as `ir.t1`
/// recorded it: a float bound or assigned to an integer local or the reverse
/// (१), an argument against the callee's declared parameter type (२), a
/// returned value against the declared return type (३), and — `V-008` — a
/// value stored into a record field (४), a run element (५) or a module global
/// (६) against the slot's declared type, a global's initialiser included. The
/// interpreter refuses the same shapes with the same word (`nirvahana.rs`), and
/// the emitters' `Refusal::FileMismatch` covers what the IR does not see
/// (integer operators, addresses, branch conditions, a narrow float store).
#[must_use]
pub fn file_mismatch_site(it: &Interpreter) -> Option<String> {
    if !g_bool(it, "वर्गविरोधमस्ति") {
        return None;
    }
    let shape = match g_int(it, "वर्गविरोधभेद") {
        1 => "a value stored into a local declared in the other register file",
        2 => "an argument in the other register file from its declared parameter",
        3 => "a returned value in the other register file from the declared return type",
        // `V-008`: memory, the three shapes a `प६४` slot can be declared in.
        4 => "a value stored into a record field declared in the other register file",
        5 => "a value stored into a run element declared in the other register file",
        6 => {
            "a value stored into (or initialising) a module global declared in the other register file"
        }
        _ => "an unnumbered shape",
    };
    // The line `ir.t1` read AT THE REFUSAL: the token arena may hold another
    // source by the time a driver asks.
    let line = g_int(it, "वर्गविरोधपङ्क्तिः");
    Some(format!(
        "FileMismatch: {shape}, at line {line} — the explicit bit move \
         `अष्टकॱप्लवसंचारः` is the way across"
    ))
}

/// **WHERE THE PARSER REFUSED**, as `व्याकर` recorded it — `W-342`.
///
/// The record is `दोषकोश`, entries १ to `दोषसूचकाङ्क`, each a line, a column
/// and a reason. It had readers in the tests and none in the drivers: a
/// parse-refused source reached `t1_image` as "declared nothing" and
/// `t1_boot` as a third-state line about the decide stage.
///
/// `दोषसूचकाङ्क` IS THE GUARD, NOT THE DRIVER'S EXIT KIND. `कार्यक्रमपठनम्`
/// zeroes it on entry and `शृङ्खलाॱपठनम्` zeroes it before the repertoire gate,
/// so a count above ० is the last source's own and that source got no
/// further than the parser. Asking `सङ्कलनविरामभेद == ९` instead would answer
/// `None` for the Rust front half, which parses without setting it.
///
/// THE FIRST ENTRY IS THE SITE. The parser recovers and may record more; the
/// count is printed so a reader knows there is more to find, and the first is
/// named because later ones are often its consequences.
#[must_use]
pub fn parse_site(it: &Interpreter) -> Option<String> {
    let n = g_int(it, "दोषसूचकाङ्क");
    if n <= 0 {
        return None;
    }
    let first = match it.global("दोषकोश") {
        Some(Value::Arena(a)) => a.borrow().get(1).cloned(),
        _ => None,
    }?;
    let more = if n > 1 {
        format!(" ({} more recorded after it)", n - 1)
    } else {
        String::new()
    };
    Some(format!(
        "line {}, column {}: {}{more}",
        int_of(&first, "पङ्क्ति"),
        int_of(&first, "अक्षर"),
        text_of(&member(&first, "कारण")),
    ))
}

/// **THE LINKER'S REFUSALS OF THE LAST LINK, AS TEXT** — `W-342`.
///
/// `संयोजन` appends one record per problem to `संयोजनदोषकोश`: a code, the name
/// it is about, a place and a number. `t1_image` printed the arena with
/// `{:?}` cut at 1200 characters, so the name of the undefined symbol reached
/// the operator as a list of octets, and usually not all of it.
///
/// WALKED TO THE CURSOR, NOT TO THE LENGTH. `स्थानसंयोजनम्` zeroes
/// `संयोजनदोषसूचकाङ्क` on entry and does not clear the arena
/// (`samyojana.t1:794-799`), so entries past the cursor are an EARLIER link's.
///
/// THE CODE IS PRINTED AS ITS NUMBER AND NOT AS PROSE. The meanings are
/// `samyojana.t1`'s `*कूटः` constants; a second copy of that table here would
/// be a twin that goes stale the first time a code is added.
#[must_use]
pub fn link_refusals(it: &Interpreter) -> Vec<String> {
    let n = usize::try_from(g_int(it, "संयोजनदोषसूचकाङ्क")).unwrap_or(0);
    let store = match it.global("संयोजनदोषकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        _ => return Vec::new(),
    };
    store
        .iter()
        .skip(1)
        .take(n)
        .map(|r| {
            let name = text_of(&member(r, "नाम"));
            let line = format!(
                "code {} `{}` at {}, value {}",
                int_of(r, "कूट"),
                name,
                int_of(r, "स्थानाङ्कः"),
                int_of(r, "मूल्यम्"),
            );
            // `V-009` (ii): a duplicate (code २, `पुनरुक्तकूटः`) named
            // `<module><member>` for a kernel member. The record holds only the
            // name — not which objects defined it — so this half cannot see
            // that one of them is the module's matrix kernel. It says ONLY the
            // conditional: two modules whose labels concatenate, or one name in
            // two sources of a module, are duplicates with no kernel at all.
            match (int_of(r, "कूट"), kernel_member_of(&name)) {
                (2, Some((_, member))) => format!(
                    "{line}; if one definition is its module's matrix kernel: {}",
                    kernel_name_reserved(member)
                ),
                _ => line,
            }
        })
        .collect()
}

/// The EMITTER's refusal, which had a full record and no reader — `W-331`.
///
/// # The defect this closes
///
/// `refusal_site` above asked only `निर्णयविरामभेद`, the DECIDE stage's verdict.
/// A source that resolves and typechecks and is then refused by the emitter
/// therefore answered `०` — `निर्णयसिद्धभेद`, *"resolved AND typechecked"* — and
/// `t1_image` printed `no site recorded`, after spending 347,659,457 extra steps
/// re-compiling to look for one. Measured 2026-09-29 on `W-330`.
///
/// **THE RECORD WAS ALREADY COMPLETE.** `yantrotsarjana.t1:167-173` declares six
/// globals and `यन्त्रनिषेधः` (`:178`) fills five of them on every refusal: the
/// variant, the ROUTINE LABEL, the block, the target and a number whose meaning
/// is per-variant (`bytes` for a branch, `params` for an entry, `symbol` for an
/// unnamed one). So the site existed in the interpreter and nothing read it.
///
/// `यन्त्रनिषेधमस्ति` IS THE GUARD AND NOT THE KIND, deliberately. Kind `०` is
/// not "no refusal" — `:168` initialises the variant to `०` while the numbered
/// kinds start at `१` (`:105`), so a corpus that refused with an unrecognised
/// variant must still be reported rather than read as silence.
///
/// The variant names come from `yantrotsarjana.t1:105-114`, which pairs each with
/// its Rust `Refusal` twin; `crates/sadhana-t1/tests/t1_sources.rs:2164-2173`
/// asserts that pairing, so this table has a contract test behind it rather than
/// a transcription.
fn emit_site(it: &Interpreter) -> Option<String> {
    if !g_bool(it, "यन्त्रनिषेधमस्ति") {
        return None;
    }
    let kind = g_int(it, "यन्त्रनिषेधभेद");
    let name = match kind {
        1 => "Unreachable",
        2 => "NoTerminator",
        3 => "TargetNotInFunction",
        4 => "UnnamedSymbol",
        5 => "LabelCollision",
        6 => "ParamAfterCall",
        7 => "ParamOutsideEntry",
        8 => "FrameTooLarge",
        9 => "EntryTakesParameters",
        10 => "BranchOutOfRange",
        11 => "JumpOutOfRange",
        12 => "StoreWidthUnnamed",
        13 => "FileMismatch",
        _ => "an unnumbered variant",
    };
    let routine = match it.global("यन्त्रनिषेधवृत्ति") {
        Some(Value::Octets(o)) if !o.as_slice().is_empty() => {
            String::from_utf8_lossy(o.as_slice()).into_owned()
        }
        // NOT A DEFECT, AND I FIRST WROTE IT UP AS ONE. The five `UnnamedSymbol`
        // sites pass `रिक्तम्` here ON PURPOSE (`yantrotsarjana.t1:489`, `:585`,
        // `:1282`, `:1940`, `:2597`): no routine is known at those sites, and the
        // discriminator they carry instead is a SITE CODE in `यन्त्रनिषेधलक्ष्य` —
        // which is what `t1_refusal_site.rs`'s first test exists to check. A
        // `पाठ` global declared `भवति ०` also reads `Int(0)` until something writes
        // text, and both cases are honestly "none" rather than "".
        _ => "-".to_string(),
    };
    // THE THIRD AND FOURTH FIELDS MEAN DIFFERENT THINGS PER VARIANT, and printing
    // them under one pair of names is how a reader is sent to the wrong place.
    // `यन्त्रनिषेधलक्ष्य` is a branch TARGET for the branch variants and a SITE
    // CODE for `UnnamedSymbol`; the globals' own comment at `:172` says
    // `यन्त्रनिषेधसंख्या` is "`param` / `bytes` / `params` / `symbol`". Measured:
    // printing `target 1` for a site code read as "the target is block 1".
    let (third, fourth) = match kind {
        4 => ("site", "symbol"),
        5 => ("first-holder", "-"),
        6 | 7 => ("-", "param"),
        8 => ("-", "frame octets"),
        9 => ("-", "params"),
        10 => ("target", "bytes"),
        13 => ("-", "value"),
        _ => ("target", "n"),
    };
    let site = format!(
        "emit: {name} ({kind}) in `{routine}`, block {}, {third} {}, {fourth} {}",
        g_int(it, "यन्त्रनिषेधपर्व"),
        g_int(it, "यन्त्रनिषेधलक्ष्य"),
        g_int(it, "यन्त्रनिषेधसंख्या"),
    );
    match reserved_kernel_name(it, kind, &routine) {
        Some(why) => Some(format!("{site}: {why}")),
        None => Some(site),
    }
}

/// `V-009` (ii): a `LabelCollision` whose label is one of the module's matrix
/// kernel routines' — a user routine named by a kernel member in a module that
/// makes a matrix call — said as the interpreter says it
/// ([`crate::t1::nirvahana::kernel_name_refusal`]).
///
/// `यन्त्रचिह्नपरीक्षा` records the label as the routine (`यन्त्रनिषेधवृत्ति`),
/// the second holder's routine index as the block and the first's as the
/// target; a holder is the kernel when its `वृत्तिकोश` record carries the symbol
/// `ir.t1` gave the kernel (`riscv64::MATRIX_PRODUCT_SYMBOL` /
/// `MATRIX_TRANSPOSE_SYMBOL`), and the label is then `<module><member>`.
fn reserved_kernel_name(it: &Interpreter, kind: i128, label: &str) -> Option<String> {
    if kind != 5 {
        return None;
    }
    let functions = match it.global("वृत्तिकोश") {
        Some(Value::Arena(a)) => Rc::clone(a),
        _ => return None,
    };
    let functions = functions.borrow();
    let kernel = [g_int(it, "यन्त्रनिषेधपर्व"), g_int(it, "यन्त्रनिषेधलक्ष्य")]
        .into_iter()
        .filter_map(|i| functions.get(usize::try_from(i).ok()?))
        .find_map(
            |f| match usize::try_from(int_of(f, "नाम")).ok().map(SymbolId) {
                Some(riscv64::MATRIX_PRODUCT_SYMBOL | riscv64::MATRIX_TRANSPOSE_SYMBOL) => {
                    built_routine_name(int_of(f, "नाम"))
                }
                _ => None,
            },
        )?;
    let module = label.strip_suffix(kernel)?;
    Some(crate::t1::nirvahana::kernel_name_refusal(module, kernel))
}

/// What to print when [`refusal_site`] answers `None`, in ONE copy.
///
/// # Why the caller's situation is a parameter and not a detail
///
/// `०` is `निर्णयसिद्धभेद` — `shrinkhala.t1:278`, *"resolved AND typechecked"* —
/// so on the `None` path it says the decide stage SUCCEEDED. What that MEANS
/// depends entirely on what the caller compiled, and the two readings send a
/// reader to different files:
///
/// - `t1_image` re-compiles the first failed source **alone**, deliberately
///   (`t1_image.rs:500-507`: the globals after a whole-corpus build belong to the
///   last source resolved, not the failed one). So `०` there means *this source
///   compiles BY ITSELF* — the refusal is load-dependent and is not in this
///   file's own front half. That is the most useful fact the run has, and the
///   message used to throw it away as "no site recorded".
/// - `t1_boot --object` compiles ONE source and nothing else, so `०` beside empty
///   output means a stage AFTER decide refused, and the file itself is the place
///   to look.
///
/// MEASURED 2026-09-29, and this function exists because of it: `ir.t1 +51`
/// (`W-330`) made the 21-source build print `refused: मध्यरूप: no site recorded
/// (निर्णयविरामभेद = Int(0))` after `diag: 347659457 more step(s)`, and
/// `tools/t1-census.sh ir` then showed `typecheck ok`, 1 assembled — it compiles
/// alone. The instrument had the answer and said "no site recorded", which reads
/// as an instrument failure rather than as a fact about the corpus.
#[must_use]
pub fn refusal_third_state(it: &Interpreter, isolated_recompile: bool) -> String {
    // NOT `g_int`, AND THE DIFFERENCE IS THE WHOLE POINT OF THIS FUNCTION.
    // `g_int` answers `0` for a global that is ABSENT as readily as for one that
    // holds zero (`:1216`, `unwrap_or(0)`), and here `0` carries a strong claim —
    // "decide succeeded". A corpus that never declared `निर्णयविरामभेद` would be
    // told its decide stage passed. The Option is kept, so absent falls to the
    // last arm and says only what it knows.
    let v = it.global("निर्णयविरामभेद");
    let raw = v
        .as_ref()
        .map_or_else(|| "-".to_string(), |v| format!("{v:?}"));
    match v.as_ref().and_then(|v| Value::as_int(v)) {
        Some(0) if isolated_recompile => format!(
            "no site in this file — it RESOLVED AND TYPECHECKED when re-compiled \
             ALONE (निर्णयविरामभेद = {raw}), so the refusal is LOAD-DEPENDENT. \
             Bisect the load with T1_CORPUS, not this source"
        ),
        Some(0) => format!(
            "no site — decide SUCCEEDED (निर्णयविरामभेद = {raw}) yet this source \
             produced no object, so a stage AFTER typecheck refused and records no site"
        ),
        Some(3) => format!(
            "no site — निर्णयः was never entered, or took an exit it does not \
             record (निर्णयविरामभेद = {raw})"
        ),
        _ => format!("no site recorded (निर्णयविरामभेद = {raw})"),
    }
}

/// The module a source declares: `मण्डलम् NAME ॥`, its first line. The name is
/// half of every label the emitter writes (research/25 §2.2), so a source that
/// declares none has to be given one by its caller rather than assigned a
/// plausible default here.
#[must_use]
pub fn module_name(source: &str) -> Option<String> {
    source
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("मण्डलम् "))
        .and_then(|r| r.split_whitespace().next())
        .map(str::to_string)
}

/// A module read out of the arenas, with what the entry choice needs.
struct Built {
    module: Module,
    /// Every routine in declaration order, with its symbol.
    declared: Vec<(String, SymbolId)>,
    /// How many `Param` instructions each routine takes — the emitter refuses
    /// an entry with any, because the startup stub calls it with none.
    parameters: HashMap<SymbolId, usize>,
}

impl Built {
    fn symbol_of(&self, name: &str) -> Option<SymbolId> {
        self.declared
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| *s)
    }

    /// `मुख्य` if it takes none, else the first routine that takes none — the
    /// order the file reads in.
    fn first_zero_parameter(&self) -> Option<SymbolId> {
        let zero = |s: &SymbolId| self.parameters.get(s).copied().unwrap_or(0) == 0;
        self.declared
            .iter()
            .find(|(n, s)| n == "मुख्य" && zero(s))
            .or_else(|| self.declared.iter().find(|(_, s)| zero(s)))
            .map(|(_, s)| *s)
    }
}

/// The binary kinds and the compare, as `मध्यरूप` numbers them — `W-204`'s two
/// and `W-245`'s eleven.
///
/// IT IS NOT TWO PLACES. This margin said the driver's copy and
/// `paradigm_encode.rs`'s "are ONE TABLE IN TWO PLACES". A search of the tree on
/// 2026-09-05 found **FOUR** transcriptions of the instruction-kind reading —
/// here, `yantra/tests/paradigm_encode.rs`, `sadhana-t1/tests/t1_exec_riscv.rs`
/// and `frontend/src/pathana.rs` — with `ir.t1:88-114` the one definition
/// they all copy. The TERMINATOR decode is in the same four; the COMPARE
/// sub-kind is in three. Eleven hand transcriptions of three tables across four
/// files, and nothing links any of them to the definition.
///
/// THE COUNT IS THE FINDING. `ir.t1` gained eleven kinds and three of the four
/// instruction readers were updated by whoever's tests went red. The fourth was
/// `pathana.rs`, which nothing runs over the demonstration programs — so the
/// demo refused five of its own six at IR, naming a kind rather than a fault,
/// while the compiler emitted them correctly and the driver ran all six to their
/// right answers. A table copied four times goes stale wherever no test looks.
fn binary_kind(kind: i128, l: ValueId, r: ValueId, sub: i128, mark: i128) -> Option<Instruction> {
    Some(match kind {
        3 => Instruction::Add(l, r),
        4 => Instruction::Sub(l, r),
        6 => Instruction::Mul(l, r),
        // `W-381` stage 3 (ruling (b)) — KINDS ७ AND ८ ARE TWO INSTRUCTIONS EACH,
        // told apart by `ध्रुवमूल्यम्` as kind १० is: ० signed, १ unsigned (a name
        // declared unsigned on either side). `if`, and the signed variant first,
        // for the kind-table reader's reason given at kind १०.
        7 => {
            if mark == 0 {
                Instruction::Div(l, r)
            } else if mark == 1 {
                Instruction::DivU(l, r)
            } else {
                return None;
            }
        }
        8 => {
            if mark == 0 {
                Instruction::Rem(l, r)
            } else if mark == 1 {
                Instruction::RemU(l, r)
            } else {
                return None;
            }
        }
        9 => Instruction::Shl(l, r),
        // `W-333` — KIND १० IS TWO INSTRUCTIONS, TOLD APART BY `ध्रुवमूल्यम्`: ० the
        // arithmetic shift, १ the logical one `मध्यरूप` builds for a left operand
        // that is a name declared unsigned. Any other value is refused: a
        // decoder that guessed would build `Shr` and compile.
        //
        // WRITTEN AS `if`, NOT AS A NESTED `match mark { 0 => …, 1 => … }`:
        // `t1_transcriptions.rs` reads every `N => Family::Variant` arm in the
        // tree as a row of a kind table, nested ones included, so a nested
        // match here was scanned as "Instruction 0 is `Shr`, Instruction 1 is
        // `ShrL`" and reddened the guard. `Shr` stays FIRST for the same
        // reader: it takes the first variant in the arm as kind १०'s.
        10 => {
            if mark == 0 {
                Instruction::Shr(l, r)
            } else if mark == 1 {
                Instruction::ShrL(l, r)
            } else {
                return None;
            }
        }
        11 => Instruction::And(l, r),
        12 => Instruction::Or(l, r),
        13 => Instruction::Xor(l, r),
        14 => Instruction::Cmp(
            match sub {
                1 => CmpOp::Eq,
                2 => CmpOp::Ne,
                3 => CmpOp::Lt,
                4 => CmpOp::Ge,
                5 => CmpOp::Ltu,
                6 => CmpOp::Geu,
                _ => return None,
            },
            l,
            r,
        ),
        _ => return None,
    })
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        _ => String::new(),
    }
}

fn member(v: &Value, name: &str) -> Value {
    match v {
        Value::Record(r) => r.borrow().get(name).cloned().unwrap_or(Value::Int(0)),
        _ => Value::Int(0),
    }
}

fn int_of(v: &Value, name: &str) -> i128 {
    member(v, name).as_int().unwrap_or(0)
}

fn set_int(v: &Value, name: &str, n: i128) {
    if let Value::Record(r) = v {
        r.borrow_mut().insert(name.to_string(), Value::Int(n));
    }
}

/// `x ॱ क्रमाङ्क` of a `मूल्याङ्क`/`पर्वाङ्क`, as Rust counts it: १-based to
/// 0-based; `None` for ०, which is `मध्यरूप`'s ABSENT.
fn id_of(v: &Value, name: &str) -> Option<usize> {
    let k = int_of(&member(v, name), "क्रमाङ्क");
    usize::try_from(k).ok().filter(|k| *k > 0).map(|k| k - 1)
}

fn arena_int(a: &Rc<RefCell<Vec<Value>>>, i: usize) -> i128 {
    a.borrow().get(i).and_then(Value::as_int).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("crates/sadhana has a grandparent")
            .join("spec")
    }

    fn front() -> Front {
        Front::load(&spec_root()).expect("the T1 front end loads against `spec`")
    }

    /// **THE DRIVER RUNS THROUGH THE PRODUCTION LOADER, NOT A HAND-WRITTEN
    /// LIST.** A driver that loads alone proves nothing: `शृङ्खला` calls
    /// `पदविभाग`, `व्याकर`, `अर्थ`, `मध्यरूप` and `यन्त्रोत्सर्जन`, and this
    /// test would go red if any one of the five left `FRONT_END` — the load
    /// fails on the unresolved import, or the call faults at the stage that
    /// needed it. That is the point of driving it here rather than asserting
    /// the list contains some names, which would pass over a list of strings
    /// naming nothing.
    #[test]
    fn the_loader_carries_the_driver_and_the_five_modules_it_calls() {
        let mut it = Interpreter::load(CHAIN, &spec_root())
            .expect("the whole chain loads: driver plus the modules it calls");
        let text = it
            .call(
                "शृङ्खलाॱमण्डलसङ्कलनम्",
                vec![
                    Value::Octets(Octets::new("मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ योगः ४ ।\nइति\n".as_bytes())),
                    Value::Octets(Octets::new("क".as_bytes())),
                ],
                60_000_000_000,
            )
            .expect("the driver runs against the loader's own module set");
        let text = match text {
            Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
            other => panic!("the driver answered no octets: {other:?}"),
        };
        assert!(
            text.contains("कग"),
            "the driver emitted no label for `कग`, so a stage returned the \
             empty run — which is exactly what an absent module looks like \
             from here:\n{text}"
        );
    }

    /// ACCEPTS, and bumps `अयुग्मशाखासंख्या` by exactly one. The mid-body `यदि`
    /// has a `न६४` arm and a `बूल` arm, which the join cannot meet — and
    /// mid-body, where the value is discarded, `artha.t1` COUNTS the
    /// disagreement and answers `शून्यार्थः` rather than poisoning the block
    /// (`W-231`'s tail-position refinement). So the source typechecks and still
    /// leaves a number behind, which is the shape a carry is visible in.
    const UNJOINED_A: &str = "मण्डलम् अ ॥\nसार्वजनिक वृत्तिः क ददाति न६४ आदि\n    यदि सत्यम् आदि\n        १ ।\n    इति अन्यथा आदि\n        सत्यम् ।\n    इति\n    प्रत्यागमनम् २ ।\nइति\n";

    /// The same shape under different names, so one `Front` can take both:
    /// `व्याकर`'s declaration store is shared and a redeclared name is refused.
    const UNJOINED_B: &str = "मण्डलम् आ ॥\nसार्वजनिक वृत्तिः ख ददाति न६४ आदि\n    यदि सत्यम् आदि\n        १ ।\n    इति अन्यथा आदि\n        सत्यम् ।\n    इति\n    प्रत्यागमनम् २ ।\nइति\n";

    /// REFUSED: `घ` is `न६४` and a `यदि` condition must be `बूल`, so the
    /// condition rule fires in every position. It leaves `अबूलशर्तसंख्या` 1,
    /// `दुष्टकारण` 1 and `पुच्छे` FALSE behind it.
    const REFUSED: &str = "मण्डलम् इ ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः घ ॱॱ न६४ भवति १ ।\n    यदि घ आदि\n        २ ।\n    इति\n    प्रत्यागमनम् ३ ।\nइति\n";

    fn to_typecheck(f: &mut Front, src: &str) -> Result<(), String> {
        f.lex(src)?;
        f.parse()?;
        f.resolve()?;
        f.typecheck()
    }

    fn to_resolve(f: &mut Front, src: &str) {
        f.lex(src).expect("lex");
        f.parse().expect("parse");
        f.resolve().expect("resolve");
    }

    /// The typecheck walk with NO reset — what `typecheck` did before `W-270`.
    /// It is here so the test below can measure the counterfactual rather than
    /// assert that the fix is a fix.
    fn walk_without_reset(f: &mut Front) -> Value {
        let declarations = f.declarations;
        f.it.call(
            "अर्थॱकार्यक्रमप्रकारपरीक्षा",
            vec![Value::Int(declarations)],
            FUEL_RESOLVE,
        )
        .expect("the walk runs")
    }

    /// `W-270`'s acceptance: TWO SOURCES THROUGH ONE `Front`, AGAINST TWO.
    ///
    /// `Front::lex` takes `&mut self` and nothing in the type stops a caller
    /// reusing one — `bin/t1_build.rs` builds a fresh `Front` per source, which
    /// is a convention and not a guarantee. This drives both ways and compares.
    ///
    /// THE VERDICTS ARE NOT WHERE THIS CAN FAIL, and saying so is the point:
    /// `कार्यक्रमप्रकारपरीक्षा` clears the five globals `typecheck` reports from,
    /// so a verdict comparison passes with or without the reset. The COUNTERS
    /// are where the difference lives, so they are asserted too — without them
    /// this test would be a label rather than a contract.
    #[test]
    fn two_sources_through_one_front_answer_as_two_fronts() {
        // Two `Front`s, one source each — the reference.
        let mut fa = front();
        let verdict_a = to_typecheck(&mut fa, UNJOINED_A);
        let (unjoined_a, unbool_a, cause_a) = (
            fa.global_int("अयुग्मशाखासंख्या"),
            fa.global_int("अबूलशर्तसंख्या"),
            fa.global_int("दुष्टकारण"),
        );

        let mut fb = front();
        let verdict_b = to_typecheck(&mut fb, UNJOINED_B);
        let (unjoined_b, unbool_b, cause_b) = (
            fb.global_int("अयुग्मशाखासंख्या"),
            fb.global_int("अबूलशर्तसंख्या"),
            fb.global_int("दुष्टकारण"),
        );

        assert_eq!(verdict_a, Ok(()), "`अ` typechecks on its own");
        assert_eq!(verdict_b, Ok(()), "`आ` typechecks on its own");
        assert_eq!(
            (unjoined_a, unbool_a, cause_a),
            (1, 0, 0),
            "one unjoined mid-body branch, no non-बूल condition, no poison"
        );

        // ONE `Front`, both sources, in order.
        let mut one = front();
        let first = to_typecheck(&mut one, UNJOINED_A);
        assert_eq!(
            first, verdict_a,
            "the first source answers as it does alone"
        );
        let second = to_typecheck(&mut one, UNJOINED_B);
        assert_eq!(
            second, verdict_b,
            "the second source answers as it does alone"
        );
        assert_eq!(
            (
                one.global_int("अयुग्मशाखासंख्या"),
                one.global_int("अबूलशर्तसंख्या"),
                one.global_int("दुष्टकारण"),
            ),
            (unjoined_b, unbool_b, cause_b),
            "the second source's counters must describe the SECOND source. \
             Without `प्रकारपरीक्षकारम्भः` the unjoined count reads 2 here \
             against `आ`'s own 1 — measured, and the reason `typecheck` calls \
             the reset"
        );
    }

    /// WHAT THE RESET PREVENTS, MEASURED BOTH WAYS.
    ///
    /// `W-270` asked whether the typecheck stage has per-program state at all,
    /// since the right answer to "this stage calls no reset" is a margin if it
    /// has none. IT HAS TEN GLOBALS, `प्रकारपरीक्षकारम्भः` owns them, and the
    /// walk clears a DISJOINT five of its own — so this drives the walk with
    /// the reset withheld and shows the carry, then with it and shows none.
    ///
    /// EXACT NUMBERS AND NOT "SOMETHING CHANGED": the accumulating counter goes
    /// 1 → 2 and the carried cause is 1 over a true 0, and a guard that only
    /// asked whether a number moved would pass on either.
    #[test]
    fn the_checkers_state_does_not_carry_between_two_sources() {
        // (a) THE COUNTER ACCUMULATES WITHOUT THE RESET.
        let mut carried = front();
        to_resolve(&mut carried, UNJOINED_A);
        walk_without_reset(&mut carried);
        assert_eq!(
            carried.global_int("अयुग्मशाखासंख्या"),
            1,
            "`अ` has exactly one unjoined mid-body branch"
        );
        to_resolve(&mut carried, UNJOINED_B);
        walk_without_reset(&mut carried);
        assert_eq!(
            carried.global_int("अयुग्मशाखासंख्या"),
            2,
            "WITHOUT the reset the second source's count is the SUM. This is \
             the defect `W-270` names, exhibited; if it ever reads 1 the walk \
             began clearing its own counters and the call in `typecheck` is \
             redundant rather than load-bearing"
        );

        // (b) AND NOT WITH IT — the same drive through the public stage.
        let mut cleared = front();
        assert_eq!(to_typecheck(&mut cleared, UNJOINED_A), Ok(()));
        assert_eq!(cleared.global_int("अयुग्मशाखासंख्या"), 1);
        assert_eq!(to_typecheck(&mut cleared, UNJOINED_B), Ok(()));
        assert_eq!(
            cleared.global_int("अयुग्मशाखासंख्या"),
            1,
            "the second source's count is the second source's"
        );

        // (c) A REFUSED SOURCE'S CAUSE MUST NOT BE READ AS THE NEXT ONE'S.
        // `दुष्टकारण` and `अबूलशर्तसंख्या` are first-only diagnostics: carried,
        // they attribute a later poison's reason to an earlier program.
        let mut after_refusal = front();
        let refused = to_typecheck(&mut after_refusal, REFUSED);
        assert!(
            refused.is_err(),
            "a `न६४` condition must be refused, or this test measures nothing: \
             {refused:?}"
        );
        assert_eq!(
            (
                after_refusal.global_int("अबूलशर्तसंख्या"),
                after_refusal.global_int("दुष्टकारण"),
                after_refusal.global_bool("पुच्छे"),
            ),
            (1, 1, false),
            "the refusal records one non-बूल condition as cause १ — and leaves \
             `पुच्छे` FALSE, which `W-253`'s margin says cannot happen for a \
             COMPLETED typecheck. It drove only accepting sources; a completed \
             REFUSAL ends false"
        );
        assert_eq!(
            to_typecheck(&mut after_refusal, UNJOINED_B),
            Ok(()),
            "`आ` typechecks after a refused source"
        );
        assert_eq!(
            (
                after_refusal.global_int("अबूलशर्तसंख्या"),
                after_refusal.global_int("दुष्टकारण"),
            ),
            (0, 0),
            "`आ` has no non-बूल condition and poisoned nothing, so both must \
             read 0. Without the reset they read the refused source's 1 and 1"
        );
    }

    /// The reset is handed the resolver this program bound, so the stage refuses
    /// an order that cannot be sound anyway — the checker reads the resolver's
    /// symbol-keyed arenas and there are none before `resolve` runs.
    #[test]
    fn typecheck_before_resolve_is_refused_by_name() {
        let mut f = front();
        f.lex(UNJOINED_A).expect("lex");
        f.parse().expect("parse");
        let stop = f.typecheck().expect_err("typecheck before resolve stops");
        assert!(
            stop.contains("typecheck before resolve"),
            "the stop must name the order, not the interpreter: {stop}"
        );
    }

    /// The two writings of each built routine's symbol agree: the number in
    /// `BUILT_ROUTINES` is the value of the `ir.t1` global its row names. The
    /// growth routine's global must exist; a row whose global `ir.t1` does not
    /// declare yet (the run compare, before its `.t1` half lands) is skipped,
    /// and is checked from the moment it is declared.
    #[test]
    fn every_built_routine_symbol_agrees_with_the_global_ir_t1_declares() {
        let f = front();
        let mut checked = 0;
        for (global, symbol, name) in BUILT_ROUTINES {
            match f.it.global(global) {
                Some(v) => {
                    assert_eq!(
                        v.as_int(),
                        Some(*symbol),
                        "`{global}` and the `{name}` row of BUILT_ROUTINES disagree"
                    );
                    checked += 1;
                }
                None => assert_ne!(
                    *name, "खण्डवृद्धिः",
                    "`{global}` is the growth routine's symbol and ir.t1 must declare it"
                ),
            }
        }
        assert!(
            checked >= 1,
            "at least the growth routine's row was checked"
        );
    }

    /// Put `v` at arena index `at`, growing the arena with ० where it is short.
    fn place(arena: &Rc<RefCell<Vec<Value>>>, at: usize, v: Value) {
        let mut a = arena.borrow_mut();
        while a.len() <= at {
            a.push(Value::Int(0));
        }
        a[at] = v;
    }

    /// `N-004` (owner ruling 2026-10-06: "chain.rs first") — **A SECOND BUILT
    /// ROUTINE IS NAMED BY ITS SYMBOL, NOT AS THE GROWTH ROUTINE.**
    ///
    /// A routine the IR builds has no declaration, so its name token is ०.
    /// Until this test, every token-० routine was given the growth routine's
    /// name, so a module with TWO built routines emitted two identical labels
    /// and the emitter refused it (`LabelCollision`). The second built routine
    /// is made here by copying the growth routine's record under the next
    /// built symbol, `१००००००८` — the module's shared run compare. No `.t1`
    /// builds it yet; this pins the Rust half first.
    #[test]
    fn a_second_built_routine_is_named_by_its_symbol_not_as_the_growth_routine() {
        const SRC: &str = "मण्डलम् क ॥ सार्वजनिक चरः ग ॱॱ अङ्कः अन्तः न६४ भवति ० । \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि ग अङ्कः १ अन्तः भवति ९ । \
प्रत्यागमनम् ग अङ्कः १ अन्तः । इति";
        let mut f = front();
        f.lex(SRC).expect("lex");
        f.parse().expect("parse");
        f.resolve().expect("resolve");
        f.typecheck().expect("typecheck");
        f.build_ir().expect("the IR builds");
        let functions = f.arena("वृत्तिकोश");
        let names = f.arena("वृत्तिनामचिह्नककोश");
        let count = usize::try_from(f.global_int("वृत्तिसूचकाङ्क")).expect("a count");
        let growth = (1..=count)
            .find(|&i| arena_int(&names, i) == 0)
            .expect("a run global's store builds the module's growth routine");
        assert_eq!(
            int_of(&functions.borrow()[growth], "नाम"),
            10_000_005,
            "the growth routine carries `वृद्धिवृत्तिसंज्ञा`"
        );
        let copy = match &functions.borrow()[growth] {
            Value::Record(r) => Value::Record(Rc::new(RefCell::new(r.borrow().clone()))),
            other => panic!("a routine record, not {other:?}"),
        };
        set_int(&copy, "नाम", 10_000_008);
        place(&functions, count + 1, copy);
        place(&names, count + 1, Value::Int(0));
        assert!(
            f.it.set_global("वृत्तिसूचकाङ्क", Value::Int(i128::try_from(count + 1).unwrap())),
            "the routine count is a declared global"
        );
        let module = f.module("क", None).expect("the module reads");
        let text = riscv64::emit_module(&module)
            .expect("two built routines emit, because each has its own label");
        assert!(
            text.contains("कखण्डवृद्धिः") && text.contains("कखण्डसाम्यम्"),
            "the growth routine and the run compare are labelled apart\n{text}"
        );
    }
}
