//! The model graph census — task `W-210`, research/23 plan §2.1 statistics 1–4.
//!
//! Research/22 §1 reads the fourteen Māheśvara sūtras as a graph: fourteen runs
//! of sounds joined into one conduit by the **primary** continuation (each
//! entry to the next; a marker to the first sound of the next run), and closed
//! into cycles by the **secondary** continuation (a marker to the phoneme
//! spelling the same consonant — doc 19 R1, which
//! [`sanskrit_text::phonology::cross_sutra_edges`] derives rather than stores).
//!
//! Research/22 §7 then claims that the cycles that matter are two — the loop at
//! `म` inside sūtra 7 and the loop `5→11→5` through `ट्`/`व्` — with `ह` at 5‑1
//! and 14‑1 feeding one each. That is a claim about a graph of 57 nodes and
//! 70 edges, and a graph that small is enumerated, not argued about. So this
//! file builds the graph from `spec/shiva-sutras.tsv` and prints what is there:
//! every simple cycle with its letters, the in‑degree of every position, and
//! the interval property over every `(start, marker)` pair.
//!
//! # Measuring, not asserting
//!
//! The counts pinned below were read off the enumeration, not written first
//! and made to pass. A refusal case removes one edge and shows the census
//! reporting the loss — without that, a routine that printed the two named
//! cycles from a list would look identical.

use sanskrit_text::phonology::{Entry, cross_sutra_edges, sequence};
use std::collections::BTreeSet;

/// Index in the sūtra sequence, 0‑based over all 57 entries, markers included.
type Node = usize;

/// The model graph: the sequence, and every out‑edge of every node.
struct Graph {
    seq: Vec<Entry>,
    adj: Vec<Vec<Node>>,
}

/// `(line, pos)` of a node, the coordinates research/22 uses.
fn at(g: &Graph, n: Node) -> (u8, u8) {
    (g.seq[n].line, g.seq[n].pos)
}

/// The node with those coordinates.
fn node(g: &Graph, line: u8, pos: u8) -> Node {
    g.seq
        .iter()
        .position(|e| e.line == line && e.pos == pos)
        .unwrap_or_else(|| panic!("no entry at {line}-{pos}"))
}

/// `7-2 म` — how a node is printed.
fn label(g: &Graph, n: Node) -> String {
    let e = g.seq[n];
    format!("{}-{} {}", e.line, e.pos, e.devanagari)
}

/// The primary conduit: every entry to the entry after it. Within a run this
/// is sound to sound; at a run's end it is marker to the first sound of the
/// next run. The last marker, `ल्` at 14‑2, has no successor.
fn primary_edges(seq: &[Entry]) -> Vec<(Node, Node)> {
    (0..seq.len().saturating_sub(1))
        .map(|i| (i, i + 1))
        .collect()
}

/// The secondary conduit: each marker to its full form, by doc 19 R1's rule.
/// Derived by the library from the same TSV; nothing here is a stored map.
fn secondary_edges(seq: &[Entry]) -> Vec<(Node, Node)> {
    let idx = |line: u8, pos: u8| {
        seq.iter()
            .position(|e| e.line == line && e.pos == pos)
            .expect("edge endpoint is in the sequence")
    };
    cross_sutra_edges()
        .iter()
        .map(|e| (idx(e.from.0, e.from.1), idx(e.to.0, e.to.1)))
        .collect()
}

/// Build the graph from both conduits, keeping only the edges `keep` accepts.
fn graph_with(keep: impl Fn((Node, Node)) -> bool) -> Graph {
    let seq = sequence();
    let mut adj = vec![Vec::new(); seq.len()];
    for (a, b) in primary_edges(&seq)
        .into_iter()
        .chain(secondary_edges(&seq))
        .filter(|&e| keep(e))
    {
        adj[a].push(b);
    }
    Graph { seq, adj }
}

/// The whole model graph.
fn graph() -> Graph {
    graph_with(|_| true)
}

