use std::{collections::BTreeMap, ops::Deref, path::Path, str::FromStr};

use fluent_syntax::{ast, parser::ParserError};
use icu::locid::{locale, LanguageIdentifier};
use serde::{Deserialize, Serialize};

use crate::{
    ir::{CIdentifier, Category, Project, TUIdentifier, TranslationUnit, TranslationUnitMap},
    PathNode,
};

mod serializer;

#[derive(Debug, Serialize, Deserialize)]
struct ProjectConfig {
    name: String,
    default_locale: Option<LanguageIdentifier>,
    #[serde(flatten)]
    categories: BTreeMap<String, CategoryConfig>,
}

fn default_locale() -> LanguageIdentifier {
    locale!("en").id
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoryConfig {
    name: String,
    #[serde(default = "default_locale")]
    default_locale: LanguageIdentifier,
}

pub fn generate(input: Project) -> Result<PathNode, ParserError> {
    let mut files = BTreeMap::new();

    let mut config = ProjectConfig {
        name: input.name,
        default_locale: input.default_locale,
        categories: Default::default(),
    };

    for (k, v) in input.categories.into_iter() {
        config.categories.insert(
            k.to_string(),
            CategoryConfig {
                name: v.name,
                default_locale: v.default_locale,
            },
        );
        let mut subfiles = BTreeMap::new();
        for m in v.translation_units.values() {
            let lang = m.locale.clone();
            let x: ast::Resource<String> = match m.try_into() {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("Error parsing translation unit: {} {}", k, m.locale);
                    eprintln!("{:?}", e);
                    std::process::exit(1);
                }
            };
            subfiles.insert(
                format!("{lang}.flt"),
                PathNode::File(fluent_syntax::serializer::serialize(&x).into_bytes()),
            );
        }
        files.insert(k.to_string(), PathNode::Directory(subfiles));
    }

    files.insert(
        "stringly.toml".into(),
        PathNode::File(toml::to_string(&config).unwrap().into_bytes()),
    );

    Ok(PathNode::Directory(files))
}

pub fn load_project_from_path(path: &Path) -> anyhow::Result<Project> {
    let config = std::fs::read_to_string(path.join("stringly.toml"))?;
    let config: ProjectConfig = toml::from_str(&config)?;

    let mut project = Project {
        name: config.name,
        default_locale: config.default_locale,
        categories: Default::default(),
    };

    for (category_id, category) in config.categories.into_iter() {
        let dir = path.join(&category_id).read_dir()?;
        let category_id = CIdentifier::try_from(category_id).unwrap();

        let mut category = Category {
            key: category_id.clone(),
            name: category.name,
            default_locale: category.default_locale.clone(),
            translation_units: Default::default(),
        };

        let iter = dir
            .filter_map(Result::ok)
            .filter(|x| {
                x.path()
                    .extension()
                    .filter(|x| x.to_str().unwrap_or_default() == "flt")
                    .is_some()
            })
            .map(|x| x.path());

        for flt_path in iter {
            let locale_str = flt_path.file_stem().and_then(|x| x.to_str()).unwrap();
            let locale = LanguageIdentifier::from_str(locale_str).unwrap();
            let flt_str = std::fs::read_to_string(flt_path)?;
            let flt: ast::Resource<String> = fluent_syntax::parser::parse(flt_str).unwrap();
            category
                .translation_units
                .insert(TranslationUnitMap::from_flt_resource(locale, &flt));
        }

        project.categories.insert(category);
    }

    Ok(project)
}

impl TranslationUnitMap {
    pub fn from_flt_resource(
        default_locale: LanguageIdentifier,
        value: &ast::Resource<String>,
    ) -> Self {
        let mut tm = TranslationUnitMap::new(default_locale);

        for resource in value.body.iter() {
            match resource {
                ast::Entry::Message(x) => {
                    let tu_id = TUIdentifier::from(x);
                    let main = serializer::serialize_pattern(x.value.as_ref().unwrap());
                    let attributes = x
                        .attributes
                        .iter()
                        .map(|x| {
                            (
                                TUIdentifier::from(x),
                                serializer::serialize_pattern(&x.value),
                            )
                        })
                        .collect();
                    tm.translation_units.insert(TranslationUnit {
                        key: tu_id,
                        main,
                        attributes,
                    });
                }
                ast::Entry::Term(x) => {
                    let tu_id = TUIdentifier::from(x);
                    let main = serializer::serialize_pattern(&x.value);
                    let attributes = x
                        .attributes
                        .iter()
                        .map(|x| {
                            (
                                TUIdentifier::from(x),
                                serializer::serialize_pattern(&x.value),
                            )
                        })
                        .collect();

                    tm.translation_units.insert(TranslationUnit {
                        key: tu_id,
                        main,
                        attributes,
                    });
                }
                _ => {}
            }
        }

        tm
    }
}

impl TryFrom<&TranslationUnitMap> for ast::Resource<String> {
    type Error = ParserError;

    fn try_from(value: &TranslationUnitMap) -> Result<Self, Self::Error> {
        let resources =
            value
                .translation_units
                .iter()
                .fold(String::new(), |mut input, (key, value)| {
                    let message = ast::Message {
                        id: ast::Identifier { name: key.deref() },
                        value: Some(ast::Pattern {
                            elements: vec![ast::PatternElement::TextElement {
                                value: &*value.main,
                            }],
                        }),
                        attributes: value
                            .attributes
                            .iter()
                            .map(|(k, v)| ast::Attribute {
                                id: ast::Identifier { name: k.deref() },
                                value: ast::Pattern {
                                    elements: vec![ast::PatternElement::TextElement { value: v }],
                                },
                            })
                            .collect::<Vec<_>>(),
                        comment: None,
                    };

                    input.push_str(&serializer::serialize_message(&message));

                    // input.push_str(key);
                    // input.push_str(" = ");
                    // input.push_str(&value.main);
                    // input.push('\n');

                    // for (k, v) in value.attributes.iter() {
                    //     input.push_str("    .");
                    //     input.push_str(k);
                    //     input.push_str(" = ");
                    //     input.push_str(v);
                    //     input.push('\n');
                    // }

                    input
                });

        fluent_syntax::parser::parse(resources).map_err(|(_, mut errors)| errors.remove(0))
    }
}
