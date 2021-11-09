// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

#[cfg(test)]
mod test {
    use std::ffi::OsStr;
    use std::fs::File;

    use walkdir::WalkDir;

    use crate::data::BulletML;

    #[test]
    fn test_parse_examples() {
        let ext = OsStr::new("xml");

        WalkDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data"))
            .sort_by(|e1, e2| e1.path().cmp(e2.path()))
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension() == Some(ext))
            .for_each(|entry| {
                println!("reading {}", entry.path().display());
                let fin = File::open(entry.path()).unwrap();
                let _: BulletML = serde_xml_rs::from_reader(fin).unwrap();
            });
    }
}
