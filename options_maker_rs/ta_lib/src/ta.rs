// Disable for the entire crate
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
// For Clippy specifically
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::module_name_repetitions)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
