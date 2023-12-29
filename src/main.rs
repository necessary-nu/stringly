use std::{
    fmt::Display,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use calamine::Xlsx;
use clap::{builder::PossibleValue, Parser, ValueEnum};
use icu::locid::LanguageIdentifier;
use stringly::{flt::load_project_from_path, ir::Project, translate};

#[derive(Debug, Clone, Copy)]
enum FromFormat {
    Fluent,
    Xlsx,
}

impl FromFormat {
    pub fn file_ext(&self) -> &str {
        match self {
            FromFormat::Fluent => "ftl",
            FromFormat::Xlsx => "xlsx",
        }
    }

    pub fn validate(&self, path: &Path) -> anyhow::Result<()> {
        match self {
            FromFormat::Fluent => match stringly::flt::parse_flt(path) {
                Ok(_) => {}
                Err((_, errs)) => match errs.into_iter().next() {
                    Some(v) => return Err(v.into()),
                    None => return Err(anyhow::anyhow!("Unknown error")).into(),
                },
            },
            FromFormat::Xlsx => todo!(),
        }

        Ok(())
    }
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
    Rust,
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Target::Fluent => "Fluent",
            Target::TypeScript => "TypeScript",
            Target::Xlsx => "XLSX",
            Target::Rust => "Rust",
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
            Target::Fluent => Some(PossibleValue::new("fluent").alias("ftl").alias("flt")),
            Target::Xlsx => Some(PossibleValue::new("xlsx")),
            Target::Rust => Some(PossibleValue::new("rust").alias("rs")),
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
    Validate(ValidateArgs),
}

#[derive(Debug, Parser)]
struct ValidateArgs {
    #[arg(short, long)]
    /// Path to the input format path
    input_path: PathBuf,

    #[arg(short, long)]
    from_format: FromFormat,

    #[arg(short, long)]
    /// Validate files recursively
    recursive: bool,
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

fn load_project(from_format: FromFormat, input_path: &Path) -> anyhow::Result<Project> {
    Ok(match from_format {
        FromFormat::Fluent => load_project_from_path(input_path)?,
        FromFormat::Xlsx => {
            let xlsx: Xlsx<_> = calamine::open_workbook(input_path)?;
            Project::try_from(xlsx)?
        }
    })
}

fn generate(to_format: Target, project: Project, output_path: &Path) -> anyhow::Result<()> {
    let tree = match to_format {
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
        Target::Rust => match stringly::rust::generate(project) {
            Ok(v) => v,
            Err(error) => {
                eprintln!("{:?}", error);
                return Err(error.into());
            }
        },
    };

    tree.write(output_path)?;
    Ok(())
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
            let project = load_project(args.from_format, &args.input_path)?;

            eprintln!("Generating for format: {}", args.to_format);
            generate(args.to_format, project, &args.output_path)?;
            Ok(())
        }
        Command::Translate(args) => {
            eprintln!("Loading from format: {}", args.from_format);
            let project = load_project(args.from_format, &args.input_path)?;
            let project =
                translate::process(&project, &args.target_language, &args.google_api_key).await?;
            eprintln!("Generating for format: {}", args.to_format);
            generate(args.to_format, project, &args.output_path)?;
            Ok(())
        }
        Command::Validate(args) => {
            if args.recursive {
                let wd = walkdir::WalkDir::new(&args.input_path);
                let files = wd
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|x| x.metadata().map(|x| x.is_file()).unwrap_or(false))
                    .filter(|x| {
                        x.path()
                            .extension()
                            .map(|x| x.as_bytes())
                            .map(|ext| ext == args.from_format.file_ext().as_bytes())
                            .unwrap_or(false)
                    });

                for f in files {
                    eprintln!("Validating: {}", f.path().display());
                    args.from_format.validate(f.path())?;
                }
            } else {
                eprintln!("Validating: {}", args.input_path.display());
                args.from_format.validate(&args.input_path)?;
            }

            Ok(())
        }
    }
}
