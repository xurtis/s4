use anyhow::Result;
use s4::{
    Apps, BuildContext, Config, Context, PlatformId, ProjectId, Setting, VariationId,
    WorkspaceContext,
};

fn main() -> Result<()> {
    let mut config = Config::load()?;

    // println!("{:#?}", config);

    let project_id: ProjectId = "sel4test".into();
    let platform_id: PlatformId = "odroidc2".into();
    let arch = s4::AArch64;

    let mut setting = Setting::default();
    setting.set_bool("mcs", true);
    // setting.set_bool("arm-hyp", true);
    println!("{}", setting);

    // let context = WorkspaceContext::create(project_id, "sel4test")?;
    let context = WorkspaceContext::load("sel4test")?;
    let easy_settings = context.easy_settings()?;
    let cmdline_flags = easy_settings
        .all()
        .map(|flag| flag.name().clone())
        .collect::<Vec<_>>();
    config.add_flags(easy_settings);
    println!("{:?}", cmdline_flags);
    let project = config.project(&project_id);

    let apps = Apps::try_new(config.defaults())?;
    // apps.docker()?.update()?;
    // project.init(context.workspace_root(), &apps)?;
    #[cfg(not)]
    let context = BuildContext::load(&context, "sel4test-odroidc2");
    let context = BuildContext::create(
        &config,
        &context,
        platform_id,
        None,
        arch,
        setting,
        "sel4test-odroidc2",
    );
    let context = context?;
    project.init_build(&context, &apps, &config)?;
    context.ninja(&apps)?.status()?;
    project.mq_run(&context, &config, &apps, None)?;

    // apps.repo().arg("init").arg("--help").status()?;
    // let context = context.builds().next().unwrap()?;
    // context.docker(&apps)?.run("/bin/bash").status()?;

    Ok(())
}
