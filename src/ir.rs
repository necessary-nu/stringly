//! Intermediate representation of our localisation project:
//!
//! Project -> Category -> TranslationUnitMap -> TranslationUnit

use std::{
    collections::BTreeMap,
    convert::Infallible,
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use icu::locid::LanguageIdentifier;

use crate::{BTreeKeyedSet, Keyed};

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub default_locale: Option<LanguageIdentifier>,
    pub categories: BTreeKeyedSet<CIdentifier, Category>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            categories: BTreeKeyedSet::new(),
            name: "Untitled".to_string(),
            default_locale: None,
        }
    }
}

impl Deref for Project {
    type Target = BTreeKeyedSet<CIdentifier, Category>;

    fn deref(&self) -> &Self::Target {
        &self.categories
    }
}

impl DerefMut for Project {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.categories
    }
}

#[derive(Debug, Clone)]
pub struct Category {
    pub key: CIdentifier,
    pub name: String,
    pub default_locale: LanguageIdentifier,
    pub descriptions: BTreeMap<TUIdentifier, String>,
    pub translation_units: BTreeKeyedSet<LanguageIdentifier, TranslationUnitMap>,
}

impl Keyed<CIdentifier> for Category {
    fn key(&self) -> CIdentifier {
        self.key.clone()
    }
}

impl Category {
    pub fn base_strings(&self) -> &TranslationUnitMap {
        self.translation_units.get(&self.default_locale).unwrap()
    }

    pub fn ordered_locale_keys(&self) -> impl Iterator<Item = &LanguageIdentifier> {
        std::iter::once(&self.default_locale)
            .chain(self.keys().filter(|x| *x != &self.default_locale))
    }

    pub fn ordered_tu_identity_keys(
        &self,
    ) -> impl Iterator<Item = (&TUIdentifier, Option<&TUIdentifier>)> {
        self.base_strings()
            .iter()
            .map(|(k, v)| {
                std::iter::once((k, None)).chain(v.attributes.keys().map(move |a| (k, Some(a))))
            })
            .flatten()
    }
}

impl Deref for Category {
    type Target = BTreeKeyedSet<LanguageIdentifier, TranslationUnitMap>;

    fn deref(&self) -> &Self::Target {
        &self.translation_units
    }
}

impl DerefMut for Category {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.translation_units
    }
}

#[derive(Debug, Clone)]
pub struct TranslationUnitMap {
    pub locale: LanguageIdentifier,
    pub translation_units: BTreeKeyedSet<TUIdentifier, TranslationUnit>,
}

impl TranslationUnitMap {
    pub fn new(locale: LanguageIdentifier) -> Self {
        Self {
            locale,
            translation_units: Default::default(),
        }
    }
}

impl Keyed<LanguageIdentifier> for TranslationUnitMap {
    fn key(&self) -> LanguageIdentifier {
        self.locale.clone()
    }
}

impl Eq for TranslationUnitMap {}

impl PartialEq for TranslationUnitMap {
    fn eq(&self, other: &Self) -> bool {
        self.locale == other.locale
    }
}

impl PartialOrd for TranslationUnitMap {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.locale.partial_cmp(&other.locale) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        Some(core::cmp::Ordering::Equal)
    }
}

impl Ord for TranslationUnitMap {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.locale.cmp(&other.locale)
    }
}

impl Deref for TranslationUnitMap {
    type Target = BTreeMap<TUIdentifier, TranslationUnit>;

    fn deref(&self) -> &Self::Target {
        &self.translation_units
    }
}

impl DerefMut for TranslationUnitMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.translation_units
    }
}

#[derive(Debug, Clone)]
pub struct TranslationUnit {
    pub key: TUIdentifier,
    pub main: String,
    pub attributes: BTreeMap<TUIdentifier, String>,
}

impl Keyed<TUIdentifier> for TranslationUnit {
    fn key(&self) -> TUIdentifier {
        self.key.clone()
    }
}

// TODO: validate the identifier is a valid FLT identifier plus optional attribute
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TUIdentifier(String);

impl Deref for TUIdentifier {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for TUIdentifier {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<String> for TUIdentifier {
    type Error = Infallible;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(TUIdentifier(value))
    }
}

impl TryFrom<&String> for TUIdentifier {
    type Error = Infallible;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<&str> for TUIdentifier {
    type Error = Infallible;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl<S: AsRef<str>> From<fluent_syntax::ast::Term<S>> for TUIdentifier {
    fn from(value: fluent_syntax::ast::Term<S>) -> Self {
        TUIdentifier(format!("-{}", value.id.name.as_ref()))
    }
}

impl<S: AsRef<str>> From<&fluent_syntax::ast::Term<S>> for TUIdentifier {
    fn from(value: &fluent_syntax::ast::Term<S>) -> Self {
        TUIdentifier(format!("-{}", value.id.name.as_ref()))
    }
}

impl<S: AsRef<str>> From<fluent_syntax::ast::Message<S>> for TUIdentifier {
    fn from(value: fluent_syntax::ast::Message<S>) -> Self {
        TUIdentifier(value.id.name.as_ref().to_string())
    }
}

impl<S: AsRef<str>> From<&fluent_syntax::ast::Message<S>> for TUIdentifier {
    fn from(value: &fluent_syntax::ast::Message<S>) -> Self {
        TUIdentifier(value.id.name.as_ref().to_string())
    }
}

impl<S: AsRef<str>> From<fluent_syntax::ast::Attribute<S>> for TUIdentifier {
    fn from(value: fluent_syntax::ast::Attribute<S>) -> Self {
        TUIdentifier(value.id.name.as_ref().to_string())
    }
}

impl<S: AsRef<str>> From<&fluent_syntax::ast::Attribute<S>> for TUIdentifier {
    fn from(value: &fluent_syntax::ast::Attribute<S>) -> Self {
        TUIdentifier(value.id.name.as_ref().to_string())
    }
}

impl Display for TUIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// TODO: validate the category name is a snaky boy
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CIdentifier(String);

impl Deref for CIdentifier {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for CIdentifier {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<String> for CIdentifier {
    type Error = Infallible;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(CIdentifier(value))
    }
}

impl TryFrom<&String> for CIdentifier {
    type Error = Infallible;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<&str> for CIdentifier {
    type Error = Infallible;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl Display for CIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
