#![feature(ptr_as_ref_unchecked, const_ptr_as_ref)]
#![doc = include_str!("../README.md")]

pub mod cell;
pub mod rc;
mod vec;

pub use vec::Vec;
