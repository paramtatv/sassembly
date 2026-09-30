//! The SLP1 fallback, from the command line — task `B-092`, doc 03 §3.3.4.
//!
//! `nidana`'s unit tests prove the transliteration. They cannot prove the
//! **boundary is wired**, and that is where every half-migration in this project
//! has hidden: `B-078` counted seventeen coded diagnostics in one file while ten
//! in another were still English, and every test it had passed.
//!
//! So this runs the real binary, on a real broken program, with a locale that
//! cannot carry Devanagari, and asserts the bytes that reach the terminal.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A source that fails to parse, written to a scratch file.
fn broken() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        // UNIQUE PER PROCESS AND PER RUN. A fixed name is SHARED: 43 worktrees
        // and several agents run gates on this machine at once, and two runs in
        // one directory corrupt each other. The clock is the load-bearing part --
        // pids are reused and these directories are never removed (W-301).
        "sansos-b092-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let path = dir.join("टूटा.sas");
    // `कम्` carries a kāraka and no verb precedes it — `P09`, ADR-0004.
    std::fs::write(&path, "कम् खन गन ।\n").expect("write");
    path
}

/// Run the assembler and return what it wrote to the terminal.
fn run(script: Option<&str>, ctype: &str, source: &Path) -> String {
    let mut c = Command::new(env!("CARGO_BIN_EXE_sadhana"));
    c.arg(source).arg(source.with_extension("elf"));
    // Cleared rather than overridden: on this machine `LC_ALL` is unset and
    // `LANG` is UTF-8, so setting only one of the three would leave the answer
    // depending on which the binary happened to read first.
    c.env_remove("LC_ALL")
        .env_remove("LC_CTYPE")
        .env_remove("LANG");
    c.env("LC_ALL", ctype);
    match script {
        Some(s) => c.env("SANSOS_SCRIPT", s),
        None => c.env_remove("SANSOS_SCRIPT"),
    };
    let out = c.output().expect("run sadhana");
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn has_devanagari(s: &str) -> bool {
    s.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c))
}

#[test]
fn a_terminal_that_cannot_render_devanagari_gets_slp1() {
    let src = broken();

    // The default terminal here can render it, and does.
    let utf8 = run(None, "en_US.UTF-8", &src);
    assert!(
        has_devanagari(&utf8),
        "the default is not Devanagari: {utf8}"
    );

    // A C locale cannot carry it at all, so the same fault comes back in ASCII.
    let ascii = run(None, "C", &src);
    assert!(
        !has_devanagari(&ascii),
        "boxes on a terminal that cannot render them: {ascii}"
    );
    assert!(!ascii.trim().is_empty(), "nothing was reported at all");

    // The same diagnostic, not a different or emptier one. `vAkyaM` is the
    // Sanskrit sentence transliterated; `ADR-0004` is the rule reference, which
    // was already legible and must survive untouched.
    assert!(ascii.contains("ADR-0004"), "{ascii}");
    assert!(ascii.contains("kam"), "the word at fault: {ascii}");

    // And an explicit ask beats a locale that would have said otherwise.
    let asked = run(Some("slp1"), "en_US.UTF-8", &src);
    assert!(!has_devanagari(&asked), "{asked}");
    let refused = run(Some("deva"), "C", &src);
    assert!(has_devanagari(&refused), "{refused}");
}

#[test]
fn the_fallback_reaches_the_usage_message_too() {
    // A reader who cannot render Devanagari cannot render it in a message about
    // arguments either, and `--संक्षिप्त` is spelled in it. This is the site
    // most likely to be missed, because it runs before anything is parsed.
    let out = Command::new(env!("CARGO_BIN_EXE_sadhana"))
        .env_remove("LC_CTYPE")
        .env_remove("LANG")
        .env("LC_ALL", "C")
        .output()
        .expect("run sadhana");
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("usage:"), "{text}");
    assert!(
        !has_devanagari(&text),
        "the usage message is unreadable: {text}"
    );
}

#[test]
fn every_line_the_binary_writes_goes_through_the_fallback() {
    // Fifteen print sites, one of which was the usage message and one of which
    // was reached only by a write error. A fallback applied to fourteen of them
    // is not a fallback — and the fourteen that work are exactly why nobody
    // notices. So the source is asked directly: `say!` is the only way out.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"),
    )
    .expect("read main.rs");
    let writes: Vec<&str> = src
        .lines()
        .filter(|l| l.contains("println!("))
        .map(str::trim)
        .collect();
    assert_eq!(
        writes.len(),
        1,
        "exactly one line in main.rs writes to a terminal, and it is the one \
         inside `say!`; these do not: {writes:?}"
    );
    assert!(
        writes[0].starts_with("eprintln!(\"{}\", transliterate("),
        "the one write does not transliterate: {}",
        writes[0]
    );
    assert!(src.matches("say!(").count() > 10, "say! is barely used");
}
