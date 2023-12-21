use std::{collections::BTreeMap, fmt::Display};

use fluent_syntax::parser::ParserError;
use heck::{ToLowerCamelCase, ToPascalCase, ToShoutySnakeCase};
use icu::locid::LanguageIdentifier;

use crate::{ir::Project, PathNode};

#[derive(Debug, Clone)]
struct Interface {
    ident: Ident,
    body: Vec<Body>,
}

impl Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "interface {} {{", self.ident)?;
        for body in &self.body {
            writeln!(f, "{}", body)?;
        }
        writeln!(f, "}}")
    }
}

#[derive(Debug, Clone)]
struct Ident(String);

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
struct Param {
    ident: Ident,
    ty: Ident,
}

impl Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.ident, self.ty)
    }
}

#[derive(Debug, Clone)]
struct ObjArg {
    ident: Ident,
    value: String,
}

impl Display for ObjArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.ident.0, self.value)
    }
}

#[derive(Debug, Clone)]
struct Method {
    ident: Ident,
    arguments: Vec<Param>,
    body: Body,
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}({}) {{ {} }}",
            self.ident,
            self.arguments
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            self.body
        )
    }
}

#[derive(Debug, Clone)]
struct Getter {
    ident: Ident,
    body: Body,
}

impl Display for Getter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "get {}() {{ {} }}", self.ident, self.body)
    }
}

#[derive(Debug, Clone)]
enum Body {
    Raw(Raw),
    BundleGetter(BundleGetter),
}

impl Display for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Body::Raw(x) => x.fmt(f),
            Body::BundleGetter(x) => x.fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
struct Raw(String);

impl Display for Raw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
struct BundleGetter {
    raw_id: String,
    attr: Option<String>,
    args: Vec<ObjArg>,
}

impl Display for BundleGetter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args = self
            .args
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        let mut msg_args = String::from("{ id: \"");
        msg_args.push_str(&self.raw_id);
        msg_args.push('"');

        if let Some(attr) = &self.attr {
            msg_args.push_str(", attr: \"");
            msg_args.push_str(attr);
            msg_args.push('"');
        }

        if !args.is_empty() {
            msg_args.push_str(&format!(", args: {{ {args} }}"));
        }

        msg_args.push_str(" }");

        f.write_fmt(format_args!(
            "return this.#context.resolve(this.#bundles, {msg_args})\n",
        ))?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Class {
    ident: Ident,
    exported: bool,
    implements: Vec<Ident>,
    body: Vec<Ast>,
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.exported {
            write!(f, "export ")?;
        }
        write!(f, "class {}", self.ident)?;
        if !self.implements.is_empty() {
            write!(f, " implements ")?;
            let impls = self
                .implements
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            write!(f, "{}", impls)?;
        }
        writeln!(f, " {{")?;
        for ast in &self.body {
            write!(f, "{}", ast)?;
        }
        write!(f, "}}")
    }
}

#[derive(Debug, Clone)]
struct Module {
    body: Vec<Ast>,
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ast in &self.body {
            write!(f, "{}", ast)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum Ast {
    Method(Method),
    Getter(Getter),
    Class(Class),
    Body(Body),
}

impl Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ast::Method(x) => x.fmt(f),
            Ast::Getter(x) => x.fmt(f),
            Ast::Class(x) => x.fmt(f),
            Ast::Body(x) => x.fmt(f),
        }
    }
}

fn dump_ftl_inline(
    lang: &LanguageIdentifier,
    res: &fluent_syntax::ast::Resource<String>,
    is_core: bool,
) -> String {
    if is_core {
        format!(
            "const {} = ftl(\"{lang}\")`\n{}`\n",
            lang.to_string().to_shouty_snake_case(),
            fluent_syntax::serializer::serialize(res)
        )
    } else {
        format!(
            "const {} = ftl(\"{lang}\", coreBundles)`\n{}`\n",
            lang.to_string().to_shouty_snake_case(),
            fluent_syntax::serializer::serialize(res)
        )
    }
}

