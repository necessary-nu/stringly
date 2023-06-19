use std::collections::BTreeMap;

use crate::{InputData, PathNode};

pub fn generate(input: InputData) -> BTreeMap<String, PathNode> {
    let mut files = BTreeMap::new();
    for (k, v) in input.into_inner().into_iter() {
        let mut subfiles = BTreeMap::new();
        for m in v {
            let lang = m.language.clone();
            let x: fluent_syntax::ast::Resource<String> = m.into();
            subfiles.insert(
                format!("{lang}.flt"),
                PathNode::File(fluent_syntax::serializer::serialize(&x).into_bytes()),
            );
        }
        files.insert(k, PathNode::Directory(subfiles));
    }
    files
}
