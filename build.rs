// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

extern crate peg;

fn main() {
    peg::cargo_build("src/data/expression/grammar.rustpeg");
}
