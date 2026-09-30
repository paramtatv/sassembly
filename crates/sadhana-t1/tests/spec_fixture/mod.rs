//! Building a substitute `spec/` root, in ONE place.
//!
//! # Why this module exists
//!
//! Two test binaries each grew their own copy of "hard-link every table from
//! `spec/`, then write the one being substituted": `spec_root_with` in
//! `t1_exec_aksara.rs` and `spec_root_with_compression` in `t1_exec_encode.rs`.
//! **Both carried the same defect and it was fixed in one of them first.** The
//! second was found only by grepping the workspace for `hard_link`.
//!
//! The defect, measured on 2026-09-26: the roots were named
//! `<prefix>-<pid>-<counter>`, which is unique *inside a process* and not on
//! disk, and they were never removed. `$TMPDIR` held **23,528**
//! `aksara-spec-*` directories from **425 distinct pids** and **616**
//! `encode-spec-*` from **616 pids**; `spec/extended-pictographic.tsv` carried
//! **2,074** hard links and `spec/compression-choices.tsv` **1,133**. A reused
//! pid lands a run on a directory that ALREADY holds a link to the table it is
//! about to substitute, and `fs::write` — open with `O_TRUNC` — then writes
//! through that link into the repository.
//!
//! It is not hypothetical. The same evening, the rail's own
//! `spec/shiva-sutras.tsv` gained the row `5\t9\t\tzz\tphoneme` — byte for byte
//! the third fixture of `probe_tables()` — and blocked the rail for six fires,
//! because a rail refuses to run on a dirty tree and only the rail would clean
//! it.
//!
//! So the two dangerous steps live here, once, with a guard built around the
//! bug's PRECONDITION rather than around its absence.

use std::path::{Path, PathBuf};

/// A temp directory name no other process can collide with.
///
/// The nanosecond stamp is the load-bearing part. `(pid, counter)` repeats
/// across processes because pids are reused and these roots are never removed;
/// the clock does not.
pub fn unique_root(prefix: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("a clock after 1970")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{stamp}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Write `content` to `dir/name`, **breaking any link the path already carries**.
///
/// This is the whole point of the module. `fs::write` alone is
/// `open(O_TRUNC)` + `write`, which edits every other name for the same inode —
/// so if `dir/name` arrived as a hard link to the repository's table, a plain
/// write edits the repository. Removing the directory entry first drops this
/// name's claim on that inode and leaves the other names untouched; the write
/// then creates a fresh file.
///
/// `NotFound` is the ordinary case, not an error: usually there is nothing to
/// remove.
pub fn write_substituted(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => panic!("could not clear {}: {e}", path.display()),
    }
    std::fs::write(&path, content).expect("write the substituted table");
}

/// **The guard, built around the PRECONDITION: a directory that ALREADY holds a
/// hard link to the file being substituted.**
///
/// A guard that merely calls the root builder and checks the result is a fresh
/// file proves nothing — a fresh directory never holds the link, so it passes
/// with the defect present, in every run where the collision did not happen to
/// occur. That is about 249 runs in 250, and it is the trap this project has
/// fallen into repeatedly: a check correct for the population it was built
/// against.
///
/// So the collision is CONSTRUCTED. Nothing under `spec/` is touched — the
/// "repository" here is a scratch file of this test's own.
#[test]
fn writing_a_substituted_table_never_edits_the_file_it_was_linked_from() {
    use std::os::unix::fs::MetadataExt;

    let dir = unique_root("spec-fixture-guard");
    std::fs::create_dir_all(&dir).expect("temp dir");

    // The stand-in for the repository's table. A scratch file, so a failure
    // here damages nothing that matters.
    let source = dir.join("source.tsv");
    std::fs::write(&source, "row\tone\nrow\ttwo\n").expect("write the source");
    let before = std::fs::read(&source).expect("read the source");
    let source_ino = std::fs::metadata(&source).expect("source").ino();

    // THE PRECONDITION: the path about to be written is a LINK to it, which is
    // exactly what a reused root handed the old code.
    let linked = dir.join("linked.tsv");
    std::fs::hard_link(&source, &linked).expect("hard link");
    assert_eq!(
        std::fs::metadata(&linked).expect("linked").ino(),
        source_ino,
        "the fixture failed to set up the collision — a link must share the inode"
    );
    assert_eq!(
        std::fs::metadata(&source).expect("source").nlink(),
        2,
        "the source should have two names now"
    );

    write_substituted(&dir, "linked.tsv", "");

    // 1. The substitution took.
    assert_eq!(
        std::fs::read(dir.join("linked.tsv")).expect("substituted"),
        Vec::<u8>::new(),
        "the substituted file should be empty"
    );
    // 2. AND THE SOURCE IS UNTOUCHED. This is the assertion the whole module is
    //    for; a plain `fs::write` truncates it to nothing here.
    assert_eq!(
        std::fs::read(&source).expect("read the source back"),
        before,
        "writing the substituted table EDITED THE FILE IT WAS LINKED FROM — the \
         unlink in `write_substituted` is gone or ineffective"
    );
    // 3. And the link was really broken, not merely survived.
    assert_ne!(
        std::fs::metadata(dir.join("linked.tsv"))
            .expect("substituted")
            .ino(),
        source_ino,
        "the substituted file still shares the source's inode"
    );

    std::fs::remove_dir_all(&dir).ok();
}
