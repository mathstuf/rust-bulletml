// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying file LICENSE for details.

#[cfg(test)]
mod test {
    extern crate itertools;
    use self::itertools::Itertools;

    extern crate serde_xml_rs;

    extern crate walkdir;
    use self::walkdir::WalkDir;

    use std::ffi::{OsStr, OsString};
    use std::fs::File;

    use data::BulletML;

    #[test]
    fn test_parse_examples() {
        WalkDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data"))
            .sort_by(OsString::cmp)
            .into_iter()
            .filter_map(|entry| {
                entry.ok()
            })
            .filter(|entry| {
                entry.path().extension() == Some(OsStr::new("xml"))
            })
            .foreach(|entry| {
                let fin = File::open(entry.path()).unwrap();
                let _: BulletML = serde_xml_rs::deserialize(fin).unwrap();
            });
        panic!()
    }
}
