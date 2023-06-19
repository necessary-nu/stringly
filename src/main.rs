use std::{path::{PathBuf}, fmt::Display};

use clap::{builder::PossibleValue, Parser, ValueEnum};
use stringly::write_path_tree;

#[derive(Debug, Clone, Copy)]
enum Target {
    Fluent,
    TypeScript,
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Target::Fluent => "Fluent",
            Target::TypeScript => "TypeScript",
        })
    }
}

impl ValueEnum for Target {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Fluent, Self::TypeScript]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Target::TypeScript => Some(PossibleValue::new("typescript").alias("ts")),
            Target::Fluent => Some(PossibleValue::new("fluent").alias("flt")),
        }
    }
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long)]
    /// Path to the output directory
    output_path: PathBuf,

    #[arg(short, long)]
    /// Path to the input .xlsx file
    input_xlsx_path: PathBuf,

    #[arg(short, long)]
    /// The target for the output
    target: Target,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    eprintln!("Generating for target: {}", args.target);

    let x = stringly::parse_xlsx(&args.input_xlsx_path)?;
    std::fs::create_dir_all(&args.output_path)?;

    let maybe_tree = match args.target {
        Target::Fluent => stringly::flt::generate(x),
        _ => return Ok(()),
    };

    let tree = match maybe_tree {
        Ok(v) => v,
        Err((_, mut errors)) => {
            eprintln!("{:?}", errors);
            return Err(errors.pop().unwrap().into());
        }
    };

    write_path_tree(&args.output_path, tree)?;
    Ok(())
}
