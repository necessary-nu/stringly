use std::{
    collections::{BTreeSet, HashMap},
    io::{Read, Seek},
    str::FromStr,
};

use calamine::{Reader, Xlsx};
use fluent_syntax::parser::ParserError;
use heck::ToSnakeCase;
use icu::locid::LanguageIdentifier;
use reqwest::header;
use rust_xlsxwriter::{Format, Workbook, XlsxError};

use crate::{
    ir::{CIdentifier, Category, Project, TUIdentifier, TranslationUnit, TranslationUnitMap},
    BTreeKeyedSet, PathNode,
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

    let mut categories: BTreeKeyedSet<_, Category> = BTreeKeyedSet::new();

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
            .map(|(i, x)| LanguageIdentifier::from_str(&x).map(|x| (i, x)))
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

                    strings.attributes.insert(meta_key.clone(), col_str);
                } else {
                    let data = TranslationUnit {
                        key: id.clone(),
                        main: col_str.to_string(),
                        attributes: Default::default(),
                    };
                    languages
                        .get_mut(col_code)
                        .unwrap()
                        .translation_units
                        .insert(data);
                }
            }
        }

        categories.insert(Category {
            key: CIdentifier::try_from(sheet.to_snake_case()).unwrap(),
            descriptions: Default::default(),
            name: sheet.to_string(),
            default_locale: base_lang_code.clone(),
            translation_units: languages,
        });
    }

    let project = Project {
        categories,
        ..Default::default()
    };
    Ok(project)
}

const COL_WIDTH: f64 = 30.0;

fn generate_worksheet(workbook: &mut Workbook, category: &Category) -> Result<(), XlsxError> {
    let sheet = workbook.add_worksheet();
    sheet.set_name(&category.name)?;

    let mut col = 0u16;
    let mut row = 0u32;

    let header_format = Format::new().set_bold().set_font_size(8).set_text_wrap();
    sheet.write_string_with_format(row, col, "Identifier", &header_format)?;
    sheet.set_column_width(col, COL_WIDTH)?;
    col += 1;
    sheet.write_string_with_format(row, col, "Description", &header_format)?;
    sheet.set_column_width(col, COL_WIDTH)?;
    col += 1;

    let autonym = category.default_locale.to_string();
    let autonym = iso639::autonym::get(&autonym)
        .and_then(|x| x.autonym)
        .unwrap_or(&*autonym);
    let title = format!("{} ({})", autonym, category.default_locale);
    sheet.write_string_with_format(row, col, title, &header_format)?;
    sheet.set_column_width(col, COL_WIDTH)?;
    col += 1;

    for map in category.values() {
        if map.locale == category.default_locale {
            continue;
        }
        let autonym = map.locale.to_string();
        let autonym = iso639::autonym::get(&autonym)
            .and_then(|x| x.autonym)
            .unwrap_or(&*autonym);
        let title = format!("{} ({})", autonym, map.locale);
        sheet.write_string_with_format(row, col, title, &header_format)?;
        sheet.set_column_width(col, COL_WIDTH)?;
        col += 1;
    }

    sheet.set_freeze_panes(1, 2)?;
    row += 1;
    col = 0;

    let tu = category.ordered_tu_identity_keys();
    let mut index_map = HashMap::new();

    let id_format = Format::new()
        .set_font_name("Roboto Mono")
        .set_font_size(8)
        .set_text_wrap();
    let mut i = 1u32;
    for (id, attr) in tu {
        let identifier = if let Some(attr) = attr {
            format!("{}__{}", id, attr)
        } else {
            id.to_string()
        };

        sheet.write_string_with_format(row, col, identifier, &id_format)?;
        if let Some(desc) = category.descriptions.get(id) {
            col += 1;
            sheet.write_string_with_format(row, col, desc, &id_format)?;
        }
        col = 0;
        row += 1;

        index_map.insert((id, attr), i);
        i += 1;
    }

    // Reset the "cursor"
    col = 2;
    let text_wrap_format = Format::new().set_text_wrap();

    for locale in category.ordered_locale_keys() {
        let map = category.get(&locale).unwrap();
        for (id, tu) in map.iter() {
            let index = *index_map.get(&(id, None)).unwrap();
            sheet.write_string_with_format(index, col, &tu.main, &text_wrap_format)?;

            for (attr, v) in tu.attributes.iter() {
                let index = *index_map.get(&(id, Some(attr))).unwrap();
                sheet.write_string_with_format(index, col, v, &text_wrap_format)?;
            }
        }
        col += 1;
    }

    Ok(())
}

pub fn generate(project: Project) -> Result<PathNode, XlsxError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();

    if let Some(core) = project
        .categories
        .get(&CIdentifier::try_from("core").unwrap())
    {
        generate_worksheet(&mut workbook, core)?;
    }

    for category in project.categories.values() {
        if category.name == "Core" {
            continue;
        }
        generate_worksheet(&mut workbook, category)?;
    }

    Ok(PathNode::File(workbook.save_to_buffer()?))
}
