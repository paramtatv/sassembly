//! **A PROGRAM READS ITS OWN COMMAND-LINE ARGUMENTS.**
//!
//! The third of the three capabilities that separate a self-hosting compiler
//! from a toolchain someone else can use — and by far the cheapest, for a
//! reason worth stating rather than enjoying.
//!
//! **A FILE NAME MUST BE ASKED FOR WHILE RUNNING.** That is why it cost an MMIO
//! window, two intrinsics and two fixpoint rounds (ADR-0041): the conversation
//! happens after `pc` has moved. **ARGUMENTS ARE KNOWN BEFORE IT MOVES**, so
//! they fit the channel that already existed — a tag word, a global immediately
//! after it, and octets written into RAM before the program starts.
//!
//! So this needed **no compiler change, no `.t1` corpus change, and therefore
//! no fixpoint round**. The program declares the two globals itself, exactly as
//! `t1_user_input_interface.rs` showed a program can.
//!
//! # What is asserted
//!
//! The SUM of the octets of the second argument, and the COUNT separately.
//! Asserting the count alone would pass on a channel that delivered the right
//! number of empty arguments; asserting one argument's octets alone would pass
//! on a channel that lost the separator and handed over one long run.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

const FUEL: u64 = 80_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// Declares the two argument globals with their tag words immediately before
/// them — the layout `input.rs` scans for — then walks the joined run to the
/// first separator and sums the octets that follow, which is argument १.
///
/// **THE FIRST VERSION OF THIS PROGRAM DID NOT COMPILE**, and the failure was
/// mine: a walk written with three nested conditions tracking a separator
/// count, where one nested loop starting after the separator says the same
/// thing. The compiler refused it; the simpler program is also the readable
/// one, which is usually how that goes.
///
/// The tag literals are the constants from `yantra::input` written in
/// Devanagari. `SASARGV\0` = 0x0056_4752_4153_4153 = २४२८५२६६६०५९८२०३५,
/// `SASARGC\0` = 0x0043_4752_4153_4153 = १८९३७२४२०४८४७९५७१.
const READS_ARGS: &str = "मण्डलम् आदेशकः ॥

सार्वजनिक चरः आदेशसङ्केतः ॱॱ न६४ भवति २४२८५२६६६०५९८२०३५ ।
सार्वजनिक चरः आदेशपङ्क्तिः ॱॱ अङ्कः अन्तः अ८ भवति ० ।
सार्वजनिक चरः गणनासङ्केतः ॱॱ न६४ भवति १८९३७२४२०४८४७९५७१ ।
सार्वजनिक चरः आदेशगणना ॱॱ न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    यदि आदेशगणना न्यूनम् २ आदि
        प्रत्यागमनम् ९००० योगः आदेशगणना ।
    इति
    चरः स्थानम् ॱॱ न६४ भवति ० ।
    यावत् स्थानम् न्यूनम् आदेशपङ्क्तिः ॱ दैर्घ्य आदि
        यदि आदेशपङ्क्तिः अङ्कः स्थानम् अन्तः समम् ० आदि
            स्थानम् भवति स्थानम् योगः १ ।
            चरः योगफलम् ॱॱ न६४ भवति ० ।
            यावत् स्थानम् न्यूनम् आदेशपङ्क्तिः ॱ दैर्घ्य आदि
                यदि आदेशपङ्क्तिः अङ्कः स्थानम् अन्तः समम् ० आदि
                    प्रत्यागमनम् योगफलम् ।
                इति
                योगफलम् भवति योगफलम् योगः आदेशपङ्क्तिः अङ्कः स्थानम् अन्तः ।
                स्थानम् भवति स्थानम् योगः १ ।
            इति
            प्रत्यागमनम् योगफलम् ।
        इति
        स्थानम् भवति स्थानम् योगः १ ।
    इति
    प्रत्यागमनम् ८८८८ ।
इति
";

fn build(src: &str, module: &str) -> Vec<u8> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(module.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    it.call(
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![
            arena(vec![octets(src.as_bytes())]),
            arena(vec![octets(module.as_bytes())]),
            Value::Int(1),
        ],
        FUEL,
    )
    .expect("मण्डलानिप्रतिबिम्बम् runs")
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default()
}