fn dump_ftl_resource_map<'a>(langs: impl Iterator<Item = &'a LanguageIdentifier>) -> String {
    langs
        .map(|x| {
            format!(
                "{:?}: {}",
                x.to_string(),
                x.to_string().to_shouty_snake_case()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn generate(input: Project) -> Result<PathNode, ParserError> {
    let mut bundle_files = BTreeMap::new();
    let mut index_bundles = vec![];

    for (module_name, category) in input.categories.into_iter() {
        let is_core = &*module_name == "core";
        let mut ftls = Vec::new();

        for (_, m) in category.translation_units.iter() {
            let lang = m.locale.clone();
            let resource = m.to_ftl_resource(&category.descriptions)?;

            ftls.push(Ast::Body(Body::Raw(Raw(dump_ftl_inline(
                &lang, &resource, is_core,
            )))));
        }

        let strings = category.base_strings();
        let resource = strings.to_ftl_resource(&category.descriptions)?;

        let ts_asts = resource
            .body
            .into_iter()
            .filter_map(|ast| match ast {
                fluent_syntax::ast::Entry::Message(x) if x.value.is_some() => {
                    let name = x.id.name;

                    let items = std::iter::once((name.clone(), None, x.value.unwrap())).chain(
                        x.attributes
                            .into_iter()
                            .map(move |y| (name.clone(), Some(y.id.name.to_string()), y.value)),
                    );
                    Some(items)
                }
                _ => None,
            })
            .flatten()
            .map(|(name, attr, value)| {
                let vars = value
                    .elements
                    .iter()
                    .filter_map(|x| match x {
                        fluent_syntax::ast::PatternElement::Placeable { expression } => {
                            Some(expression)
                        }
                        _ => None,
                    })
                    .map(|p| match p {
                        fluent_syntax::ast::Expression::Select { selector, .. } => selector,
                        fluent_syntax::ast::Expression::Inline(selector) => selector,
                    })
                    .filter_map(|p| match p {
                        fluent_syntax::ast::InlineExpression::VariableReference { id } => {
                            Some((Ident(id.name.to_lower_camel_case()), id))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                let ident = if let Some(attr) = attr.as_deref() {
                    Ident(format!("{name}__{attr}").to_lower_camel_case())
                } else {
                    Ident(format!("{name}").to_lower_camel_case())
                };

                if vars.is_empty() {
                    Ast::Getter(Getter {
                        ident,
                        body: Body::BundleGetter(BundleGetter {
                            raw_id: name,
                            attr,
                            args: vec![],
                        }),
                    })
                } else {
                    Ast::Method(Method {
                        ident,
                        arguments: vars
                            .iter()
                            .map(|(camel, _real)| Param {
                                ident: camel.clone(),
                                ty: Ident("string".into()),
                            })
                            .collect(),
                        body: Body::BundleGetter(BundleGetter {
                            raw_id: name,
                            attr,
                            args: vars
                                .iter()
                                .map(|(camel, real)| ObjArg {
                                    ident: Ident(format!("{:?}", real.name)),
                                    value: camel.to_string(),
                                })
                                .collect(),
                        }),
                    })
                }
            });

        let core_import = if is_core {
            ""
        } else {
            "import { bundles as coreBundles } from \"./core\"\n"
        };
        let header: &str = "import { Context, ftl } from \"../util\"\n\n";

        let bundles = if is_core {
            "#bundles = bundles\n".to_string()
        } else {
            format!(
                "#bundles = {{\n{}\n}}\n",
                dump_ftl_resource_map(category.translation_units.keys())
            )
        };

        let ts_ast = Class {
            ident: Ident(module_name.to_pascal_case()),
            exported: true,
            implements: vec![],
            body: [Ast::Body(Body::Raw(Raw(
                bundles
            ))), Ast::Body(Body::Raw(Raw(
                "#context: Context\nconstructor(context: Context) { this.#context = context; }\n"
                    .into(),
            )))]
            .into_iter()
            .chain(ts_asts)
            .collect(),
        };

        let mut module = Module {
            body: [Ast::Body(Body::Raw(Raw(format!("{core_import}{header}"))))]
                .into_iter()
                .chain(ftls.into_iter())
                .chain(std::iter::once(Ast::Class(ts_ast)))
                .collect(),
        };

        // Hack for core
        if is_core {
            let x = format!(
                "export const bundles = Object.freeze({{ {} }})\n\n",
                dump_ftl_resource_map(category.translation_units.keys())
            );
            module
                .body
                .insert(module.body.len() - 1, Ast::Body(Body::Raw(Raw(x))));
        }
        bundle_files.insert(
            format!("{}.ts", module_name.to_lower_camel_case()),
            PathNode::File(format!("{}", module).into_bytes()),
        );
        index_bundles.push(module_name.to_lower_camel_case());
    }

    let imports = index_bundles
        .iter()
        .map(|x| {
            let name = x.to_pascal_case();
            format!("import {{ {name} }} from \"./bundle/{x}\"")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let class_fields = index_bundles
        .iter()
        .map(|x| {
            let name = x.to_pascal_case();
            format!("this.#{x} = new {name}(context)")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let getters = index_bundles
        .iter()
        .map(|x| {
            let name = x.to_pascal_case();
            format!("#{x}: {name}\nget {x}() {{ return this.#{x} }}\n")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let class_wrapper = format!(
        "export class Strings {{
    #context: Context

    {getters}

    constructor(context: Context) {{
        {class_fields}
        this.#context = context
    }}

    clone(): Strings {{
      return new Strings(this.#context)
    }}
}}
"
    );
    let index_file = [
        "import { Context, StringsContext } from \"./util\"".to_string(),
        imports,
        class_wrapper,
        r#"export const context = new StringsContext(Strings, "en")
export const strings: Strings = context.strings"#
            .to_string(),
    ]
    .join("\n");

    let mut files = BTreeMap::new();
    files.insert("bundle".to_string(), PathNode::Directory(bundle_files));
    files.insert(
        "util.ts".to_string(),
        PathNode::File(UTIL_TS.as_bytes().to_vec()),
    );
    files.insert(
        "index.ts".to_string(),
        PathNode::File(index_file.into_bytes()),
    );

    Ok(PathNode::Directory(files))
}

const UTIL_TS: &str = r#"import { FluentBundle, FluentResource } from "@fluent/bundle"

export type MessageRequest = {
  id: string
  attr?: string
  args?: Record<string, string>
}

export type Context = {
  resolve: (
    bundles: Record<string, FluentBundle>,
    { id, attr, args }: MessageRequest
  ) => string | null
}

function mergeBundle(intoBundle: FluentBundle, fromBundle: FluentBundle) {
  for (const [k, v] of Object.entries(fromBundle._functions)) {
    intoBundle._functions[k] = v
  }

  for (const [k, v] of fromBundle._messages.entries()) {
    intoBundle._messages.set(k, v)
  }

  for (const [k, v] of fromBundle._terms.entries()) {
    intoBundle._terms.set(k, v)
  }
}

export function ftl(
  locale: string,
  coreBundles: Record<string, FluentBundle> = {}
) {
  return (input: TemplateStringsArray) => {
    const resource = new FluentResource(input.raw[0])
    const bundle = new FluentBundle(locale)
    bundle.addResource(resource)
    const coreBundle = coreBundles[locale]
    if (coreBundle != null) {
      mergeBundle(bundle, coreBundle)
    }
    return bundle
  }
}

interface StringsConstructor<S> {
  new (context: Context): S
}

export class StringsContext<S> {
  #observers: Array<(newLocale: string) => void>
  #currentLocale: string
  #strings: S

  get locale(): string {
    return this.#currentLocale
  }

  constructor(
    type: StringsConstructor<S>,
    locale: string,
    observers: Array<(newLocale: string) => void> = []
  ) {
    const self = this
    this.#observers = observers
    this.#currentLocale = locale
    this.#strings = new type({
      resolve(
        bundles: Record<string, FluentBundle>,
        { id, attr, args }: MessageRequest
      ) {
        const locale = self.#currentLocale

        const bundle = bundles[locale]
        if (bundle == null) {
          console.error("Bundle was not found for locale", locale)
          return null
        }

        const message = bundle.getMessage(id)
        if (message == null) {
          console.error("Message was not found for locale", locale, id)
          return null
        }

        let pattern

        if (attr != null) {
          pattern = message.attributes[attr]
        } else {
          pattern = message.value
        }

        if (pattern == null) {
          console.error("Pattern was not found for locale", locale, id)
          return null
        }

        return bundle.formatPattern(pattern, args)
      },
    })
  }

  addObserver(observer: (newLocale: string) => void) {
    this.#observers.push(observer)
  }

  removeObserver(observer: (newLocale: string) => void) {
    const index = this.#observers.indexOf(observer)
    if (index > -1) {
      this.#observers.splice(index, 1)
    }
  }

  setLocale(newLocale: string) {
    this.#currentLocale = newLocale
    for (const observer of this.#observers) {
      observer(newLocale)
    }
  }

  get strings() {
    return this.#strings
  }
}
"#;
