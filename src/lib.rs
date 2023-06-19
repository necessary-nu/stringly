use std::{collections::BTreeMap, ops::Deref, path::Path};

use calamine::{Reader, Xlsx};
use fluent_syntax::{parser::ParserError, ast::Resource};
use heck::ToSnakeCase;

pub mod flt;
pub mod ts;

pub enum PathNode {
    File(Vec<u8>),
    Directory(BTreeMap<String, PathNode>),
}

#[derive(Debug)]
pub struct StringMap {
    pub language: String,
    pub strings: BTreeMap<String, StringData>,
}

pub type ParseError = (Resource<std::string::String>, Vec<ParserError>);

impl TryFrom<StringMap> for fluent_syntax::ast::Resource<String> {
    type Error = ParseError;

    fn try_from(value: StringMap) -> Result<Self, Self::Error> {
        let resources = value.strings.into_iter().fold(String::new(), |mut input, (key, value)| {
            input.push_str(&key);
            input.push_str(" = ");
            input.push_str(&value.base);
            input.push('\n');

            for (k, v) in value.meta.into_iter() {
                input.push_str("    .");
                input.push_str(&k);
                input.push_str(" = ");
                input.push_str(&v);
                input.push('\n');
            }

            input
        });

        fluent_syntax::parser::parse(resources)
    }
}

#[derive(Debug)]
pub struct StringData {
    pub base: String,
    pub meta: BTreeMap<String, String>,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct InputData(BTreeMap<String, Vec<StringMap>>);

impl InputData {
    pub fn into_inner(self) -> BTreeMap<String, Vec<StringMap>> {
        self.0
    }
}

impl Deref for InputData {
    type Target = BTreeMap<String, Vec<StringMap>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn parse_xlsx(xlsx_path: &Path) -> anyhow::Result<InputData> {
    let mut workbook: Xlsx<_> = calamine::open_workbook(xlsx_path)?;
    let sheets = workbook
        .worksheets()
        .iter()
        .cloned()
        .map(|x| x.0)
        .filter(|x| *x != "TODO")
        .collect::<Vec<_>>();

    let mut projects = BTreeMap::new();

    for sheet in sheets {
        let range = workbook.worksheet_range(&sheet).unwrap()?;
        let mut rows = range.rows().enumerate();
        let headers = rows.next().unwrap();

        // Collect the headers and their index
        let Some(id_idx) = headers.1.iter().position(|x| x.as_string().as_deref() == Some("Identifier")) else {
            eprintln!("[{}] No identifier column found in sheet; skipping", sheet);
            continue;
        };

        // Collect columns with language codes
        let lang_cols = headers
            .1
            .iter()
            .enumerate()
            .filter_map(|(i, x)| x.as_string().as_deref().map(|x| (i, x.trim().to_string())))
            .filter_map(|(i, x)| x.split_whitespace().last().map(|x| (i, x.to_string())))
            .filter(|(_, x)| x.starts_with("(") && x.ends_with(")"))
            .map(|(i, x)| {
                (
                    i,
                    x.trim_start_matches('(').trim_end_matches(')').to_string(),
                )
            })
            .collect::<Vec<_>>();

        let Some((base_lang_idx, _base_lang_code)) = lang_cols.first() else {
            eprintln!("[{}] No base language found in sheet; skipping", sheet);
            continue;
        };

        let mut languages = lang_cols
            .iter()
            .map(|(_, x)| {
                (
                    x,
                    StringMap {
                        language: x.to_string(),
                        strings: Default::default(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        while let Some((row_idx, row)) = rows.next() {
            let Some(id) = row.get(id_idx).unwrap().as_string() else {
                eprintln!("[{}] No identifier found at row {}; skipping", &sheet, row_idx);
                continue;
            };
            let mut chunks = id.split("__");
            let id = chunks.next().unwrap();
            let meta_key = chunks.next();

            let Some(_base_str) = row.get(*base_lang_idx).unwrap().as_string() else {
                eprintln!("[{}] No base string found at row {}; skipping", &sheet, row_idx);
                continue;
            };

            for (col_idx, col_code) in lang_cols.iter() {
                let col_str = match row
                    .get(*col_idx)
                    .unwrap()
                    .as_string()
                    .filter(|x| !x.trim().is_empty())
                {
                    Some(v) => v,
                    None => continue,
                };

                if let Some(meta_key) = meta_key {
                    let strings = languages
                        .get_mut(col_code)
                        .unwrap()
                        .strings
                        .get_mut(id);
                    let strings = match strings {
                        Some(v) => v,
                        None => {
                            eprintln!("[{}] No parent string found for attribute at row {}; skipping", &sheet, row_idx);
                            continue;
                        }
                    };

                    strings
                        .meta
                        .insert(meta_key.to_string(), col_str);
                } else {
                    let data = StringData {
                        base: col_str.to_string(),
                        meta: Default::default(),
                    };
                    languages
                        .get_mut(col_code)
                        .unwrap()
                        .strings
                        .insert(id.to_string(), data);
                }
            }
        }

        projects.insert(
            sheet.to_snake_case(),
            languages.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
        );
    }

    Ok(InputData(projects))
}

pub fn write_path_tree(prefix: &Path, tree: BTreeMap<String, PathNode>) -> std::io::Result<()> {
    for (k, v) in tree.into_iter() {
        let path = prefix.join(&k);
        match v {
            PathNode::File(data) => {
                std::fs::write(path, data)?;
            }
            PathNode::Directory(tree) => {
                std::fs::create_dir_all(&path)?;
                write_path_tree(&prefix.join(&k), tree)?;
            }
        }
    }

    Ok(())
}
