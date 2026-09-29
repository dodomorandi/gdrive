#![expect(
    clippy::must_use_candidate,
    clippy::new_without_default,
    clippy::return_self_not_must_use,
    reason = "generated request types intentionally expose constructors and builders directly"
)]
#![expect(
    clippy::doc_markdown,
    reason = "Discovery descriptions are copied verbatim into generated documentation"
)]
#![expect(
    rustdoc::bare_urls,
    reason = "Discovery descriptions may contain plain URLs"
)]

include!(concat!(env!("OUT_DIR"), "/drive_v3.rs"));
