use std::collections::BTreeMap;

use fluent_syntax::{ast::Resource, parser::ParserError};

use crate::{
    ir::{Project, TranslationUnitMap},
    PathNode,
};

pub fn generate(input: Project) -> Result<BTreeMap<String, PathNode>, ParseError> {
    let mut files = BTreeMap::new();
    for (k, v) in input.categories.into_iter() {
        let mut subfiles = BTreeMap::new();
        for m in v.translation_units.values() {
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

impl TryFrom<&TranslationUnitMap> for fluent_syntax::ast::Resource<String> {
    type Error = ParseError;

    fn try_from(value: &TranslationUnitMap) -> Result<Self, Self::Error> {
        let resources =
            value
                .translation_units
                .iter()
                .fold(String::new(), |mut input, (key, value)| {
                    input.push_str(key);
                    input.push_str(" = ");
                    input.push_str(&value.main);
                    input.push('\n');

                    for (k, v) in value.attributes.iter() {
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
