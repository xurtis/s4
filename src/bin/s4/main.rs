use anyhow::Result;
use s4::Config;

fn main() -> Result<()> {
    let config = Config::builtin()?;

    println!("{:#?}", config);

    Ok(())
}
