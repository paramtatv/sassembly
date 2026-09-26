//! Part of SANSOS.
use sadhana::lex::Kind;
/// The the crate.
/// The the crate.
/// The the crate.
/// The the crate.
/// The the crate.
/// The the crate.
use sadhana::lex::lex_t1 as lex;
use sadhana::t1::anita;
use sadhana::t1::parse::Parser;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    // `--spec-root` IS A FLAG AND NOT A GUESS — ADR-0024.
    //
    // The embed pass needs a root to resolve names against, and the three
    // alternatives do not survive self-hosting: `spec/` does not ship
    // (`release.yml:231` — "three binaries, under their own names"), T1 has no
    // I/O with which to set an environment variable (ADR-0019:35), and walking
    // up from the source encodes this checkout's layout into the compiler.
    // A flag puts the root where `--लक्ष्य` already lives: in the invocation,
    // stated by whoever drives the build.
    //
    // OMISSION IS AN ERROR, NOT A DEFAULT. ADR-0024 left the default open and
    // said a silent guess would undo the decision, so a `.t1` file that uses
    // the embed without a root is REFUSED rather than resolved against
    // somewhere plausible.
    let mut spec_root: Option<PathBuf> = None;
    let mut positional: Vec<&String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--spec-root" {
            match args.get(i + 1) {
                Some(v) => {
                    spec_root = Some(PathBuf::from(v));
                    i += 2;
                }
                None => {
                    eprintln!("--spec-root needs a directory");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            positional.push(&args[i]);
            i += 1;
        }
    }
    if positional.is_empty() {
        eprintln!("Usage: t1_parse [--spec-root <dir>] <file.t1>");
        return ExitCode::FAILURE;
    }

    let filepath = positional[0];
    let source = fs::read_to_string(filepath).expect("Failed to read file");

    let tokens = match lex(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lex error: {:?}", e);
            return ExitCode::FAILURE;
        }
    };

    // THE SEAM. A pass that rewrites a token stream runs after lexing and
    // before parsing; `anita::resolve()` has been complete and unreachable
    // since ADR-0019 because nothing called it (`D-002i`).
    let tokens = match &spec_root {
        Some(root) => match anita::resolve(tokens, root) {
            Ok(t) => t,
            Err(errors) => {
                for e in &errors {
                    eprintln!("Embed error: {e:?}");
                }
                return ExitCode::FAILURE;
            }
        },
        // No root given. A source that does NOT use the embed is fine and
        // passes straight through; a source that DOES must fail here, loudly,
        // rather than reach the parser as a bare word and surface three stages
        // later as a mysterious syntax error. The first draft of this arm was a
        // plain passthrough and a probe using the embed compiled "successfully"
        // with no root at all — the exact silent-success shape this tree keeps
        // finding.
        None => {
            if tokens
                .iter()
                .any(|tok| !matches!(tok.kind, Kind::Str { .. }) && tok.text == anita::EMBED_WORD)
            {
                eprintln!(
                    "{} used but no --spec-root given: the embed cannot be resolved (ADR-0024)",
                    anita::EMBED_WORD
                );
                return ExitCode::FAILURE;
            }
            tokens
        }
    };

    let mut parser = Parser::new(&tokens);
    match parser.parse_program() {
        Ok(program) => {
            println!(
                "Successfully parsed {}. {} declarations.",
                filepath,
                program.declarations.len()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Error parsing {}: {:?}", filepath, err);
            ExitCode::FAILURE
        }
    }
}
