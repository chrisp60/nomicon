#![feature(ptr_as_ref_unchecked, const_ptr_as_ref, const_mut_refs)]
#![doc = include_str!("../README.md")]

pub mod arc;
pub mod cell;
pub mod rc;
mod vec;

pub use vec::Vec;