/// Every simple cycle, each reported once, rooted at its lowest node.
///
/// A depth‑first walk from each start `s` that only enters nodes above `s`
/// finds each cycle exactly once, at its minimum node. Fine for 57 nodes; a
/// larger graph would want Johnson's algorithm.
fn simple_cycles(g: &Graph) -> Vec<Vec<Node>> {
    fn walk(
        g: &Graph,
        s: Node,
        v: Node,
        path: &mut Vec<Node>,
        on: &mut Vec<bool>,
        out: &mut Vec<Vec<Node>>,
    ) {
        for &w in &g.adj[v] {
            if w == s {
                out.push(path.clone());
            } else if w > s && !on[w] {
                on[w] = true;
                path.push(w);
                walk(g, s, w, path, on, out);
                path.pop();
                on[w] = false;
            }
        }
    }
    let mut out = Vec::new();
    for s in 0..g.seq.len() {
        let mut on = vec![false; g.seq.len()];
        on[s] = true;
        walk(g, s, s, &mut vec![s], &mut on, &mut out);
    }
    out.sort_by_key(|c| (c.len(), c.clone()));
    out
}

/// In‑degree of every node over the given edge set.
fn in_degree(n: usize, edges: &[(Node, Node)]) -> Vec<usize> {
    let mut d = vec![0; n];
    for &(_, b) in edges {
        d[b] += 1;
    }
    d
}

/// Does the cycle contain `a → b` as a consecutive step?
fn cycle_uses(c: &[Node], a: Node, b: Node) -> bool {
    (0..c.len()).any(|i| c[i] == a && c[(i + 1) % c.len()] == b)
}

/// The cycle research/22 §7 calls the small loop: `म ङ ण न म्`.
fn the_ma_loop(g: &Graph) -> Vec<Node> {
    (2..=6).map(|p| node(g, 7, p)).collect()
}

/// The cycle research/22 §7 calls the large loop: `व र ट् ⇢ ट त व्`.
fn the_ta_loop(g: &Graph) -> Vec<Node> {
    [(5, 3), (5, 4), (5, 5), (11, 7), (11, 8), (11, 9)]
        .into_iter()
        .map(|(l, p)| node(g, l, p))
        .collect()
}

#[test]
fn statistic_1_every_secondary_edge_is_derived() {
    let seq = sequence();
    let markers = seq.iter().filter(|e| e.is_marker).count();
    let secondary = secondary_edges(&seq);
    let primary = primary_edges(&seq);
    let run_steps = primary.iter().filter(|&&(a, _)| seq[a].is_marker).count();

    println!("METRIC paradigm_model_nodes {}", seq.len());
    println!("METRIC paradigm_model_markers {markers}");
    println!("METRIC paradigm_model_primary_edges {}", primary.len());
    println!("METRIC paradigm_model_primary_run_steps {run_steps}");
    println!("METRIC paradigm_model_secondary_edges {}", secondary.len());
    println!(
        "METRIC paradigm_model_edge_derivation_rate {}/{}",
        secondary.len(),
        markers
    );

    assert_eq!(seq.len(), 57);
    assert_eq!(markers, 14);
    assert_eq!(secondary.len(), 14, "one derived edge per marker");
    assert_eq!(
        run_steps, 13,
        "13 marker→next-run steps; ल् at 14-2 ends the conduit"
    );
    // Every secondary edge leaves a marker and lands on a sound.
    for &(a, b) in &secondary {
        assert!(seq[a].is_marker, "secondary edge from a non-marker");
        assert!(!seq[b].is_marker, "secondary edge onto a marker");
        assert_eq!(
            seq[a].devanagari.trim_end_matches('\u{094d}'),
            seq[b].devanagari,
            "the marker and its target spell different consonants"
        );
    }
}

