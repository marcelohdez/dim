use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct DimOpts {
    #[arg(short, long, help = "Duration in seconds")]
    pub duration: Option<u64>,
}
