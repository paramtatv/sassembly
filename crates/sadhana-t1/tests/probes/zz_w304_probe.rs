//! SCRATCH PROBE for W-304 Q-a / Q-b. Not committed.
use sadhana::lex::lex_t1;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

const NAMES: &[&str] = &[
    "lex.t1",
    "ast.t1",
    "parse.t1",
    "sanskrit_text.t1",
    "sanchaya.t1",
    "artha.t1",
    "vastu.t1",
    "ir.t1",
    "utsarjana.t1",
    "yantrotsarjana.t1",
    "vishlesana.t1",
    "ashtaka.t1",
    "encode.t1",
    "vakyavibhaga.t1",
    "shrinkhala.t1",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_dir(dir: &Path, whole: bool) -> Interpreter {
    let mut texts: Vec<(String, String)> = Vec::new();
    if whole {
        let mut ps: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("t1"))
            .collect();
        ps.sort();
        for p in ps {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            texts.push((n, std::fs::read_to_string(&p).unwrap()));
        }
    } else {
        for n in NAMES {
            texts.push((n.to_string(), std::fs::read_to_string(dir.join(n)).unwrap()));
        }
    }
    let refs: Vec<(&str, &str)> = texts.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    Interpreter::load(&refs, &repo().join("spec")).unwrap_or_else(|e| panic!("load {}: {e:?}", dir.display()))
}

fn oct(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}
fn gi(it: &Interpreter, n: &str) -> String {
    format!("{:?}", it.global(n))
}

fn module(line: &str) -> String {
    format!(
        "मण्डलम् परीक्षणम् ॥\n\nसार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n{line}\n    प्रत्यागमनम् २१ ।\nइति\n"
    )
}

const PROBES: &[(&str, &str)] = &[
    ("P0 clean", "    चरः प ॱॱ न६४ भवति ० ।"),
    ("A1 ascii margin own line", "    ॰ ASCII margin (x+y) = 3;"),
    ("A2 ascii trailing margin", "    चरः प ॱॱ न६४ भवति ० ।   ॰ trailing ASCII x.y"),
    ("A3 margin glued to word", "    चरः प ॱॱ न६४ भवति ० ।॰x"),
    ("A4 literal holding ॰ then margin", "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् क ॰ इति । ॰ ASCII after"),
    ("A5 margin mentioning उक्तम्", "    चरः प ॱॱ न६४ भवति ० ।  ॰ see उक्तम् SAS here"),
    ("B1 SAS in literal", "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् SAS इति ।"),
    ("B2 ॰ then SAS in literal", "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् ॰ SAS इति ।"),
    ("B3 unclosed literal with SAS", "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् SAS"),
    ("B4 glued in literal", "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् कSक इति ।"),
    ("B5 इति-pair then SAS in literal", "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् इति इति SAS इति ।"),
    ("B6 unclosed literal clean", "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् क"),
];

#[test]
fn probe() {
    let which = std::env::var("PROBE").unwrap_or_default();
    let main_dir = repo().join(".scratch-w304/main-src/crates/sadhana-t1/src");
    let tick_dir = repo().join(".scratch-w304/tick-src/crates/sadhana-t1/src");
    let mine_dir = repo().join("crates/sadhana-t1/src");
    for (label, line) in PROBES {
        let src = module(line);
        let rust = match lex_t1(&src) {
            Ok(_) => "lexes".to_string(),
            Err(es) => es.iter().map(|e| format!("L{} `{}` {}", e.line, e.aksara, e.reason)).collect::<Vec<_>>().join(" | "),
        };
        println!("=== {label}\n  RUST lex_t1: {rust}");
        if which.contains("scan") {
            let mut m = load_dir(&main_dir, true);
            let r = m.call("अक्षरकोशॱपरिधिदोषः", vec![oct(&src)], 600_000_000).map(|v| format!("{v:?}"));
            println!("  MAIN परिधिदोषः: {r:?} (len {})", src.len());
            let mut t = load_dir(&tick_dir, true);
            let r = t.call("अक्षरकोशॱसङ्ग्रहदोषः", vec![oct(&src)], 600_000_000).map(|v| format!("{v:?}"));
            println!("  TICK सङ्ग्रहदोषः: {r:?} cause {}", gi(&t, "सङ्ग्रहदोषकारणम्"));
            let mut y = load_dir(&mine_dir, true);
            let r = y.call("अक्षरकोशॱपरिधिदोषः", vec![oct(&src)], 600_000_000).map(|v| format!("{v:?}"));
            println!("  MINE परिधिदोषः: {r:?} cause {}", gi(&y, "परिधिदोषकारणम्"));
        }
        for (tag, dir) in [("MAIN", &main_dir), ("TICK", &tick_dir), ("MINE", &mine_dir)] {
            if !which.contains(&tag.to_lowercase()) {
                continue;
            }
            // Front's path: पदविभाग then व्याकरॱकार्यक्रमपठनम्
            let mut it = load_dir(dir, false);
            let entry = if tag == "MINE" { "अक्षरकोशॱपरिधिपदविभाग" } else { "पदविभागॱपदविभाग" };
            let tok = it.call(entry, vec![oct(&src)], 4_000_000_000).map(|v| format!("{v:?}"));
            let tokn = tok.as_ref().ok().and_then(|s| s.trim_start_matches("Int(").trim_end_matches(')').parse::<i128>().ok()).unwrap_or(0);
            let dec = it.call("व्याकरॱकार्यक्रमपठनम्", vec![Value::Int(tokn)], 40_000_000_000).map(|v| format!("{v:?}"));
            println!(
                "  {tag} Front: tokens {tok:?} decls {dec:?} errs {} | परिधिदोषस्थितम् {} स्थानम् {} सङ्ग्रहविरामः {}",
                gi(&it, "दोषसूचकाङ्क"),
                gi(&it, "परिधिदोषस्थितम्"),
                gi(&it, "परिधिदोषस्थानम्"),
                gi(&it, "सङ्ग्रहविरामः")
            );
            let mut it = load_dir(dir, false);
            let p = it.call("शृङ्खलाॱपठनम्", vec![oct(&src)], 40_000_000_000).map(|v| format!("{v:?}"));
            println!(
                "  {tag} पठनम्: {p:?} errs {} | परिधिदोषस्थितम् {} स्थानम् {} सङ्ग्रहविरामस्थानम् {}",
                gi(&it, "दोषसूचकाङ्क"),
                gi(&it, "परिधिदोषस्थितम्"),
                gi(&it, "परिधिदोषस्थानम्"),
                gi(&it, "सङ्ग्रहविरामस्थानम्")
            );
        }
    }
}