#[test]
fn statistic_2_the_simple_cycle_census() {
    let g = graph();
    let cycles = simple_cycles(&g);
    let secondary = secondary_edges(&g.seq);
    let back: Vec<(Node, Node)> = secondary.iter().copied().filter(|&(a, b)| b < a).collect();

    println!("METRIC paradigm_model_cycles {}", cycles.len());
    for c in &cycles {
        let letters: Vec<String> = c.iter().map(|&n| label(&g, n)).collect();
        println!("CYCLE {} {}", c.len(), letters.join(" "));
    }
    let lengths: BTreeSet<usize> = cycles.iter().map(Vec::len).collect();
    for len in &lengths {
        let n = cycles.iter().filter(|c| c.len() == *len).count();
        println!("METRIC paradigm_model_cycles_of_length_{len} {n}");
    }
    println!(
        "METRIC paradigm_model_backward_secondary_edges {}",
        back.len()
    );
    for &(a, b) in &back {
        let n = cycles.iter().filter(|c| cycle_uses(c, a, b)).count();
        let shortest = cycles
            .iter()
            .filter(|c| cycle_uses(c, a, b))
            .map(Vec::len)
            .min()
            .unwrap_or(0);
        println!(
            "METRIC paradigm_model_cycles_through_{}_{} {n} (shortest {shortest})",
            at(&g, a).0,
            at(&g, a).1
        );
    }
    let on_no_cycle: Vec<String> = (0..g.seq.len())
        .filter(|&n| !cycles.iter().any(|c| c.contains(&n)))
        .map(|n| label(&g, n))
        .collect();
    println!(
        "METRIC paradigm_model_nodes_on_no_cycle {} ({})",
        on_no_cycle.len(),
        on_no_cycle.join(" ")
    );

    // Structure that the enumeration exposes, checked rather than assumed.
    assert!(
        !cycles.is_empty(),
        "the secondary edges close no cycle at all"
    );
    for c in &cycles {
        let uniq: BTreeSet<Node> = c.iter().copied().collect();
        assert_eq!(uniq.len(), c.len(), "a reported cycle repeats a node");
        let backs = back.iter().filter(|&&(a, b)| cycle_uses(c, a, b)).count();
        assert_eq!(
            backs, 1,
            "every simple cycle uses exactly one backward edge"
        );
    }

    // The two cycles research/22 names are present and are the two shortest.
    let ma = the_ma_loop(&g);
    let ta = the_ta_loop(&g);
    assert!(cycles.contains(&ma), "the म loop is not among the cycles");
    assert!(
        cycles.contains(&ta),
        "the 5→11→5 loop is not among the cycles"
    );
    assert_eq!(cycles[0], ma, "the shortest cycle is the म loop");
    assert_eq!(cycles[1], ta, "the second shortest is the 5→11→5 loop");

    // But they are two of many. Pinned from the enumeration on 2026-09-03:
    // 21 simple cycles, one per (backward edge, choice of forward shortcuts).
    assert_eq!(
        cycles.len(),
        21,
        "the cycle count moved: re-read the census"
    );
    assert_eq!(lengths.iter().next(), Some(&5));
    assert_eq!(lengths.iter().next_back(), Some(&39));
    assert_eq!(back.len(), 6);
}

#[test]
fn statistic_3_in_degree_per_position() {
    let g = graph();
    let primary = primary_edges(&g.seq);
    let secondary = secondary_edges(&g.seq);
    let all: Vec<(Node, Node)> = primary.iter().chain(&secondary).copied().collect();
    let total = in_degree(g.seq.len(), &all);
    let sec = in_degree(g.seq.len(), &secondary);

    for n in 0..g.seq.len() {
        if total[n] != 1 {
            println!(
                "INDEGREE {} total {} secondary {}",
                label(&g, n),
                total[n],
                sec[n]
            );
        }
    }
    let max = *total.iter().max().expect("non-empty");
    let at_max: Vec<Node> = (0..g.seq.len()).filter(|&n| total[n] == max).collect();
    println!("METRIC paradigm_model_max_in_degree {max}");
    println!(
        "METRIC paradigm_model_positions_at_max_in_degree {}",
        at_max.len()
    );
    let targets: BTreeSet<Node> = secondary.iter().map(|&(_, b)| b).collect();
    println!("METRIC paradigm_model_secondary_targets {}", targets.len());

    // ण at 7-4 is the one position reached three ways: from न's predecessor
    // ङ on the conduit and from both ण् markers, 1-4 and 6-2.
    let na = node(&g, 7, 4);
    assert_eq!(max, 3);
    assert_eq!(
        at_max,
        vec![na],
        "ण (7-4) is the unique in-degree-3 position"
    );
    assert_eq!(sec[na], 2, "two markers spell ण");
    // Every other secondary target is reached once by the conduit and once
    // by its marker; every non-target once; the first vowel not at all.
    for n in 0..g.seq.len() {
        let want = if n == na {
            3
        } else if targets.contains(&n) {
            2
        } else if n == 0 {
            0
        } else {
            1
        };
        assert_eq!(total[n], want, "in-degree of {}", label(&g, n));
        assert_eq!(
            sec[n],
            usize::from(targets.contains(&n)) + usize::from(n == na)
        );
    }
    assert_eq!(targets.len(), 13, "14 edges onto 13 distinct positions");
}

