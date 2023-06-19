use std::{collections::BTreeMap, path::PathBuf};

use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long)]
    /// Path to the output directory
    output_path: PathBuf,

    #[arg(short, long)]
    /// Path to the input .xlsx file
    input_xlsx_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    let x = stringly::parse_xlsx(&args.input_xlsx_path).unwrap();
    std::fs::create_dir_all(&args.output_path).unwrap();

    let mut files = BTreeMap::new();

    for (k, v) in x.into_iter() {
        let mut subfiles = BTreeMap::new();
        for m in v {
            let lang = m.language.clone();
            let x: fluent_syntax::ast::Resource<String> = m.into();
            subfiles.insert(
                format!("{lang}.flt"),
                fluent_syntax::serializer::serialize(&x),
            );
        }
        files.insert(k, subfiles);
    }

    for (k, v) in files.into_iter() {
        for (k2, v2) in v.into_iter() {
            let dir_path = args.output_path.join(&k);
            std::fs::create_dir_all(&dir_path).unwrap();
            let path = dir_path.join(&k2);
            std::fs::write(path, v2).unwrap();
        }
    }
}
