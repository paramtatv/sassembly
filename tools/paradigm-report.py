#!/usr/bin/env python3
"""Rewrite research/22-stats.md from the paradigm censuses' METRIC lines — `W-206`, `W-214`.

research/23's plan (§1, "Output") rules that every paradigm statistic is a
`METRIC paradigm_<part>_<stat> <value>` line printed by an `#[ignore]` census
test, and that a generator rewrites `research/22-stats.md` from those lines
"so the numbers are never typed by hand". This is the generator. It reads the
`--nocapture` stdout of every census — W-206/207's `t1_paradigm.rs`, W-208's
`t1_paradigm_calls.rs`, W-209's `t1_paradigm_names.rs`, W-210's
`model_graph_census.rs`, W-211's `paradigm_t0.rs`, W-212's
`paradigm_boundary.rs`, W-213's `paradigm_history.rs` — and writes one
document: the plan's 32 statistics with their instrument and verdict, the
paradigm's rules with their verdict, then every metric grouped by part.

BYTE-STABLE. Two runs over the same input write the same bytes: the tables
are sorted, nothing is dated, and no environment (path, commit, clock) is
read. `t1_paradigm.rs` asserts this by running the script twice, and
`crates/metrics/tests/paradigm_report.rs` asserts that the checked-in report
is what the checked-in snapshot (`research/22-stats.census.txt`) generates.
The one thing taken from the input besides METRIC lines is a `# tree: <hash>`
header, so the report can say which tree its numbers were read from.

EXPECTED VALUES AND VERDICTS (W-214). A rule statistic (plan §1) has a
predicted value, 100% or 0 (or a graph fact: 14/14, in-degree 3). The table
shows it beside the measured one and a VERDICT:
  holds             the measured value is the expected one;
  exception listed  it is not, and the exceptions are enumerated in the
                    census output and carried by the row named;
  unmeasurable      the census could not take the number and says why;
  shape             a descriptive statistic; the paradigm predicts nothing.
A rule statistic that departs from its expectation and has NO row is
rendered `exception UNLISTED` — that is the state the plan's §4 forbids.

Usage:  tools/paradigm-report.py <census-output> [<census-output> …]
                                 [--out research/22-stats.md]
        A `-` input reads stdin.

Devanagari note: Python's `re` word boundary `\\b` does not work on
Devanagari, so nothing here uses it; METRIC names are ASCII by construction.
"""

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUT = ROOT / "research" / "22-stats.md"

# ── the censuses ───────────────────────────────────────────────────────────
# part → (row, landing hash, census file, test). The hash is the commit the
# census landed in, cited beside every MEASURED line of research/22 §3-§7.
LANDINGS = {
    "corpus": ("W-206", "ef9222d2", "crates/sadhana-t1/tests/t1_paradigm.rs", "measure_corpus_paradigm"),
    "letters": ("W-206", "ef9222d2", "crates/sadhana-t1/tests/t1_paradigm.rs", "measure_corpus_paradigm"),
    "sequence": ("W-206", "ef9222d2", "crates/sadhana-t1/tests/t1_paradigm.rs", "measure_corpus_paradigm"),
    "spans": ("W-206", "ef9222d2", "crates/sadhana-t1/tests/t1_paradigm.rs", "measure_corpus_paradigm"),
    "operators": ("W-206", "ef9222d2", "crates/sadhana-t1/tests/t1_paradigm.rs", "measure_corpus_paradigm"),
    "signs": ("W-206", "ef9222d2", "crates/sadhana-t1/tests/t1_paradigm.rs", "measure_corpus_paradigm"),
    "loop": ("W-207", "66b2ecd5", "crates/sadhana-t1/tests/t1_paradigm.rs", "measure_corpus_loops"),
    "call": ("W-208", "418aa9fa", "crates/sadhana-t1/tests/t1_paradigm_calls.rs", "measure_corpus_calls"),
    "name": ("W-209", "534ce87d", "crates/sadhana-t1/tests/t1_paradigm_names.rs", "measure_paradigm_names"),
    "model": ("W-210", "e522536e", "crates/sanskrit-text/tests/model_graph_census.rs", "statistic_1 … statistic_4"),
    "halves": ("W-211", "f2bc9ab7", "crates/sadhana/tests/paradigm_t0.rs", "measure_corpus_halves"),
    "t0call": ("W-211", "f2bc9ab7", "crates/sadhana/tests/paradigm_t0.rs", "measure_corpus_calls_and_labels"),
    "label": ("W-211", "f2bc9ab7", "crates/sadhana/tests/paradigm_t0.rs", "measure_corpus_calls_and_labels"),
    "boundary": ("W-212", "b88b199f", "crates/yantra/tests/paradigm_boundary.rs", "census_of_the_machine_boundary"),
    "history": ("W-213", "7d352733", "crates/metrics/tests/paradigm_history.rs", "paradigm_history_census"),
    # W-244 (2026-09-04): W-237's encode census enters the snapshot through the retake script.
    "encode": ("W-237", "784d47ee", "crates/yantra/tests/paradigm_encode.rs", "measure_corpus_encode"),
}