/// How doc 19 R2's `(start, marker)` pairs are keyed — task `W-217`.
///
/// The count depends on four choices the sentence "start at a phoneme, run
/// forward, stop at a named marker" leaves open: whether `ह` at 5‑1 and 14‑1
/// are two starts or one name, whether `ण्` at 1‑4 and 6‑2 are two markers or
/// one name, and whether a marker closing the start's own sūtra counts. Doc 19
/// now states the first variant; the others are here so that a future reader
/// who gets a different number can see which convention they counted under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Convention {
    /// Every phoneme POSITION (43) against every marker POSITION after it
    /// (14); the start's own sūtra counts. Doc 19 R2, as corrected.
    ByPosition,
    /// Start by name, first occurrence (42); marker by name, first occurrence
    /// after the start (13). This is how `phonology::pratyahara` keys.
    ByName,
    /// Position starts (43), name markers (13).
    PositionStartNameMarker,
    /// Name starts (42), position markers (14).
    NameStartPositionMarker,
    /// By position, but a marker in the start's own sūtra does not count.
    LaterSutraOnly,
}

/// Every convention, in the order doc 19's note lists them.
const CONVENTIONS: [Convention; 5] = [
    Convention::ByPosition,
    Convention::ByName,
    Convention::PositionStartNameMarker,
    Convention::NameStartPositionMarker,
    Convention::LaterSutraOnly,
];

/// Phoneme index `0..43` in sūtra order — the coordinate doc 19 R2 uses —
/// or `None` at a marker.
fn phoneme_index(seq: &[Entry]) -> Vec<Option<usize>> {
    let mut k = 0;
    seq.iter()
        .map(|e| {
            if e.is_marker {
                None
            } else {
                k += 1;
                Some(k - 1)
            }
        })
        .collect()
}

/// The pratyāhāra census under one convention: every `(start, marker)` pair
/// it admits, as the phoneme indices of the set each pair names.
fn pratyaharas_under(c: Convention) -> Vec<Vec<usize>> {
    let seq = sequence();
    let index = phoneme_index(&seq);
    let name_keyed_start = matches!(c, Convention::ByName | Convention::NameStartPositionMarker);
    let name_keyed_marker = matches!(c, Convention::ByName | Convention::PositionStartNameMarker);
    let mut out = Vec::new();
    for (s, start) in seq.iter().enumerate() {
        if start.is_marker {
            continue;
        }
        let first_of_name = seq
            .iter()
            .position(|e| !e.is_marker && e.devanagari == start.devanagari);
        if name_keyed_start && first_of_name != Some(s) {
            continue;
        }
        let mut names_seen = BTreeSet::new();
        for (m, marker) in seq.iter().enumerate().skip(s + 1) {
            if !marker.is_marker {
                continue;
            }
            if name_keyed_marker && !names_seen.insert(marker.devanagari) {
                continue;
            }
            if c == Convention::LaterSutraOnly && marker.line == start.line {
                continue;
            }
            // The pratyāhāra: start at s, run forward, stop at m, drop markers.
            out.push((s..m).filter_map(|i| index[i]).collect());
        }
    }
    out
}

/// `(pairs, distinct sets)` under one convention, sets keyed by position.
fn census_under(c: Convention) -> (usize, usize) {
    let sets = pratyaharas_under(c);
    let distinct: BTreeSet<&Vec<usize>> = sets.iter().collect();
    (sets.len(), distinct.len())
}

