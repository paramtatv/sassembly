//! ॥ THE DIRECTORY-WIDE ALLOW IS GONE — AND IT DID NOT GUARD WHAT IT LOOKED
//! LIKE IT GUARDED ॥
//!
//! `W-274`, 2026-09-05. This file opened with
//! `#![allow(dead_code, missing_docs, unused_variables, clippy::all)]` over the
//! whole T1 directory. Removed, and the twenty-one warnings it had been hiding
//! are fixed or answered at their sites:
//!   ॱ 6 `new_without_default` — `Default` written, delegating to `new()`.
//!   ॱ 4 `manual_contains`, 3 `collapsible_if`, 1 `single_match`, 1 `op_ref` —
//!     rewritten.
//!   ॱ 3 `unused_variables` — bound `_` where the arm genuinely ignores them.
//!   ॱ 3 `dead_code` — `Reader::advance`, `FrameNew`, `TypeChecker::resolver` —
//!     kept, with a SITE-LEVEL allow each carrying its own reason. A blanket
//!     allow says nothing; a site-level allow naming why is a record. The last
//!     of the three is unread AND load-bearing: it is the shared borrow `W-272`
//!     rests on, and deleting it would take the invariant with it.
//!
//! `missing_docs` WAS INERT IN THAT LIST and is not carried forward. It is
//! allow-by-default and `sadhana/src/lib.rs` sets no crate-level `warn`, so one
//! of the four names had never done anything. Listing it implied a guard that
//! did not exist.
//!
//! WHAT THIS DOES NOT BUY, and the next reader needs it in the same breath as
//! the clean lint run: **it does not guard the `pub` case.** `dead_code` does
//! not fire on a `pub` item of a library crate — it is public API and therefore
//! reachable by definition. `Resolver::resolved_symbols` was `pub`, declared,
//! never written and never read, and it appeared NOWHERE in those twenty-one
//! warnings; `NodeId` is the same shape. Those are deleted by `W-274`'s other
//! two units, not by this one. A clean run here means the private dead code is
//! named. It says nothing at all about what `pub` hides.

pub mod anita;
pub mod ast;
pub mod drishya;
pub mod parse;

pub mod resolve;

pub mod abi;
/// `W-381` — the interpreter/native agreement rule, ONE copy, read by
/// `t1_image`'s differential gate and by the engine-agreement ratchet.
pub mod agreement;
pub mod build;
/// `W-256`'s driver: the T1 front end driven from library code, source to
/// emittable module. Lifted from `paradigm_encode.rs`, which was the only
/// thing that drove the chain.
pub mod chain;
pub mod comptime;
/// `W-237`, 2026-09-04 — `emit.rs` was DELETED from this directory, and the line
/// that declared it is this note. It called itself "T0 emission for riscv64"
/// (doc 03 §8 phase 3.4.8) and its text was not T0: `main`, `x5`, `a0` are
/// Latin, `,` is not one of the six T0 signs, `निधेहि`/`प्रत्यावर्तनम्` are in no
/// mnemonic table, so `सङ्केतन` refused every line of it (research/25 §1.2).
/// Its T1 twin `उत्सर्जन ॱ कार्यक्रमोत्सर्जनम्` went with it. The emitter that
/// reaches the machine is `riscv64` below with its twin `yantrotsarjana.t1`,
/// and `crates/yantra/tests/paradigm_encode.rs` measures it over the corpus.
pub mod ir;
pub mod mandala;
/// `D-002j` — the interpreter. Until this existed, NOTHING in the tree executed
/// a `.t1` body: 0 of 256 routines had a body `parse_program` descended into,
/// so every Sassembly ratchet counted routines whose TEXT looked real. A
/// routine with its comparison inverted stayed green across the whole suite.
pub mod nirvahana;
/// `W-203`, 2026-09-04 — `passes.rs` was DELETED from this directory, and the
/// line that would have declared it is this note. It was never declared here,
/// so the compiler never saw it: a file the gate cannot check is a claim, not
/// code. What it claimed: a `PassManager` with `mem2reg`, `constant_fold` and
/// `dce`, i.e. `B-085`'s four passes minus inlining. What it was: 185 lines
/// against an `Instruction` enum that has FIVE variants (`ConstInt`, `Call`,
/// `Add`, `Sub`, `Param`), naming NINE that do not exist — `Alloca`, `Store`,
/// `Load`, `Mul`, `Div`, `Eq`, `Neq`, `Lt`, `Gt` — plus a test written against
/// an `ast::BinaryOp` and a `Declaration::Function { is_public, .. }` that
/// `ast.rs` has never had. Its own margin said `Add(val, val)` "is bad". It
/// arrived on 2026-08-26 in a stash of UNTRACKED files (`2d62b085`, reachable
/// only from `origin/stash-anchor-2026-08-31`), not in any commit that built.
/// Declaring it would have put a second DCE beside the live one in `opt`; the
/// one thing it had right — that a `CondBranch`'s condition is a DCE root —
/// is now in `opt::dce_pass`, with a test that fails without it. `ast.t1:19`,
/// `parse.t1:17` and `sanskrit-text/tests/grammar_t1.rs:958` name this file
/// as the example of a non-twin; they read as history now, and stay.
pub mod opt;
pub mod regalloc;
/// `W-235` — the RISC-V emitter: T1 IR to T0 assembly text that `सङ्केतन`
/// encodes (research/25 §2). The name contains `riscv` because that is how
/// W-212's flip test in `crates/yantra/tests/paradigm_boundary.rs` recognises a
/// second backend; R3 rewrites that test to run the corpus.
pub mod riscv64;
pub mod typecheck;
pub mod types;
pub mod unparse;

pub mod x86_64;
