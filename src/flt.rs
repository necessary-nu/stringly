use std::collections::BTreeMap;

use crate::{InputData, PathNode, ParseError};

pub fn generate(input: InputData) -> Result<BTreeMap<String, PathNode>, ParseError> {
    let mut files = BTreeMap::new();
    for (k, v) in input.into_inner().into_iter() {
        let mut subfiles = BTreeMap::new();
        for m in v {
            let lang = m.language.clone();
            let x: fluent_syntax::ast::Resource<String> = m.try_into()?;
            subfiles.insert(
                format!("{lang}.flt"),
                PathNode::File(fluent_syntax::serializer::serialize(&x).into_bytes()),
            );
        }
        files.insert(k, PathNode::Directory(subfiles));
    }
    Ok(files)
}