#[test]
fn statistic_4_every_pratyahara_is_an_interval() {
    let seq = sequence();
    let index = phoneme_index(&seq);
    let sets = pratyaharas_under(Convention::ByPosition);
    let pairs = sets.len();
    let intervals = sets
        .iter()
        .filter(|members| {
            let lo = members[0];
            members.iter().enumerate().all(|(k, &p)| p == lo + k)
        })
        .count();
    let distinct: BTreeSet<&Vec<usize>> = sets.iter().collect();

    println!("METRIC paradigm_model_pratyahara_pairs {pairs}");
    println!("METRIC paradigm_model_pratyahara_intervals {intervals}");
    println!(
        "METRIC paradigm_model_pratyahara_distinct_sets {}",
        distinct.len()
    );
    println!("METRIC paradigm_model_interval_rate {intervals}/{pairs}");

    assert!(
        sets.iter().all(|m| !m.is_empty()),
        "an empty pratyāhāra: two markers are adjacent"
    );
    assert_eq!(
        intervals, pairs,
        "a pratyāhāra that is not a contiguous range"
    );
    // THE RATIFIED CONVENTION AND ITS NUMBER — `ByName`, ruled 2026-09-29.
    //
    // **THIS ASSERTED `ByPosition` UNTIL THAT RULING, AND THE CODE NEVER AGREED
    // WITH IT.** Doc 19 R2 stated 305 while `phonology::pratyahara` has always
    // keyed by name and computed 301 — a four-pair disagreement between the
    // specification and the implementation, standing since 2026-09-04. It was
    // harmless while the masks were a census and stops being harmless under
    // `N-001`, where they become the instruction-selection matcher and this
    // inventory becomes that matcher's alphabet.
    //
    // The ruling is grammatical: an anubandha is a saṁjñā, a NAMED symbolic
    // entity, not a location. So a start is a phoneme by name at first
    // occurrence (42 — the two `ह` at 5‑1 and 14‑1 are ONE name) and a marker is
    // a marker by name at first occurrence after it (13 — the two `ण्` are ONE
    // name).
    assert_eq!(
        census_under(Convention::ByName),
        (301, 301),
        "doc 19 R2 ratifies the BY-NAME convention: 301 pairs, 301 sets — and this \
         is what `phonology::pratyahara` keys, so a disagreement here is the doc \
         and the code parting company again"
    );
    // BY POSITION IS STILL PINNED, because it is correct arithmetic about a
    // different convention and the closed form below derives it. Keeping both
    // means a future reader who counts 305 can see which convention they counted
    // under rather than concluding the census is wrong.
    assert_eq!(
        census_under(Convention::ByPosition),
        (305, 305),
        "the by-position reading: 43 phoneme starts (ह twice) × the marker \
         positions after each (ण् twice), own sūtra included"
    );
    // The 43 phonemes are the 43 starts.
    assert_eq!(index.iter().flatten().count(), 43);
    let counts_per_sutra: Vec<usize> = (1..=14u8)
        .map(|l| seq.iter().filter(|e| e.line == l && !e.is_marker).count())
        .collect();
    assert_eq!(counts_per_sutra, [3, 2, 2, 2, 4, 1, 5, 2, 3, 5, 8, 2, 3, 1]);
    let by_formula: usize = counts_per_sutra
        .iter()
        .enumerate()
        .map(|(r, n)| n * (14 - r))
        .sum();
    assert_eq!(by_formula, 305, "the closed form agrees with the walk");
}

