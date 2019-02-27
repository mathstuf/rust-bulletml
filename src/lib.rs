// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying file LICENSE for details.

//! BulletML
//!
//! A BulletML parser and interpreter.

#![warn(missing_docs)]

#[macro_use]
extern crate failure;

mod crates {
    pub extern crate failure;
}

mod data;
