// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

//! BulletML
//!
//! A BulletML parser and interpreter.

#![warn(missing_docs)]

#[macro_use]
extern crate failure;

mod crates {
    pub extern crate failure;
}

pub mod data;
mod run;