# `paradigm_<part>_…` → the section heading and the plan section it answers,
# in the plan's §2 order.
PARTS = [
    ("corpus", "The corpus", "§0"),
    ("letters", "Sites per letter (research/22 §7)", "§0, §1"),
    ("model", "The model graph (1–4)", "§2.1"),
    ("sequence", "The primary conduit — statements (5)", "§2.2"),
    ("spans", "Span pairing (6)", "§2.2"),
    ("name", "The secondary conduit and the junction — names (7–10, 20, 21)", "§2.3, §2.6"),
    ("loop", "The म loop (11–15)", "§2.4"),
    ("call", "The ट loop (16–19)", "§2.5"),
    ("t0call", "The ट loop on the machine — T0 calling convention", "§6.5"),
    ("boundary", "The ह junction at the machine boundary (22, 23)", "§2.6"),
    ("operators", "Operators by ladder level (24)", "§2.7"),
    ("signs", "Sign purity (25)", "§2.7"),
    ("halves", "The two halves (28–30)", "§2.8"),
    ("label", "The machine's labels — T0 references", "§6.4"),
    ("encode", "The ट loop encoded — the T1 emitter twin over the corpus (W-237)", "§6.5"),
    ("history", "Conservation over history (31, 32)", "§2.9"),
]

# W-211's T0 calling-convention metrics share the `call` prefix with W-208's
# T1 call statistics; the second token tells them apart.
T0_CALL_TOKENS = {
    "jal", "jalr", "ecall", "programs", "window", "read", "temporaries",
    "undecoded", "written",
}

