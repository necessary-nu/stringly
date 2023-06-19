use std::{collections::BTreeMap, fmt::Display};

use heck::{ToLowerCamelCase, ToPascalCase, ToShoutySnakeCase};

use crate::{InputData, ParseError, PathNode};

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

        if args.is_empty() {
            f.write_fmt(format_args!(
                "return this.#context.resolve(this.#bundles, \"{}\")\n",
                self.raw_id
            ))?;
        } else {
            f.write_fmt(format_args!(
                "return this.#context.resolve(this.#bundles, \"{}\", {{ {args} }})\n",
                self.raw_id
            ))?;
        }
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
        write!(f, " {{\n")?;
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

fn dump_flt_inline(lang: &str, res: &fluent_syntax::ast::Resource<String>) -> String {
    format!(
        "const {} = flt(\"{lang}\")`\n{}`\n",
        lang.to_shouty_snake_case(),
        fluent_syntax::serializer::serialize(&res)
    )
}

fn dump_flt_resource_map<'a>(langs: impl Iterator<Item = &'a String>) -> String {
    let inner = langs
        .map(|x| format!("{:?}: {}", x, x.to_shouty_snake_case()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("#bundles = {{\n{}\n}}\n", inner)
}

pub fn generate(input: InputData) -> Result<BTreeMap<String, PathNode>, ParseError> {
    let mut bundle_files = BTreeMap::new();
    let mut index_bundles = vec![];

    for (module_name, project) in input.into_inner().into_iter() {
        let mut flts = Vec::new();

        for (_, m) in project.strings.iter() {
            let lang = m.language.clone();
            let resource: fluent_syntax::ast::Resource<String> = m.try_into()?;

            flts.push(Ast::Body(Body::Raw(Raw(dump_flt_inline(&lang, &resource)))));
        }

        let strings = project.base_strings();
        let resource: fluent_syntax::ast::Resource<String> = strings.try_into()?;

        let ts_asts = resource
            .body
            .into_iter()
            .filter_map(|ast| match ast {
                fluent_syntax::ast::Entry::Message(x) if x.value.is_some() => {
                    let name = x.id.name;

                    Some(
                        std::iter::once((name.clone(), x.value.unwrap())).chain(
                            x.attributes
                                .into_iter()
                                .map(move |y| (format!("{}_{}", &name, y.id.name), y.value)),
                        ),
                    )
                }
                _ => None,
            })
            .flatten()
            .map(|(name, value)| {
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

                if vars.is_empty() {
                    Ast::Getter(Getter {
                        ident: Ident(name.to_lower_camel_case()),
                        body: Body::BundleGetter(BundleGetter {
                            raw_id: name,
                            args: vec![],
                        }),
                    })
                } else {
                    Ast::Method(Method {
                        ident: Ident(name.to_lower_camel_case()),
                        arguments: vars
                            .iter()
                            .map(|(camel, _real)| Param {
                                ident: camel.clone(),
                                ty: Ident("string".into()),
                            })
                            .collect(),
                        body: Body::BundleGetter(BundleGetter {
                            raw_id: name,
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

        let header: &str = "import { Context, flt } from \"../util\"\n\n";

        let ts_ast = Class {
            ident: Ident(module_name.to_pascal_case()),
            exported: true,
            implements: vec![],
            body: [Ast::Body(Body::Raw(Raw(
                dump_flt_resource_map(project.strings.keys()),
            ))), Ast::Body(Body::Raw(Raw(
                "#context: Context\nconstructor(context: Context) { this.#context = context; }\n"
                    .into(),
            )))]
            .into_iter()
            .chain(ts_asts)
            .collect(),
        };

        let module = Module {
            body: [Ast::Body(Body::Raw(Raw(header.into())))]
                .into_iter()
                .chain(flts.into_iter())
                .chain(std::iter::once(Ast::Class(ts_ast)))
                .collect(),
        };
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
    {getters}

    constructor(context: Context) {{
        {class_fields}
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

    Ok(files)
}

const UTIL_TS: &str = r#"import { FluentBundle, FluentResource } from "@fluent/bundle"

export type Context = {
    resolve: (bundles: Record<string, FluentBundle>, id: string, args?: Record<string, string>) => string | null
}

export function flt(locale: string) {
    return (input: TemplateStringsArray) => {
        const resource = new FluentResource(input.raw[0])
        const bundle = new FluentBundle(locale)
        bundle.addResource(resource)
        return bundle
    }
}

interface StringsConstructor<S> {
    new (context: Context): S;
}

export class StringsContext<S> {
    #observers: Array<(newLocale: string) => void>
    #currentLocale: string;
    #strings: S

    get locale(): string {
        return this.#currentLocale
    }
    
    constructor(type: StringsConstructor<S>, locale: string, observers: Array<(newLocale: string) => void> = []) {
        const self = this
        this.#observers = observers
        this.#currentLocale = locale
        this.#strings = new type({
            resolve(bundles: Record<string, FluentBundle>, id: string, args?: Record<string, string>) {
                const locale = self.#currentLocale
                
                const bundle = bundles[locale]
                if (bundle == null) {
                    return null
                }

                const message = bundle.getMessage(id)
                if (message == null) {
                    return null
                }

                if (message.value != null) {
                    return bundle.formatPattern(message.value, args)
                }

                return null
            }
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
        this.#currentLocale = newLocale;
        for (const observer of this.#observers) {
            observer(newLocale)
        }
    }

    get strings() {
        return this.#strings
    }
}
"#;
