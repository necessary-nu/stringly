use std::{fmt::Display, path::PathBuf};

use clap::{builder::PossibleValue, Parser, ValueEnum};
use icu::locid::Locale;
use stringly::{translate, write_path_tree, xlsx::parse_xlsx};

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
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Parser)]
enum Command {
    Generate(GenerateArgs),
    Translate(TranslateArgs),
}

#[derive(Debug, Parser)]
struct GenerateArgs {
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

#[derive(Debug, Parser)]
struct TranslateArgs {
    #[arg(short, long)]
    /// Path to the output directory
    output_path: PathBuf,

    #[arg(short, long)]
    /// Path to the input .xlsx file
    input_xlsx_path: PathBuf,

    #[arg(short, long)]
    /// Which sheet to use
    sheet_name: Option<String>,

    #[arg(short, long)]
    /// The target language to be translated into
    target_language: Locale,

    #[arg(env = "GOOGLE_API_KEY", long = "api-key")]
    /// Google API key
    google_api_key: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}

async fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    let Some(command) = args.command else {
        eprintln!("No command specified");
        return Ok(());
    };

    match command {
        Command::Generate(args) => {
            eprintln!("Generating for target: {}", args.target);

            let x = parse_xlsx(&args.input_xlsx_path)?;
            std::fs::create_dir_all(&args.output_path)?;

            let maybe_tree = match args.target {
                Target::Fluent => stringly::flt::generate(x),
                Target::TypeScript => stringly::ts::generate(x),
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
        Command::Translate(args) => {
            let tree = translate::process(
                &args.input_xlsx_path,
                &args.target_language,
                &args.google_api_key,
            )
            .await?;
            write_path_tree(&args.output_path, tree)?;

            Ok(())
        }
    }
}