#[test]
fn refused_another_convention_gives_another_count_and_none_gives_doc_19s_313() {
    // Doc 19 R2 carried "313 pairs, 300 distinct sets" from 2026-08-13 until
    // `W-217`. Every convention a reader might reasonably count under is
    // enumerated here with the number it gives, so that a future "the census
    // says 301" is answered by NAME — "you counted by name, as
    // `phonology::pratyahara` keys" — rather than by re-deriving.
    let pinned = [
        (Convention::ByPosition, (305, 305)),
        (Convention::ByName, (301, 301)),
        (Convention::PositionStartNameMarker, (302, 302)),
        (Convention::NameStartPositionMarker, (304, 304)),
        (Convention::LaterSutraOnly, (262, 262)),
    ];
    assert_eq!(pinned.len(), CONVENTIONS.len());
    for (c, want) in pinned {
        let got = census_under(c);
        println!(
            "METRIC paradigm_model_pratyahara_census_{c:?} {}/{}",
            got.0, got.1
        );
        assert_eq!(got, want, "the count under {c:?} moved");
        if c != Convention::ByPosition {
            assert_ne!(
                got,
                (305, 305),
                "{c:?} would be indistinguishable from the pinned convention"
            );
        }
        // Under every convention the sets are distinct and the rate is 100%.
        assert_eq!(got.0, got.1, "{c:?}: two pairs name one set");
    }
    // What doc 19 said, under every convention, is not what any of them says.
    let doc_19_as_written = (313, 300);
    for c in CONVENTIONS {
        assert_ne!(
            census_under(c),
            doc_19_as_written,
            "{c:?} reproduces doc 19's withdrawn 313/300"
        );
    }
    assert!(
        CONVENTIONS.iter().all(|&c| census_under(c).0 != 313),
        "some convention gives 313 pairs"
    );
    assert!(
        CONVENTIONS.iter().all(|&c| census_under(c).1 != 300),
        "some convention gives 300 sets"
    );

    // And why "distinct" has to be BY POSITION: keyed by the names a set
    // contains, ह at 14-1 is ह at 5-1, so हल् and यल् are the same 33 names
    // and 305 collapses to 294. That is the one collapse a name-keyed reader
    // would meet, and it is what `pratyahara("ह", "ल्").len() == 34` for 33
    // sounds is about.
    let seq = sequence();
    let phoneme_names: Vec<&str> = seq
        .iter()
        .filter(|e| !e.is_marker)
        .map(|e| e.devanagari)
        .collect();
    let by_name: BTreeSet<BTreeSet<&str>> = pratyaharas_under(Convention::ByPosition)
        .iter()
        .map(|members| members.iter().map(|&i| phoneme_names[i]).collect())
        .collect();
    println!(
        "METRIC paradigm_model_pratyahara_distinct_name_sets {}",
        by_name.len()
    );
    assert_eq!(by_name.len(), 294, "distinct sets keyed by name");
    assert_eq!(
        sanskrit_text::phonology::pratyahara("ह", "ल्")
            .expect("हल्")
            .len(),
        34,
        "हल् walks ह twice"
    );
}

#[test]
fn ha_feeds_one_circuit_from_each_of_its_two_places() {
    // Research/22 §7's two traces, checked edge by edge against the graph and
    // then against the cycle census.
    let g = graph();
    let cycles = simple_cycles(&g);
    let has = |a: (u8, u8), b: (u8, u8)| g.adj[node(&g, a.0, a.1)].contains(&node(&g, b.0, b.1));

    // From 5-1: ह → य → व, into the large loop.
    assert!(has((5, 1), (5, 2)), "ह → य");
    assert!(has((5, 2), (5, 3)), "य → व");
    assert!(
        the_ta_loop(&g).contains(&node(&g, 5, 3)),
        "व is on the 5→11→5 loop"
    );

    // From 14-1: ह → ल् ⇢ ल → ण् ⇢ ण, into the small loop.
    assert!(has((14, 1), (14, 2)), "ह → ल्");
    assert!(has((14, 2), (6, 1)), "ल् ⇢ ल");
    assert!(has((6, 1), (6, 2)), "ल → ण्");
    assert!(has((6, 2), (7, 4)), "ण् ⇢ ण");
    assert!(
        the_ma_loop(&g).contains(&node(&g, 7, 4)),
        "ण is on the म loop"
    );

    // The junction is not symmetric, and the census says how: ह at 5-1 lies
    // on no cycle at all (nothing points back before य), while ह at 14-1 lies
    // on every cycle closed by ल् — it is a member of the ल circuit, not only
    // a feeder of the म one.
    let ha5 = node(&g, 5, 1);
    let ha14 = node(&g, 14, 1);
    let on5 = cycles.iter().filter(|c| c.contains(&ha5)).count();
    let on14 = cycles.iter().filter(|c| c.contains(&ha14)).count();
    println!("METRIC paradigm_model_cycles_through_ha_5_1 {on5}");
    println!("METRIC paradigm_model_cycles_through_ha_14_1 {on14}");
    assert_eq!(on5, 0);
    assert_eq!(on14, 6);
    // And none of the ल् cycles passes through म itself: they enter at ण.
    let ma = node(&g, 7, 2);
    let la_cycles_through_ma = cycles
        .iter()
        .filter(|c| c.contains(&ha14) && c.contains(&ma))
        .count();
    println!("METRIC paradigm_model_la_cycles_through_ma {la_cycles_through_ma}");
}

