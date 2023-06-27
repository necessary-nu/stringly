use std::{fmt::Display, path::PathBuf};

use calamine::Xlsx;
use clap::{builder::PossibleValue, Parser, ValueEnum};
use icu::locid::LanguageIdentifier;
use stringly::{flt::load_project_from_path, ir::Project, translate};

#[derive(Debug, Clone, Copy)]
enum FromFormat {
    Fluent,
    Xlsx,
}

impl Display for FromFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FromFormat::Fluent => "Fluent",
            FromFormat::Xlsx => "XLSX",
        })
    }
}

impl ValueEnum for FromFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Fluent, Self::Xlsx]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Xlsx => Some(PossibleValue::new("xlsx")),
            Self::Fluent => Some(PossibleValue::new("fluent").alias("flt")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Target {
    Fluent,
    TypeScript,
    Xlsx,
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Target::Fluent => "Fluent",
            Target::TypeScript => "TypeScript",
            Target::Xlsx => "XLSX",
        })
    }
}

impl ValueEnum for Target {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Fluent, Self::TypeScript, Self::Xlsx]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Target::TypeScript => Some(PossibleValue::new("typescript").alias("ts")),
            Target::Fluent => Some(PossibleValue::new("fluent").alias("flt")),
            Target::Xlsx => Some(PossibleValue::new("xlsx")),
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
    /// Path to the input format path
    input_path: PathBuf,

    #[arg(short, long)]
    from_format: FromFormat,

    #[arg(short, long)]
    /// The target for the output
    to_format: Target,

    #[arg(short, long)]
    /// Path to the output directory
    output_path: PathBuf,
}

#[derive(Debug, Parser)]
struct TranslateArgs {
    #[arg(short, long)]
    /// Path to the input format path
    input_path: PathBuf,

    #[arg(short, long)]
    from_format: FromFormat,

    #[arg(short, long)]
    /// The target for the output
    to_format: Target,

    #[arg(short, long)]
    /// Path to the output directory
    output_path: PathBuf,

    #[arg(short = 'l', long = "language")]
    /// The target language to be translated into
    target_language: LanguageIdentifier,

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
            eprintln!("Loading from format: {}", args.from_format);

            let project = match args.from_format {
                FromFormat::Fluent => load_project_from_path(&args.input_path)?,
                FromFormat::Xlsx => {
                    let xlsx: Xlsx<_> = calamine::open_workbook(&args.input_path)?;
                    Project::try_from(xlsx)?
                }
            };

            eprintln!("Generating for format: {}", args.to_format);

            let tree = match args.to_format {
                Target::Fluent => match stringly::flt::generate(project) {
                    Ok(v) => v,
                    Err(error) => {
                        eprintln!("{:?}", error);
                        return Err(error.into());
                    }
                },
                Target::TypeScript => match stringly::ts::generate(project) {
                    Ok(v) => v,
                    Err(error) => {
                        eprintln!("{:?}", error);
                        return Err(error.into());
                    }
                },
                Target::Xlsx => match stringly::xlsx::generate(project) {
                    Ok(v) => v,
                    Err(error) => {
                        eprintln!("{:?}", error);
                        return Err(error.into());
                    }
                },
            };

            tree.write(&args.output_path)?;
            Ok(())
        }
        Command::Translate(args) => {
            eprintln!("Loading from format: {}", args.from_format);

            let project = match args.from_format {
                FromFormat::Fluent => load_project_from_path(&args.input_path)?,
                FromFormat::Xlsx => {
                    let xlsx: Xlsx<_> = calamine::open_workbook(&args.input_path)?;
                    Project::try_from(xlsx)?
                }
            };

            let project =
                translate::process(&project, &args.target_language, &args.google_api_key).await?;

            eprintln!("Generating for format: {}", args.to_format);

            let maybe_tree = match args.to_format {
                Target::Fluent => stringly::flt::generate(project),
                Target::TypeScript => stringly::ts::generate(project),
                Target::Xlsx => {
                    unimplemented!()
                }
            };

            let tree = match maybe_tree {
                Ok(v) => v,
                Err(error) => {
                    eprintln!("{:?}", error);
                    return Err(error.into());
                }
            };

            tree.write(&args.output_path)?;
            Ok(())
        }
    }
}
