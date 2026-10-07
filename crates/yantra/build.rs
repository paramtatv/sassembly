//! The build commit stamp for this crate's binaries (`W-347`).
//!
//! ONE statement of it, in `tools/build-stamp.rs`, included here rather than copied
//! into each crate so the two cannot drift. That file carries the reasoning, and its
//! first property is that it CANNOT FAIL A BUILD: no git, no repository, a broken
//! checkout — every path stamps `unknown` and exits 0.
//!
//! The doc comment lives here rather than in the included file because a build
//! script is its own crate and `missing_docs` is asked of the crate root, which an
//! `include!` in the middle of the file cannot provide.

include!("../../tools/build-stamp.rs");