# ── rule statistics: what the paradigm predicts ────────────────────────────
# name → expected value. `0` and `100%` are the plan's two rule values;
# `n/n` means numerator equals denominator; an integer is a graph fact.
# Anything not listed is a shape statistic and shows `–`.
EXPECTED = {
    # W-206 — plan 6, 24, 25 and the instrument's own refusal (L1).
    "paradigm_corpus_files_refused": "0",
    "paradigm_letters_expression_slots_unreached": "0",
    "paradigm_spans_adi_iti_unpaired": "0",
    "paradigm_spans_arabhya_samaptam_unpaired": "0",
    "paradigm_spans_ankah_antah_unpaired": "0",
    "paradigm_spans_uktam_iti_unpaired": "0",
    "paradigm_spans_adi_iti_rate": "100%",
    "paradigm_spans_arabhya_samaptam_rate": "100%",
    "paradigm_spans_ankah_antah_rate": "100%",
    "paradigm_spans_uktam_iti_rate": "100%",
    "paradigm_operators_chained_comparisons": "0",
    "paradigm_signs_nonrepertoire": "0",
    # W-207 — plan 11: a back-edge lands inside its own span (X3).
    "paradigm_loop_backedge_rate": "100%",
    # W-208 — plan 16, 17; and the instrument's own consistency.
    "paradigm_call_routines_missing_return_on_some_path": "0",
    "paradigm_call_routines_without_return_type_that_return": "0",
    "paradigm_call_arity_disagreements": "0",
    "paradigm_call_arity_agreement_rate_x100": "100%",
    "paradigm_call_sites_unresolved": "0",
    "paradigm_call_routines_duplicate_names": "0",
    "paradigm_call_slices_read_as_calls": "0",
    "paradigm_call_declaration_readers_disagree": "0",
    "paradigm_call_sources_with_parse_errors": "0",
    # W-209 — plan 7, 10, 20, 21 (S3) and the replica's agreement with the resolver.
    "paradigm_name_resolution_rate_bare_walked": "100%",
    "paradigm_name_bare_uses_resolved_across_modules": "0",
    "paradigm_name_cross_module_refs_member_missing": "0",
    "paradigm_name_qualified_refs_to_undeclared_module": "0",
    "paradigm_name_imports_of_undeclared_modules": "0",
    "paradigm_name_bare_collisions_by_module": "0",
    "paradigm_name_import_cycles": "0",
    "paradigm_name_import_graph_acyclic": "1",
    "paradigm_name_local_uses_before_declaration": "0",
    "paradigm_name_replica_disagreements": "0",
    "paradigm_name_modules_declared_by_two_files": "0",
    # W-210 — plan 1–4.
    "paradigm_model_edge_derivation_rate": "n/n",
    "paradigm_model_interval_rate": "n/n",
    "paradigm_model_max_in_degree": "3",
    "paradigm_model_positions_at_max_in_degree": "1",
    "paradigm_model_s7_rows_misplaced": "0",
    # W-211 — plan 28 (X4).
    "paradigm_halves_roundtrip_rate": "100%",
    "paradigm_halves_roundtrip_failures": "0",
    "paradigm_halves_compressed_roundtrip_rate": "100%",
    "paradigm_halves_compressed_roundtrip_failures": "0",
    "paradigm_halves_programs_refused_by_encoder": "0",
    # W-212 — plan 22, 23 (S5; ह-14 → ल).
    "paradigm_boundary_shared_stream_runs": "0",
    "paradigm_boundary_shared_stream_fraction": "0",
    "paradigm_boundary_ecall_both_channels": "0",
    "paradigm_boundary_end_bare_reentry_fraction": "0",
    "paradigm_boundary_end_kernel_reentry_fraction": "100%",
    "paradigm_boundary_end_kernel_ledger_disagreements": "0",
    # W-213 — plan 31, 32 (X4 as conservation).
    "paradigm_history_ran": "1",
    "paradigm_history_ledger_sum_changes_annotated_pct": "100%",
    "paradigm_history_ledger_unannotated": "0",
    "paradigm_history_ledger_inconsistent": "0",
    "paradigm_history_ledger_removed": "0",
    "paradigm_history_census_drops_since_w193": "0",
}

# A rule statistic away from its expectation must have its exceptions listed
# in the census output AND carried by a row. name → the row.
EXCEPTIONS = {
    "paradigm_operators_chained_comparisons": "W-229 — nine sites, one idiom `x न्यूनम् y समम् असत्यम्`",
    "paradigm_call_arity_disagreements": "W-227 — a text passed as its triple against a slice parameter",
    "paradigm_call_arity_agreement_rate_x100": "W-227",
    "paradigm_call_slices_read_as_calls": "W-228 — a slice form the parser reads as a call",
    "paradigm_name_bare_collisions_by_module": "W-192 — the runtime keys `global` by bare name",
    "paradigm_name_import_cycles": "W-224 — four 2-cycles, every one through `सङ्केतन`",
    "paradigm_name_import_graph_acyclic": "W-224",
    "paradigm_name_imports_of_undeclared_modules": "row proposed in W-214's delivery — `lib.t1 → पद`; the module lex.t1 declares is `पदविभाग`",
    "paradigm_name_modules_declared_by_two_files": "row proposed in W-214's delivery — `वास्तु` is declared by both ast.t1 and vastu.t1 (S3: two files, one name)",
    "paradigm_halves_compressed_roundtrip_rate": "W-233 — 3-bit register fields decode without the encoder's bias",
    "paradigm_halves_compressed_roundtrip_failures": "W-233",
    "paradigm_boundary_end_kernel_reentry_fraction": "W-219 — `jal x0, .` idle spins halt the scheduler",
    # W-244 (2026-09-04): W-213's history census reads one ledger FALL twice — the
    # commit and the trunk's merge of it — and it is W-237's, annotated there:
    # four T1 sources deleted (the false T0 emitter pair and their twins) with
    # their 30 अ६४ + 12 न६४, 2807 -> 2767. A deletion said out loud is what the
    # conservation rule permits; the census counts it as a removal, which it is.
    "paradigm_history_ledger_removed": "W-237 — four sources deleted with their 42 bindings, 2807 → 2767, annotated (784d47ee); one fall, seen at the commit and at its merge",
    "paradigm_history_ledger_sum_changes_annotated_pct": "W-218 — 2894899b (W-179), `न६४` 321→323 said nowhere",
    "paradigm_history_ledger_unannotated": "W-218",
    "paradigm_history_census_drops_since_w193": "W-194 — IR 15→1 was a redefinition, reported not excused",
}