/// Where research/22 §7 puts a letter: on one of its two loops, or on one of
/// the two paths from `ह` into them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placement {
    MaLoop,
    TaLoop,
    PathFromHa5,
    PathFromHa14,
}

/// The fifteen rows of research/22 §7's letter table, as the thesis states
/// them: position, letter, and which loop or path the row puts it on.
const SECTION_7: [((u8, u8), &str, Placement); 15] = [
    ((5, 1), "ह", Placement::PathFromHa5),
    ((5, 2), "य", Placement::PathFromHa5),
    ((5, 3), "व", Placement::TaLoop),
    ((5, 4), "र", Placement::TaLoop),
    ((5, 5), "ट्", Placement::TaLoop),
    ((11, 7), "ट", Placement::TaLoop),
    ((11, 8), "त", Placement::TaLoop),
    ((11, 9), "व्", Placement::TaLoop),
    ((7, 2), "म", Placement::MaLoop),
    ((7, 3), "ङ", Placement::MaLoop),
    ((7, 4), "ण", Placement::MaLoop),
    ((7, 5), "न", Placement::MaLoop),
    ((7, 6), "म्", Placement::MaLoop),
    ((6, 1), "ल", Placement::PathFromHa14),
    ((14, 1), "ह", Placement::PathFromHa14),
];

/// `7-2:म:sound` — one cell of the emitted table.
fn cell(g: &Graph, n: Node) -> String {
    let e = g.seq[n];
    let kind = if e.is_marker { "marker" } else { "sound" };
    format!("{}-{}:{}:{kind}", e.line, e.pos, e.devanagari)
}

/// Follow the conduit from `from`, taking the secondary edge out of a marker
/// when one exists, until `until` is reached. The two §7 paths are exactly
/// this walk from each `ह`.
fn path(g: &Graph, from: Node, until: Node) -> Vec<Node> {
    let secondary = secondary_edges(&g.seq);
    let mut out = vec![from];
    let mut v = from;
    while v != until {
        v = secondary
            .iter()
            .find(|&&(a, _)| a == v)
            .map_or(v + 1, |&(_, b)| b);
        out.push(v);
        assert!(out.len() <= g.seq.len(), "the walk did not reach its end");
    }
    out
}

