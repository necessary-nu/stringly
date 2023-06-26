use std::{collections::BTreeMap, sync::OnceLock};

use icu::locid::Locale;
use regex::{Captures, Regex};
use serde::Deserialize;
use serde_json::json;

use crate::{
    ir::{Project, TUIdentifier, TranslationUnit, TranslationUnitMap},
    PathNode,
};

const GOOGLE_TRANSLATE_URL: &str = "https://translation.googleapis.com/language/translate/v2";

#[derive(Debug, Clone, Deserialize)]
struct TranslateResponse {
    data: TranslateData,
}

#[derive(Debug, Clone, Deserialize)]
struct TranslateData {
    translations: Vec<TranslateItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslateItem {
    translated_text: String,
}

pub struct KeyedString {
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct KeyedTranslation<'a> {
    pub key: &'a str,
    pub source: &'a str,
    pub target: String,
}

async fn translate<'a>(
    api_key: &str,
    segments: &'a [KeyedString],
    source_locale: &Locale,
    target_locale: &Locale,
) -> Result<Vec<KeyedTranslation<'a>>, reqwest::Error> {
    let client = reqwest::Client::builder().build()?;
    let mut translated = vec![];

    let source_language = source_locale.id.language.to_string();
    let target_language = target_locale.id.language.to_string();

    for q in segments.chunks(128) {
        let response = client
            .post(GOOGLE_TRANSLATE_URL)
            .query(&[("key", &api_key)])
            .json(&json!({
                "q": q.iter().map(|s| &s.value).collect::<Vec<_>>(),
                "source": &source_language,
                "target": &target_language
            }))
            .send()
            .await?
            .error_for_status()?;

        let response: TranslateResponse = response.json().await?;
        for (target, KeyedString { key, value: source }) in
            response.data.translations.into_iter().zip(q)
        {
            translated.push(KeyedTranslation {
                key,
                source,
                target: target.translated_text,
            });
        }
    }

    Ok(translated)
}

static TO_HTML_REGEX: std::sync::OnceLock<Regex> = OnceLock::new();
static FROM_HTML_REGEX: std::sync::OnceLock<Regex> = OnceLock::new();

fn convert_to_html(text: &str) -> String {
    let regex = TO_HTML_REGEX.get_or_init(|| Regex::new(r"\{\s*\$(.+?)\s*\}").unwrap());
    regex
        .replace_all(text, |c: &Captures| {
            format!("<a id=\"{}\">{}</a>", &c[1], &c[1])
        })
        .to_string()
}

fn convert_from_html(text: &str) -> String {
    let regex = FROM_HTML_REGEX.get_or_init(|| Regex::new(r#"<a id="(.+?)">.+?</a>"#).unwrap());
    regex
        .replace_all(text, |c: &Captures| format!("{{ ${} }}", &c[1]))
        .to_string()
}

pub async fn process(
    input: &Project,
    target_language: &Locale,
    google_api_key: &str,
) -> anyhow::Result<BTreeMap<String, PathNode>> {
    let mut files = BTreeMap::new();

    for (k, v) in input.categories.iter() {
        let mut subfiles = BTreeMap::new();
        let source_language = &v.base_locale;

        let strings = v
            .base_strings()
            .translation_units
            .iter()
            .flat_map(|(key, x)| {
                let source = convert_to_html(&x.main);
                std::iter::once(KeyedString {
                    key: key.to_string(),
                    value: source,
                })
                .chain(x.attributes.iter().map(move |x| {
                    let source = convert_to_html(x.1);

                    KeyedString {
                        key: format!("{key}__{}", x.0),
                        value: source,
                    }
                }))
            })
            .collect::<Vec<_>>();

        let strings = translate(google_api_key, &strings, source_language, target_language).await?;

        let mut out = TranslationUnitMap {
            locale: target_language.clone(),
            translation_units: BTreeMap::new(),
        };

        for x in strings.into_iter() {
            let mut iter = x.key.split("__");
            let base_id = TUIdentifier::try_from(iter.next().unwrap()).unwrap();
            let meta_id = match iter.next() {
                Some(v) => Some(TUIdentifier::try_from(v).unwrap()),
                None => None,
            };

            if let Some(meta_id) = meta_id {
                let map = out.translation_units.get_mut(&base_id).unwrap();
                map.attributes
                    .insert(meta_id.to_string(), convert_from_html(&x.target));
            } else {
                out.translation_units.insert(
                    base_id.clone(),
                    TranslationUnit {
                        main: convert_from_html(&x.target),
                        attributes: Default::default(),
                    },
                );
            }
        }

        let x: fluent_syntax::ast::Resource<String> = (&out).try_into().unwrap();
        subfiles.insert(
            format!("{target_language}.flt"),
            PathNode::File(fluent_syntax::serializer::serialize(&x).into_bytes()),
        );

        files.insert(k.to_string(), PathNode::Directory(subfiles));
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_e2e() {
        let test = "This is { $var } and also { $upsetting-var }.";
        let html = convert_to_html(test);
        assert_eq!(
            html,
            "This is <a id=\"var\">var</a> and also <a id=\"upsetting-var\">upsetting-var</a>."
        );
        let text = convert_from_html(&html);
        assert_eq!(text, "This is { $var } and also { $upsetting-var }.")
    }
}