#[test]
fn a_program_reads_the_arguments_it_was_given() {
    let image = build(READS_ARGS, "आदेशकः");
    assert!(!image.is_empty(), "the program built no image");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");

    let args: [&[u8]; 3] = [b"prog", b"abc", b"zz"];
    let placed = yantra::input::inject_arguments(&mut m.mem, m.base, &args)
        .expect("the image declares the argument globals");
    assert_eq!(placed.argc, 3, "three arguments were handed over");

    let mut out: Vec<u8> = Vec::new();
    let status = match m.run(400_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the program did not finish: {other:?}"),
    };

    // 'a'+'b'+'c' = 97+98+99. The SECOND argument, so the separator was found
    // and skipped: a channel that lost it would sum "prog\0abc..." and answer
    // something much larger.
    assert_eq!(
        status, 294,
        "the program summed argument १ as {status}; `abc` is 294. 9000+n means \
         it saw fewer than two arguments (n of them); 8888 means it walked off \
         the end without finding the separator"
    );
}

#[test]
fn an_image_without_the_globals_is_refused_rather_than_silently_empty() {
    // The negative control. A program that never declared the interface must
    // not be handed arguments into whatever happens to be at that address.
    const NO_GLOBALS: &str = "मण्डलम् रिक्तः ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    प्रत्यागमनम् ० ।
इति
";
    let image = build(NO_GLOBALS, "रिक्तः");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let args: [&[u8]; 1] = [b"prog"];
    let refusal = yantra::input::inject_arguments(&mut m.mem, m.base, &args);
    let msg = refusal.expect_err("an image without the globals must be refused");
    assert!(
        msg.contains("SASARGV"),
        "the refusal must name the tag it looked for, and said: {msg}"
    );
}

/// **THE TWO STATEMENTS OF EACH TAG ARE MADE TO AGREE, HERE.**
///
/// A tag exists twice: as a Rust constant the host scans for, and as a
/// Devanagari literal the program declares. Nothing connects them but a human
/// writing the same number twice — and on the first write of this file, BOTH
/// were wrong. The scan refused, naming the tag it wanted, and that is the only
/// reason it took minutes rather than an afternoon.
///
/// `t1_input_channel.rs` makes exactly this check for the three `निवेश…` tags,
/// by reading the literals out of `shrinkhala.t1`. The argument literals live in
/// this file's own fixture, so the check lives here: the constant is rendered to
/// Devanagari and the fixture must contain it.
///
/// This is a guard against a defect that has already happened once, not a
/// hypothetical.
#[test]
fn the_tag_literals_in_the_fixture_are_the_constants_the_host_scans_for() {
    fn devanagari(mut n: u64) -> String {
        if n == 0 {
            return "०".into();
        }
        let d = ['०', '१', '२', '३', '४', '५', '६', '७', '८', '९'];
        let mut out = Vec::new();
        while n > 0 {
            out.push(d[(n % 10) as usize]);
            n /= 10;
        }
        out.reverse();
        out.into_iter().collect()
    }

    for (tag, what) in [
        (yantra::input::ARGV_TAG, "SASARGV"),
        (yantra::input::ARGC_TAG, "SASARGC"),
    ] {
        let want = devanagari(tag);
        assert!(
            READS_ARGS.contains(&want),
            "the fixture does not declare {what} as {want} ({tag:#x}). The host \
             scans for the constant and the program declares the literal; if \
             they differ the scan refuses and the program is handed nothing"
        );
    }

    // AND THE TAGS ARE BELOW 2^63, which is not decoration: a word above that is
    // NEGATIVE to a signed lowering, and the `.t1` literal goes through one.
    for (tag, what) in [
        (yantra::input::ARGV_TAG, "SASARGV"),
        (yantra::input::ARGC_TAG, "SASARGC"),
    ] {
        assert!(
            tag < 1 << 63,
            "{what} is {tag:#x}, at or above 2^63 — a `.t1` literal that large \
             is negative to a signed lowering"
        );
    }
}

