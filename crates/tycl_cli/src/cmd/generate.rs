use anyhow::Context;
use clap::Args as ClapArgs;
use std::fs;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    pub schema: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(short, long)]
    pub root: Option<String>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let source = fs::read_to_string(&args.schema)
        .with_context(|| format!("reading {}", args.schema.display()))?;
    let schema = tycl_parser::parse_schema(&source)
        .map_err(|e| anyhow::anyhow!("schema parse error: {e}"))?;

    let root_name = if let Some(entry) = schema.entries.get("$root-name") {
        match &entry.default {
            tycl_parser::Value::String(s) => s.clone(),
            _ => anyhow::bail!("$root-name must be a string value"),
        }
    } else if let Some(root) = args.root {
        root
    } else {
        anyhow::bail!("missing $root-name in schema or --root argument")
    };

    let tokens = crate::codegen::rust::generate_rust(&schema, &root_name)?;
    let formatted = prettyplease::unparse(&syn::parse_file(&tokens.to_string())?);

    if let Some(out) = args.output {
        fs::write(out, formatted)?;
    } else {
        println!("{}", formatted);
    }

    Ok(())
}
