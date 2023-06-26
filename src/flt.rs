use std::collections::BTreeMap;

use fluent_syntax::{ast::Resource, parser::ParserError};

use crate::{
    ir::{InputData, StringMap},
    PathNode,
};

pub fn generate(input: InputData) -> Result<BTreeMap<String, PathNode>, ParseError> {
    let mut files = BTreeMap::new();
    for (k, v) in input.into_inner().into_iter() {
        let mut subfiles = BTreeMap::new();
        for m in v.strings.values() {
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

pub type ParseError = (Resource<std::string::String>, Vec<ParserError>);

impl TryFrom<&StringMap> for fluent_syntax::ast::Resource<String> {
    type Error = ParseError;

    fn try_from(value: &StringMap) -> Result<Self, Self::Error> {
        let resources = value
            .strings
            .iter()
            .fold(String::new(), |mut input, (key, value)| {
                input.push_str(key);
                input.push_str(" = ");
                input.push_str(&value.base);
                input.push('\n');

                for (k, v) in value.meta.iter() {
                    input.push_str("    .");
                    input.push_str(k);
                    input.push_str(" = ");
                    input.push_str(v);
                    input.push('\n');
                }

                input
            });

        fluent_syntax::parser::parse(resources)
    }
}