/// **AND THROUGH THE BINARY, WHICH IS THE ONLY PATH A USER HAS.**
///
/// The three tests above call `inject_arguments` directly. That proves the
/// channel and NOT the thing a person does, which is type a command. A manual
/// run is not a guard: it passes once, on the machine of whoever ran it, and
/// leaves no trace when someone later adds a flag to `yantra-run` that eats the
/// program's arguments before they reach it.
///
/// So this builds an image with `t1_image`, runs `yantra-run` with three
/// arguments after it, and reads the halt. Two things are asserted and they
/// fail differently: the BINARY reports four handed over (`argv[0]` is the
/// image, per execve), and the PROGRAM answers ५१८ — the octet sum of `alpha`,
/// which is argument १. Four empty arguments would satisfy the first and fail
/// the second.
///
/// Ignored by default because it shells out to two release binaries and builds
/// an ELF — and **an ignored test that nothing names is a test that never
/// runs**, so `tools/deep-gate.sh` names it explicitly with
/// `--include-ignored`, beside `t1_selfhost_strict`, where those binaries have
/// already been built. This project has paid for the other arrangement: the
/// governing goal passed for cycles behind a stale `#[ignore]` nobody executed.
#[test]
#[ignore = "MEASUREMENT, ~4s — and it PASSES. Ignored because it builds an ELF \
 with t1_image and shells out to yantra-run, which a per-commit gate has no \
 release binaries for. `tools/deep-gate.sh` names it with --include-ignored, \
 beside t1_selfhost_strict, where those binaries already exist"]
fn the_binary_hands_a_program_its_command_line() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bin = root.join("target/release");
    let (t1_image, yantra_run) = (bin.join("t1_image"), bin.join("yantra-run"));
    if !t1_image.exists() || !yantra_run.exists() {
        // A HONEST SKIP, and it says what is missing rather than passing
        // silently: the release binaries are what this test is about.
        eprintln!("skipped: build with `cargo build --release -p sadhana -p yantra` first");
        return;
    }

    let dir = std::env::temp_dir().join(format!(
        // pid AND a clock: a pid is REUSED and these roots are never
        // removed, so a pid-only name is unique inside this process and
        // not on disk (W-301).
        "argv-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let src = dir.join("आदेशकः.t1");
    std::fs::write(&src, READS_ARGS).expect("fixture written");
    let elf = dir.join("argprog.elf");

    let built = std::process::Command::new(&t1_image)
        .current_dir(&root)
        .arg("--compiler")
        .arg("crates/sadhana-t1/src")
        .arg("--entry")
        .arg("आदेशकः")
        .arg("मुख्यम्")
        // `W-381`: the entry is positional only, so there is no predict and the
        // differential gate has nothing to compare. The program's subject is
        // the command line yantra-run hands it below, which neither the predict
        // nor the gate's own run carries.
        .arg("--accept-divergence")
        .arg("the program answers its command line, which only the yantra-run below hands it")
        .arg("-o")
        .arg(&elf)
        .arg(&src)
        .output()
        .expect("t1_image runs");
    assert!(
        elf.exists(),
        "t1_image built no image: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let run = std::process::Command::new(&yantra_run)
        .current_dir(&root)
        .arg(&elf)
        .args(["alpha", "beta", "gamma"])
        .output()
        .expect("yantra-run runs");
    let err = String::from_utf8_lossy(&run.stderr);

    assert!(
        err.contains("args: 4 handed over"),
        "the binary did not hand over four arguments. If a FLAG was added to \
         yantra-run, it may be eating the program's arguments before they reach \
         it — everything after the image belongs to the program. stderr:\n{err}"
    );
    // `alpha` is argument १ — argv[0] being the image, per execve. Its octets
    // sum to ५१८, and asserting the SUM rather than the count is what proves
    // the octets themselves crossed: a channel handing over four EMPTY
    // arguments satisfies the line above and fails here.
    assert!(
        err.contains("status: Some(518)"),
        "the binary handed over four arguments and the program did not sum \
         `alpha` (518) out of them — so the count crossed and the octets did \
         not, or the separator was lost. stderr:\n{err}"
    );

    std::fs::remove_dir_all(&dir).ok();
}
