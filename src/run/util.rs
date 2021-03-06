// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use std::collections::hash_map::{Entry, HashMap};

#[derive(Debug, Fail)]
pub enum EntityError {
    #[fail(display = "duplicate {} entity `{}`", _1, _0)]
    Duplicate(String, &'static str),
}

pub fn try_insert<N, V, F>(
    name: N,
    map: &mut HashMap<String, V>,
    f: F,
    kind: &'static str,
) -> Result<(), EntityError>
where
    N: Into<String>,
    F: FnOnce() -> V,
{
    let entry = map.entry(name.into());
    if let Entry::Occupied(ref o) = entry {
        Err(EntityError::Duplicate(o.key().clone(), kind))?;
    }

    entry.or_insert_with(f);

    Ok(())
}