# Statistics the census could not take, and why. The value printed is the
# instrument's own statement of the gap, not a measurement.
UNMEASURABLE = {
    "paradigm_loop_backedge_instrument": "measured on the AST; the IR back-edge (plan 11 as written) waits on the `यावत्` lowering being walked",
    "paradigm_halves_unparser_candidates": "plan 29 — T1 has no unparser; parse/print cannot round-trip (W-215)",
    "paradigm_boundary_t1_runnable_on_yantra": "plan 23 for T1 — no RISC-V emitter, so no T1 program reaches the machine (W-221)",
    "paradigm_halves_twin_pairs_without_agreement_test": "plan 30 — the agreement RATE is unmeasured; only whether a test exists per pair is counted",
    "paradigm_halves_t1_corpus_t0_readable": "the 16 `.सस` are T1 sources; no T0 chain reads them",
}

# ── the plan's 32 statistics ───────────────────────────────────────────────
# (number, plan section, title, part, key metrics). The key metrics decide the
# statistic's verdict; an empty list means no census produces it.
STATISTICS = [
    (1, "§2.1", "Edge derivation rate", "model", ["paradigm_model_edge_derivation_rate"]),
    (2, "§2.1", "Cycle census", "model", ["paradigm_model_cycles", "paradigm_model_cycles_not_named_by_s7"]),
    (3, "§2.1", "In-degree per position", "model", ["paradigm_model_max_in_degree", "paradigm_model_positions_at_max_in_degree"]),
    (4, "§2.1", "Interval property", "model", ["paradigm_model_interval_rate"]),
    (5, "§2.2", "Statements per routine", "sequence", ["paradigm_sequence_statements_per_routine_median", "paradigm_sequence_statements_per_routine_max"]),
    (6, "§2.2", "Span pairing rate", "spans", [
        "paradigm_spans_adi_iti_rate", "paradigm_spans_arabhya_samaptam_rate",
        "paradigm_spans_ankah_antah_rate", "paradigm_spans_uktam_iti_rate"]),
    (7, "§2.3", "Resolution rate per use", "name", ["paradigm_name_resolution_rate_bare_walked", "paradigm_name_resolution_rate_lower_bound_all_uses"]),
    (8, "§2.3", "Uses per declaration; dead names", "name", ["paradigm_name_dead_names"]),
    (9, "§2.3", "Use–declaration distance", "name", ["paradigm_name_global_uses_before_declaration", "paradigm_name_local_uses_before_declaration"]),
    (10, "§2.3", "Bare-name collisions", "name", ["paradigm_name_bare_collisions_by_module", "paradigm_name_bare_uses_resolved_across_modules"]),
    (11, "§2.4", "Back-edge containment", "loop", ["paradigm_loop_backedge_rate"]),
    (12, "§2.4", "Condition shape (ङ)", "loop", ["paradigm_loop_condition_yavat_level_compare_sites"]),
    (13, "§2.4", "Entry (ण)", "loop", ["paradigm_loop_entry_local_binding_rate"]),
    (14, "§2.4", "Step (न)", "loop", ["paradigm_loop_step_present_rate", "paradigm_loop_step_absent"]),
    (15, "§2.4", "Nesting", "loop", ["paradigm_loop_nesting_max_depth"]),
    (16, "§2.5", "Return presence (व्)", "call", ["paradigm_call_routines_missing_return_on_some_path", "paradigm_call_routines_without_return_type_that_return"]),
    (17, "§2.5", "Arguments per call; arity agreement (र)", "call", ["paradigm_call_arity_agreement_rate_x100", "paradigm_call_arity_disagreements"]),
    (18, "§2.5", "Call graph and SCCs", "call", ["paradigm_call_graph_edges_distinct", "paradigm_call_sccs_larger_than_one", "paradigm_call_routines_self_recursive"]),
    (19, "§2.5", "Return-to-site rate", "call", ["paradigm_call_return_to_site_rate_x100"]),
    (20, "§2.6", "Import graph (य)", "name", ["paradigm_name_import_cycles"]),
    (21, "§2.6", "Exports per module; S3 compliance", "name", [
        "paradigm_name_cross_module_refs_qualified", "paradigm_name_bare_uses_resolved_across_modules",
        "paradigm_name_imports_of_undeclared_modules", "paradigm_name_modules_declared_by_two_files"]),
    (22, "§2.6", "Host boundary purity", "boundary", ["paradigm_boundary_shared_stream_runs", "paradigm_boundary_ecall_both_channels"]),
    (23, "§2.6", "Program end (ह-14 → ल)", "boundary", ["paradigm_boundary_end_bare_reentry_fraction", "paradigm_boundary_end_kernel_reentry_fraction"]),
    (24, "§2.7", "Operator frequency by ladder level; chained comparisons", "operators", ["paradigm_operators_chained_comparisons"]),
    (25, "§2.7", "Sign purity (S1)", "signs", ["paradigm_signs_nonrepertoire"]),
    (26, "§2.7", "Sigil statistics (S2, T0 corpus)", None, []),
    (27, "§2.7", "Type-width distribution versus vowel grade", None, []),
    (28, "§2.8", "Encode/decode round trip", "halves", ["paradigm_halves_roundtrip_rate", "paradigm_halves_compressed_roundtrip_rate"]),
    (29, "§2.8", "Parse/print round trip", "halves", ["paradigm_halves_unparser_candidates"]),
    (30, "§2.8", "Twin agreement", "halves", ["paradigm_halves_twin_pairs_without_agreement_test"]),
    (31, "§2.9", "Ledger conservation over history", "history", ["paradigm_history_ledger_sum_changes_annotated_pct"]),
    (32, "§2.9", "Census monotonicity", "history", ["paradigm_history_census_drops_since_w193"]),
]

