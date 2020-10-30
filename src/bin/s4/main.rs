use anyhow::Result;
use s4::{Apps, Config};

fn main() -> Result<()> {
    let config = Config::load()?;

    // println!("{:#?}", config);

    let apps = Apps::try_new(config.defaults())?;

    apps.repo().arg("init").arg("--help").status()?;
    apps.docker()?.run("/bin/bash").status()?;

    Ok(())
}
