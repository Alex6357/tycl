use anyhow::Context;
use clap::{Args as ClapArgs, ValueEnum};
use std::fs;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    pub input: PathBuf,
    #[arg(short, long)]
    pub schema: Option<PathBuf>,
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormat {
    Json,
    Toml,
    Yaml,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let source = fs::read_to_string(&args.input)
        .with_context(|| format!("reading {}", args.input.display()))?;

    let doc = if let Some(schema_path) = &args.schema {
        let schema_source = fs::read_to_string(schema_path)
            .with_context(|| format!("reading schema {}", schema_path.display()))?;
        let schema = tycl_parser::parse_schema(&schema_source)
            .map_err(|e| anyhow::anyhow!("schema parse error: {e}"))?;
        tycl_parser::parse_with_schema(&source, &schema)
            .map_err(|e| anyhow::anyhow!("document parse error: {e}"))?
    } else {
        tycl_parser::parse(&source).map_err(|e| anyhow::anyhow!("document parse error: {e}"))?
    };

    let serialized = match args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&doc)?,
        OutputFormat::Toml => toml::to_string_pretty(&doc)?,
        OutputFormat::Yaml => serde_yaml::to_string(&doc)?,
    };

    if let Some(out) = args.output {
        fs::write(out, serialized)?;
    } else {
        println!("{}", serialized);
    }

    Ok(())
}