# ── the paradigm's rules (research/22 §3, §4, §7) ─────────────────────────
# (rule, where, the claim in one line, the statistics that bear on it, the
# metrics that decide, a standing note). The verdict is computed from the
# metrics; the note carries what was decided in writing (a demotion is a
# ruling in research/22, not a number, so it is stated here as text).
RULES = [
    ("S1", "§3.1", "no operator is a sign, no marker is a word", [25],
     ["paradigm_signs_nonrepertoire"], ""),
    ("S2", "§3.2", "a sigil is the marker of the operand it closes; the conjunct split is the pratyāhāra rule", [26],
     [], "no census produces statistic 26; ADR-0005's 439/2,862 is the only number"),
    ("S3", "§3.3", "resolution keys by (module, name); a bare name is a junction only for one entity", [10, 21],
     ["paradigm_name_bare_uses_resolved_across_modules", "paradigm_name_cross_module_refs_member_missing",
      "paradigm_name_bare_collisions_by_module", "paradigm_name_imports_of_undeclared_modules",
      "paradigm_name_modules_declared_by_two_files"],
     "the corpus obeys it; the runtime's bare keying is the violator; two junction findings need rows"),
    ("S4", "§3.4", "a second sense is admitted only as the inverse direction of the first", [],
     [], "no paradigm statistic; `crates/sanskrit-text/tests/lexicon.rs::no_term_carries_two_senses` holds the lexicon at one sense per word"),
    ("S5", "§3.5", "at every junction the two circuits keep their own channel", [22],
     ["paradigm_boundary_shared_stream_runs", "paradigm_boundary_ecall_both_channels"], ""),
    ("X1", "§4.1", "no construct is delimited by layout or arity; every span is a pair", [6],
     ["paradigm_spans_adi_iti_rate", "paradigm_spans_arabhya_samaptam_rate",
      "paradigm_spans_ankah_antah_rate", "paradigm_spans_uktam_iti_rate"], ""),
    ("X2", "§4.2", "the ladder in the grammar is the table; compare is non-associative", [24],
     ["paradigm_operators_chained_comparisons"], ""),
    ("X3", "§4.3", "a back-edge targets a point inside its own span; a call targets another span and returns", [11, 16],
     ["paradigm_loop_backedge_rate", "paradigm_call_routines_missing_return_on_some_path"], ""),
    ("X4", "§4.4", "every forward pass has a stated inverse; a transformation's proof is conservation", [28, 29, 30, 31],
     ["paradigm_halves_roundtrip_rate", "paradigm_halves_compressed_roundtrip_rate",
      "paradigm_halves_unparser_candidates", "paradigm_history_ledger_sum_changes_annotated_pct"],
     "holds for T0; the T1 inverse does not exist (W-215, W-221)"),
    ("L1", "§7", "a construct that cannot be placed on the letter table is not needed or not understood", [],
     ["paradigm_corpus_files_refused", "paradigm_letters_expression_slots_unreached", "paradigm_model_s7_rows_misplaced"], ""),
    ("L1′", "§8", "a construct placed on a loop must name which backward edge closes it", [2],
     ["paradigm_model_backward_secondary_edges"], "a graph fact (six backward edges close all 21 cycles); no construct census names its edge yet"),
    ("§7 ह-14→ल", "§7", "a program's end re-enters the loop through ल", [23],
     ["paradigm_boundary_end_kernel_reentry_fraction"], "DEMOTED to a description (W-214): the exceptions are idle spins, legitimate programs"),
    ("plan §2.6 (20)", "§7 य", "the import graph is acyclic — a junction is entered from one side", [20],
     ["paradigm_name_import_cycles"], "DEMOTED to a description (W-214): the four cycles are the compiler importing itself, legitimate programs"),
]


