//! **A routine that stores into a run PARAMETER returns the run.**
//!
//! Natively an index store past a run's capacity cuts a NEW block and writes
//! the new base back into the run's own location (`ir.t1`, "GROWTH"). In a
//! callee that location is the PARAMETER, so the caller keeps the old block. The
//! interpreter shares a run by reference and never shows it.
//!
//! It cost the self-hosting fixpoint its link. `samyojana.t1`'s
//! `नामस्थानयोजनम्` appended to the caller's scope table and returned a COUNT;
//! natively the caller's table stayed one entry long, every symbol lookup
//! failed, and the native self-image build's link refused `स्तूपान्तः` and the
//! entry — no image (2026-09-22). The corpus convention, which every other such
//! routine already followed, is to RETURN the run and have the caller assign it:
//! `फलम् भवति अष्टकसंयोगः आरभ्य फलम् ऽ … समाप्तम्`.
//!
//! So: every routine with an index store into a run-typed parameter must declare
//! a run return type — unless it NEVER GROWS the run, which the source states by
//! checking the parameter's length before it stores (`यदि P ॱ दैर्घ्य न्यूनम् …`,
//! refusing when short). That is read from the routine itself rather than
//! listed by name here: a test that names a routine counts as a reference to it
//! and moves `t1_paradigm_names`'s unreferenced-name census.

use std::path::Path;

fn is_run_type(ty: &[&str]) -> bool {
    matches!(ty.first(), Some(&"अङ्कः") | Some(&"पाठ") | Some(&"पाठः"))
        || (ty.first() == Some(&"सम्भाव्य") && is_run_type(&ty[1..]))
}

/// `(routine, parameter, line)` for every index store into a run parameter in
/// a routine whose declared return type is not a run.
fn offending(src: &str) -> Vec<(String, String, usize)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        let head = l.strip_prefix("सार्वजनिक ").unwrap_or(l);
        let Some(rest) = head.strip_prefix("वृत्तिः ") else {
            i += 1;
            continue;
        };
        let routine = rest.split_whitespace().next().unwrap_or("").to_string();
        let Some((_, after)) = rest.split_once(" आदाय ") else {
            i += 1;
            continue;
        };
        let (params_text, ret_text) = after.split_once(" ददाति ").unwrap_or((after, ""));
        let ret: Vec<&str> = ret_text
            .trim_end_matches("आदि")
            .split_whitespace()
            .take_while(|w| *w != "आदि")
            .collect();
        let toks: Vec<&str> = params_text
            .split_whitespace()
            .filter(|t| *t != "ऽ")
            .collect();
        let mut params = Vec::new();
        let mut j = 0;
        while j + 1 < toks.len() {
            if toks[j + 1] == "ॱॱ" {
                let mut k = j + 2;
                let mut ty = Vec::new();
                while k < toks.len() && !(k + 1 < toks.len() && toks[k + 1] == "ॱॱ") {
                    ty.push(toks[k]);
                    k += 1;
                }
                if is_run_type(&ty) {
                    params.push(toks[j].to_string());
                }
                j = k;
            } else {
                j += 1;
            }
        }
        let mut end = i + 1;
        while end < lines.len() && !lines[end].starts_with("इति") {
            end += 1;
        }
        let body = &lines[i + 1..end];
        params.retain(|p| {
            let check = format!("यदि {p} ॱ दैर्घ्य न्यूनम् ");
            !body.iter().any(|l| l.trim_start().starts_with(&check))
        });
        let mut k = i + 1;
        while k < lines.len() && !lines[k].starts_with("इति") {
            let s = lines[k].trim_start();
            for p in &params {
                let prefix = format!("{p} अङ्कः ");
                if s.starts_with(&prefix)
                    && s.contains(" अन्तः भवति ")
                    && !s.contains(" अन्तः ॱ ")
                    && !is_run_type(&ret)
                {
                    found.push((routine.clone(), p.clone(), k + 1));
                }
            }
            k += 1;
        }
        i = k + 1;
    }
    found
}

#[test]
fn a_routine_that_stores_into_a_run_parameter_returns_the_run() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bad = Vec::new();
    let mut scanned = 0;
    for e in std::fs::read_dir(&dir).expect("the corpus is readable") {
        let p = e.expect("an entry").path();
        if p.extension().is_some_and(|x| x == "t1") {
            scanned += 1;
            let file = p.file_name().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&p).expect("a source");
            for (r, param, line) in offending(&src) {
                bad.push(format!(
                    "{file}:{line}: `{r}` stores into its parameter `{param}`"
                ));
            }
        }
    }
    assert!(
        scanned >= 20,
        "scanned only {scanned} sources — the scan has gone blind"
    );
    assert!(
        bad.is_empty(),
        "natively, growth writes the new block into the PARAMETER and the caller \
         keeps the old one. Return the run and have the caller assign it:\n{}",
        bad.join("\n")
    );
}

/// The scan must see the forbidden shape, and must pass the corpus convention.
#[test]
fn the_scan_sees_the_shape_it_forbids() {
    let src = "मण्डलम् क ॥\n\
               सार्वजनिक वृत्तिः गणय आदाय र ॱॱ अङ्कः अन्तः न६४ क ॱॱ न६४ ददाति न६४ आदि\n\
               \x20   र अङ्कः क अन्तः भवति ५ ।\n\
               \x20   प्रत्यागमनम् क योगः १ ।\n\
               इति\n\
               सार्वजनिक वृत्तिः योजय आदाय र ॱॱ अङ्कः अन्तः न६४ ऽ क ॱॱ न६४ ददाति अङ्कः अन्तः न६४ आदि\n\
               \x20   र अङ्कः क अन्तः भवति ५ ।\n\
               \x20   प्रत्यागमनम् र ।\n\
               इति\n\
               सार्वजनिक वृत्तिः भरय आदाय र ॱॱ अङ्कः अन्तः न६४ ददाति न६४ आदि\n\
               \x20   यदि र ॱ दैर्घ्य न्यूनम् ४ आदि\n\
               \x20       प्रत्यागमनम् ० ।\n\
               \x20   इति\n\
               \x20   र अङ्कः ३ अन्तः भवति ५ ।\n\
               \x20   प्रत्यागमनम् ४ ।\n\
               इति\n\
               सार्वजनिक वृत्तिः क्षेत्रम् आदाय र ॱॱ अङ्कः अन्तः अभि ददाति न६४ आदि\n\
               \x20   र अङ्कः १ अन्तः ॱ भेद भवति ५ ।\n\
               \x20   प्रत्यागमनम् ० ।\n\
               इति\n";
    let f = offending(src);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(
        f[0].0, "गणय",
        "the count-returning appender is the one flagged"
    );
    // Not flagged: a routine that returns the run, one that refuses when the
    // run is short (never grows it), and a FIELD write through an element.
}
