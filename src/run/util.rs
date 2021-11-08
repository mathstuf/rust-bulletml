// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use std::collections::hash_map::{Entry, HashMap};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EntityError {
    #[error("duplicate {} entity `{}`", kind, name)]
    Duplicate { name: String, kind: &'static str },
}

impl EntityError {
    fn duplicate<N>(kind: &'static str, name: N) -> Self
    where
        N: Into<String>,
    {
        Self::Duplicate {
            kind,
            name: name.into(),
        }
    }
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
        return Err(EntityError::duplicate(kind, o.key()));
    }

    entry.or_insert_with(f);

    Ok(())
}