def harvest(lines):
    """`METRIC <name> <value>` → {name: value}, the way `crates/metrics` reads it.

    Only `paradigm_` names are taken; a later line for the same name wins,
    exactly as `metrics::harvest` overwrites. A name with no value (a census
    printing an empty list) is kept with the empty string. `# tree: <hash>`
    lines are returned separately.
    """
    out = {}
    trees = []
    for line in lines:
        s = line.strip()
        if s.startswith("# tree:"):
            trees.append(s[len("# tree:"):].strip())
            continue
        if not s.startswith("METRIC "):
            continue
        rest = s[len("METRIC "):]
        parts = rest.split(" ", 1)
        name = parts[0]
        value = parts[1].strip() if len(parts) == 2 else ""
        if name.startswith("paradigm_"):
            out[name] = value
    return out, trees


def part_of(name):
    """`paradigm_spans_adi_iti_rate` → `spans`; W-211's `paradigm_call_jal_*` → `t0call`."""
    rest = name[len("paradigm_"):]
    toks = rest.split("_")
    part = toks[0]
    if part == "call" and len(toks) > 1 and toks[1] in T0_CALL_TOKENS:
        return "t0call"
    return part


def leading(value):
    """The number a value begins with: `32 (5-2 य …)` → `32`; `0 # note` → `0`."""
    return value.split(" ", 1)[0] if value else ""


def as_percent(token):
    """A token as a percentage, or None: `100.0%`, `0.8333`, `95`, `18/19`, `100`."""
    try:
        if token.endswith("%"):
            return float(token[:-1])
        if "/" in token:
            a, b = token.split("/", 1)
            a, b = float(a), float(b)
            return 100.0 if b == 0 else a * 100.0 / b
        v = float(token)
        return v * 100.0 if 0.0 <= v <= 1.0 and "." in token else v
    except ValueError:
        return None


def holds(expected, value):
    """Does `value` meet `expected`?"""
    token = leading(value)
    if expected == "0":
        if token == "":
            return True
        try:
            return float(token) == 0.0
        except ValueError:
            return False
    if expected == "100%":
        p = as_percent(token)
        return p is not None and abs(p - 100.0) < 1e-9
    if expected == "n/n":
        if "/" not in token:
            return False
        a, b = token.split("/", 1)
        return a == b and a != ""
    return token == expected


