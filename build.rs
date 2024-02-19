use std::{env, io::Result};

use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate_to, Shell};

include!("src/cli.rs");

fn main() -> Result<()> {
    let mut cli = DimOpts::command();

    let dir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(dir) => dir,
    };

    for &shell in Shell::value_variants() {
        let comp_file = generate_to(shell, &mut cli, "dim", &dir)?;
        println!("cargo:warning=generated completion for {shell}: {comp_file:?}");
    }

    Ok(())
}
