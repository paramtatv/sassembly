//! **`W-374`, THE INTERPRETER'S HALF: A COUNTER READ IS REFUSED BY NAME, NEVER ANSWERED.**
//!
//! The retired-instruction counter (`yantra::COUNTER`, 0x1000_0108) is reached from `.t1`
//! through `W-350`'s device-read seam, `अष्टकॱउपकरणचतुरष्टकाहारः`. Natively that is an `lw`
//! that `yantra` answers with the count. The interpreter (`nirvahana.rs`) executes no
//! RV64 instructions, so it HAS no retired count: any number it gave would be invented,
//! and a plausible one would let a predict agree with a native run that read something
//! else. Its existing `W-350` refusal of every device read is therefore the right answer
//! for the counter too, and this file pins it AT THE COUNTER'S ADDRESS.
//!
//! It is also the interpreter's share of `W-372`: Stage 1 is built by the interpreted
//! compiler, so a counter read on any path it executes stops Stage 1 by name.
//!
//! RED CONTROL, recorded in the commit: with the refusal removed from `nirvahana.rs`, the
//! call reaches `ashtaka.t1`'s body and answers its sentinel 4563406848, and this reds.

use sadhana::t1::nirvahana::Interpreter;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn source(name: &str) -> String {
    let p = repo_root().join("crates/sadhana-t1/src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()))
}

/// Reads the counter's low word through the seam. २६८४३५७२० = 0x1000_0108.
const CALLER: &str = "\
मण्डलम् मुद्रणपरीक्षा ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः लेखनम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति अष्टकॱउपकरणचतुरष्टकाहारः २६८४३५७२० ।
    प्रत्यागमनम् क ।
इति
";

#[test]
fn the_interpreter_refuses_a_counter_read_by_name() {
    assert_eq!(0x1000_0108u64, 268_435_720, "the address CALLER spells");
    let ashtaka = source("ashtaka.t1");
    let mut it = Interpreter::load(
        &[("ashtaka.t1", ashtaka.as_str()), ("caller.t1", CALLER)],
        &repo_root().join("spec"),
    )
    .expect("अष्टक and the caller load together");
    let got = it.call("मुद्रणपरीक्षाॱलेखनम्", vec![], 5_000_000);
    let err = match got {
        Ok(v) => panic!(
            "a counter read must not be answered by the interpreter — it has no retired \
             count; got {v:?}"
        ),
        Err(e) => e.reason,
    };
    assert!(
        err.contains("उपकरणचतुरष्टकाहारः") && err.contains("DEVICE REGISTER"),
        "the refusal names the intrinsic and why: {err}"
    );
}