def verdict(name, value):
    """One of `holds`, `exception listed — …`, `exception UNLISTED`, `unmeasurable — …`, `shape`."""
    if name in UNMEASURABLE:
        return f"unmeasurable — {UNMEASURABLE[name]}"
    if name not in EXPECTED:
        return "shape"
    if holds(EXPECTED[name], value):
        return "holds"
    if name in EXCEPTIONS:
        return f"exception listed — {EXCEPTIONS[name]}"
    return "exception UNLISTED"


def verdict_class(v):
    return v.split(" ", 1)[0]


def worst(verdicts):
    """The verdict a set of metrics earns as a whole."""
    order = ["exception", "unmeasurable", "holds", "shape"]
    classes = {verdict_class(v) for v in verdicts}
    for c in order:
        if c in classes:
            return c
    return "unmeasured"


def statistic_verdict(keys, metrics):
    if not keys:
        return "unmeasured — no census produces it"
    present = [k for k in keys if k in metrics]
    if not present:
        return "unmeasured — its census printed none of: " + ", ".join(f"`{k}`" for k in keys)
    vs = [verdict(k, metrics[k]) for k in present]
    w = worst(vs)
    if w == "exception":
        rows = sorted({EXCEPTIONS[k].split(" ")[0] for k in present if verdict_class(verdict(k, metrics[k])) == "exception" and k in EXCEPTIONS})
        unlisted = [k for k in present if verdict(k, metrics[k]) == "exception UNLISTED"]
        if unlisted:
            return "exception UNLISTED — " + ", ".join(f"`{k}`" for k in unlisted)
        return "exception listed — " + ", ".join(rows)
    if w == "unmeasurable":
        return "unmeasurable"
    if w == "holds":
        return "holds"
    return "shape"


def cite(keys, metrics):
    """`name = value` for each key present, the value cut to its leading token."""
    out = []
    for k in keys:
        if k in metrics:
            v = leading(metrics[k]) if metrics[k] else "none"
            out.append(f"`{k}` {v}")
    return "; ".join(out) if out else "–"


def landing(part):
    if part is None or part not in LANDINGS:
        return "–"
    row, h, f, t = LANDINGS[part]
    return f"{row} `{h}` — `{f}`"