#[test]
fn section_7_letter_table_read_off_the_graph() {
    // The coordinator's addition: emit what §7 asserts as measurement, so the
    // thesis's table can be checked against the graph rather than against
    // the owner's statement.
    let g = graph();
    let cycles = simple_cycles(&g);

    for (k, c) in cycles.iter().enumerate() {
        let cells: Vec<String> = c.iter().map(|&n| cell(&g, n)).collect();
        println!(
            "METRIC paradigm_model_cycle_{k}_positions {}",
            cells.join(" ")
        );
    }
    let via_ya = path(&g, node(&g, 5, 1), node(&g, 5, 3));
    let via_la = path(&g, node(&g, 14, 1), node(&g, 7, 4));
    for (name, p) in [("ha_5_1_to_va", &via_ya), ("ha_14_1_to_na", &via_la)] {
        let cells: Vec<String> = p.iter().map(|&n| cell(&g, n)).collect();
        println!("METRIC paradigm_model_path_{name} {}", cells.join(" "));
    }

    // The paths are what §7 traces, letter for letter.
    let coords = |p: &[Node]| p.iter().map(|&n| at(&g, n)).collect::<Vec<_>>();
    assert_eq!(coords(&via_ya), [(5, 1), (5, 2), (5, 3)]);
    assert_eq!(coords(&via_la), [(14, 1), (14, 2), (6, 1), (6, 2), (7, 4)]);

    // Every row of §7's table, checked: the letter is at that position, and
    // the position is on the loop or path the row puts it on.
    let ma = the_ma_loop(&g);
    let ta = the_ta_loop(&g);
    let mut placed = 0;
    let mut misplaced = Vec::new();
    for ((line, pos), letter, place) in SECTION_7 {
        let n = node(&g, line, pos);
        let here = match place {
            Placement::MaLoop => ma.contains(&n),
            Placement::TaLoop => ta.contains(&n),
            Placement::PathFromHa5 => via_ya.contains(&n),
            Placement::PathFromHa14 => via_la.contains(&n),
        };
        if g.seq[n].devanagari == letter && here {
            placed += 1;
        } else {
            misplaced.push(format!("{line}-{pos} {letter} {place:?}"));
        }
    }
    println!(
        "METRIC paradigm_model_s7_rows_placed {placed}/{}",
        SECTION_7.len()
    );
    println!(
        "METRIC paradigm_model_s7_rows_misplaced {}",
        misplaced.join("; ")
    );
    assert!(
        misplaced.is_empty(),
        "§7 places a letter where the graph does not: {misplaced:?}"
    );

    // What §7 does not say. The two loops it names hold 11 positions; the
    // graph puts 43 positions on some cycle. Everything from य at 5-2 to ल्
    // at 14-2 is on a cycle; only the 14 entries of the vowel runs (sounds
    // and markers) and ह at 5-1 are not. In particular ल (6-1) and ह (14-1),
    // which §7 puts on a path INTO
    // the small loop, lie on six cycles of their own, and ञ (7-1), which §7
    // leaves off the small loop, lies on the ञ् cycle of length 9.
    let on_named: BTreeSet<Node> = ma.iter().chain(&ta).copied().collect();
    let on_any: BTreeSet<Node> = cycles.iter().flatten().copied().collect();
    let unnamed: Vec<String> = on_any
        .difference(&on_named)
        .map(|&n| label(&g, n))
        .collect();
    println!(
        "METRIC paradigm_model_positions_on_named_loops {}",
        on_named.len()
    );
    println!(
        "METRIC paradigm_model_positions_on_any_cycle {}",
        on_any.len()
    );
    println!(
        "METRIC paradigm_model_positions_on_unnamed_cycles_only {} ({})",
        unnamed.len(),
        unnamed.join(" ")
    );
    assert_eq!(on_named.len(), 11);
    assert_eq!(on_any.len(), 43);
    assert_eq!(g.seq.len() - on_any.len(), 14);
    let first_on_cycle = on_any.iter().next().copied().expect("some cycle");
    assert_eq!(
        at(&g, first_on_cycle),
        (5, 2),
        "cycles begin at य; ह 5-1 is outside them"
    );
    let cycles_unnamed = cycles.len() - 2;
    println!("METRIC paradigm_model_cycles_not_named_by_s7 {cycles_unnamed}");
    assert_eq!(cycles_unnamed, 19);
}

#[test]
fn refused_a_graph_missing_the_return_edge_has_no_large_loop() {
    // The census must measure, not recite. Drop व् ⇢ व (11-9 → 5-3) and the
    // 5→11→5 cycle must vanish from the enumeration along with every other
    // cycle that edge closed, while the म loop stays.
    let whole = graph();
    let ret = (node(&whole, 11, 9), node(&whole, 5, 3));
    let cut = graph_with(|e| e != ret);
    let cycles = simple_cycles(&cut);

    assert!(
        !cycles.contains(&the_ta_loop(&cut)),
        "5→11→5 reported without its return edge"
    );
    assert!(
        cycles.contains(&the_ma_loop(&cut)),
        "the म loop does not depend on व्"
    );
    assert_eq!(cycles.len(), 21 - 3, "व् ⇢ व closed exactly three cycles");
    assert!(cycles.iter().all(|c| !cycle_uses(c, ret.0, ret.1)));

    // And the other way: drop म् ⇢ म and only the म loop goes.
    let back = (node(&whole, 7, 6), node(&whole, 7, 2));
    let cut = graph_with(|e| e != back);
    let cycles = simple_cycles(&cut);
    assert!(!cycles.contains(&the_ma_loop(&cut)));
    assert!(cycles.contains(&the_ta_loop(&cut)));
    assert_eq!(cycles.len(), 21 - 1, "म् ⇢ म closed exactly one cycle");
}
