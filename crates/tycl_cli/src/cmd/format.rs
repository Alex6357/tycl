use anyhow::Context;
use clap::Args as ClapArgs;
use std::fs;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let source = fs::read_to_string(&args.input)
        .with_context(|| format!("reading {}", args.input.display()))?;
    let doc = tycl_edit::Document::parse(&source)
        .map_err(|e| anyhow::anyhow!("parse error: {e:?}"))?;
    let formatted = doc.to_string();
    if let Some(out) = args.output {
        fs::write(out, formatted)?;
    } else {
        println!("{}", formatted);
    }
    Ok(())
}