def render(metrics, trees=()):
    known = {p for p, _, _ in PARTS}
    lines = [
        "# 22 — Measured: the paradigm statistics",
        "",
        "**Generated by `tools/paradigm-report.py` from the `METRIC paradigm_*` lines of",
        "every paradigm census — `measure_corpus_paradigm` and `measure_corpus_loops`",
        "(`crates/sadhana-t1/tests/t1_paradigm.rs`), `t1_paradigm_calls.rs`,",
        "`t1_paradigm_names.rs`, `crates/sanskrit-text/tests/model_graph_census.rs`,",
        "`crates/sadhana/tests/paradigm_t0.rs`, `crates/yantra/tests/paradigm_boundary.rs`,",
        "`crates/metrics/tests/paradigm_history.rs`. Do not edit — regenerate from",
        "`research/22-stats.census.txt`; `crates/metrics/tests/paradigm_report.rs` fails when",
        "this file is not what that snapshot generates.** Definitions, expected values and",
        "falsifiers are research/23's plan, §2; the letters are research/22 §7; the",
        "measured lines beside each rule are research/22 §3, §4, §7.",
        "",
        "A rule statistic shows the paradigm's prediction in the *expected* column and a",
        "*verdict*: **holds**, **exception listed** (with the row that carries the",
        "exceptions; the census output names each site), or **unmeasurable** (the census",
        "says why). A shape statistic shows `–` and `shape`. A measured value that departs",
        "from its expectation is a finding with a file and a line in the census's own",
        "output, not a number to explain away; one with no row is rendered",
        "`exception UNLISTED`, which research/23 §4 forbids the report to carry silently.",
        "",
    ]
    if trees:
        lines.append("Snapshot taken at tree " + ", ".join(f"`{t}`" for t in sorted(set(trees))) + ".")
        lines.append("")

    if not metrics:
        lines.append("_No `METRIC paradigm_*` lines were given; nothing is measured._")
        lines.append("")
        return "\n".join(lines)

    # ── verdict summary ──
    rule_verdicts = {}
    lines.append("## The rules of research/22, measured")
    lines.append("")
    lines.append("| rule | where | claim | statistics | decided by | verdict | standing |")
    lines.append("|---|---|---|---|---|---|---|")
    for rule, where, claim, stats, keys, note in RULES:
        v = statistic_verdict(keys, metrics)
        rule_verdicts[rule] = v
        st = ", ".join(str(s) for s in stats) if stats else "–"
        lines.append(f"| **{rule}** | {where} | {claim} | {st} | {cite(keys, metrics)} | {v} | {note or '–'} |")
    lines.append("")
    n_hold = sum(1 for v in rule_verdicts.values() if v == "holds")
    n_exc = sum(1 for v in rule_verdicts.values() if v.startswith("exception"))
    n_unm = sum(1 for v in rule_verdicts.values() if v.startswith("unmeasur"))
    n_dem = sum(1 for _, _, _, _, _, note in RULES if note.startswith("DEMOTED"))
    lines.append(
        f"{len(RULES)} rules: {n_hold} hold, {n_exc} with exceptions listed, "
        f"{n_unm} unmeasurable or unmeasured, {n_dem} demoted to descriptions in writing."
    )
    lines.append("")

    # ── the 32 statistics ──
    lines.append("## The plan's 32 statistics (research/23 §2)")
    lines.append("")
    lines.append("| # | plan | statistic | instrument (row, landing, census) | key values | verdict |")
    lines.append("|---:|---|---|---|---|---|")
    counts = {}
    for n, sec, title, part, keys in STATISTICS:
        v = statistic_verdict(keys, metrics)
        counts[verdict_class(v)] = counts.get(verdict_class(v), 0) + 1
        lines.append(f"| {n} | {sec} | {title} | {landing(part)} | {cite(keys, metrics)} | {v} |")
    lines.append("")
    lines.append(
        "32 statistics: "
        + ", ".join(f"{counts[c]} {c}" for c in ["holds", "exception", "unmeasurable", "unmeasured", "shape"] if c in counts)
        + "."
    )
    lines.append("")

    # ── every metric, by part ──
    sections = []
    for part, title, plan in PARTS:
        rows = sorted(k for k in metrics if part_of(k) == part)
        if rows:
            sections.append((part, title, plan, rows))
    extra = sorted(k for k in metrics if part_of(k) not in known)
    if extra:
        sections.append((None, "Other", "–", extra))

    for part, title, plan, rows in sections:
        lines.append(f"## {title}")
        lines.append("")
        lines.append(f"Plan {plan}. {landing(part)}.")
        lines.append("")
        lines.append("| statistic | measured | expected | verdict |")
        lines.append("|---|---:|---:|---|")
        for k in rows:
            shown = metrics[k].replace("|", "\\|") if metrics[k] else "none"
            lines.append(f"| `{k}` | {shown} | {EXPECTED.get(k, '–')} | {verdict(k, metrics[k])} |")
        lines.append("")

    all_v = [verdict(k, metrics[k]) for k in metrics]
    tally = {}
    for v in all_v:
        c = verdict_class(v)
        tally[c] = tally.get(c, 0) + 1
    lines.append(
        f"{len(metrics)} statistics: "
        + ", ".join(f"{tally[c]} {c}" for c in ["holds", "exception", "unmeasurable", "shape"] if c in tally)
        + "."
    )
    lines.append("")
    return "\n".join(lines)


def main(argv):
    inputs = []
    out = DEFAULT_OUT
    it = iter(argv)
    for a in it:
        if a == "--out":
            out = pathlib.Path(next(it))
        else:
            inputs.append(a)
    if not inputs:
        sys.stderr.write(__doc__)
        return 2
    metrics = {}
    trees = []
    for name in inputs:
        if name == "-":
            m, t = harvest(sys.stdin.read().splitlines())
        else:
            m, t = harvest(pathlib.Path(name).read_text(encoding="utf-8").splitlines())
        metrics.update(m)
        trees.extend(t)
    text = render(metrics, trees)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(text.encode("utf-8"))
    sys.stderr.write(f"{out}: {len(metrics)} statistics\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
