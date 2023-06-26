use std::{
    collections::BTreeSet,
    io::{Read, Seek},
    str::FromStr,
};

use calamine::{Reader, Xlsx};
use heck::ToSnakeCase;
use icu::locid::Locale;

use crate::{
    ir::{CIdentifier, Category, Project, TUIdentifier, TranslationUnit, TranslationUnitMap},
    BTreeKeyedSet,
};

impl<T> TryFrom<Xlsx<T>> for Project
where
    T: Read + Seek,
{
    type Error = anyhow::Error;

    fn try_from(value: Xlsx<T>) -> Result<Self, Self::Error> {
        parse_xlsx(value)
    }
}

fn parse_xlsx<T>(mut workbook: Xlsx<T>) -> anyhow::Result<Project>
where
    T: Read + Seek,
{
    let sheets = workbook
        .worksheets()
        .iter()
        .cloned()
        .map(|x| x.0)
        .filter(|x| *x != "TODO")
        .collect::<Vec<_>>();

    let mut categories: BTreeKeyedSet<_, Category> = BTreeKeyedSet::new(|category: &Category| {
        CIdentifier::try_from(category.name.to_snake_case()).unwrap()
    });

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
            .filter(|(_, x)| x.starts_with('(') && x.ends_with(')'))
            .map(|(i, x)| {
                (
                    i,
                    x.trim_start_matches('(').trim_end_matches(')').to_string(),
                )
            })
            .map(|(i, x)| Locale::from_str(&x).map(|x| (i, x)))
            .collect::<Result<Vec<_>, _>>()?;

        let Some((base_lang_idx, base_lang_code)) = lang_cols.first() else {
            eprintln!("[{}] No base language found in sheet; skipping", sheet);
            continue;
        };

        let mut languages = BTreeKeyedSet::from_set(
            lang_cols
                .iter()
                .map(|(_, x)| TranslationUnitMap {
                    locale: x.clone(),
                    translation_units: Default::default(),
                })
                .collect::<BTreeSet<_>>(),
            |x| x.locale.clone(),
        );

        for (row_idx, row) in rows {
            let Some(id) = row.get(id_idx).unwrap().as_string() else {
                eprintln!("[{}] No identifier found at row {}; skipping", &sheet, row_idx);
                continue;
            };
            let mut chunks = id.split("__");
            let id = TUIdentifier::try_from(chunks.next().unwrap())?;
            let meta_key = match chunks.next() {
                Some(v) => Some(TUIdentifier::from_str(v)?),
                None => None,
            };

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

                if let Some(meta_key) = meta_key.as_ref() {
                    let strings = languages
                        .get_mut(col_code)
                        .unwrap()
                        .translation_units
                        .get_mut(&id);
                    let strings = match strings {
                        Some(v) => v,
                        None => {
                            eprintln!(
                                "[{}] No parent string found for attribute at row {}; skipping",
                                &sheet, row_idx
                            );
                            continue;
                        }
                    };

                    strings.attributes.insert(meta_key.to_string(), col_str);
                } else {
                    let data = TranslationUnit {
                        main: col_str.to_string(),
                        attributes: Default::default(),
                    };
                    languages
                        .get_mut(col_code)
                        .unwrap()
                        .translation_units
                        .insert(id.clone(), data);
                }
            }
        }

        categories.insert(Category {
            name: sheet.to_string(),
            base_locale: base_lang_code.clone(),
            translation_units: languages,
        });
    }

    let project = Project { categories };
    Ok(project)
}
