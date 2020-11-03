use anyhow::Result;
use s4::{
    Apps, BuildContext, Config, Context, PlatformId, ProjectId, Setting, VariationId,
    WorkspaceContext,
};

fn main() -> Result<()> {
    let config = Config::load()?;

    // println!("{:#?}", config);

    let apps = Apps::try_new(config.defaults())?;
    // apps.docker()?.update()?;

    let project_id: ProjectId = "sel4test".into();
    let platform_id: PlatformId = "pc99".into();
    let variation_id: VariationId = "haswell".into();
    let arch = s4::Sel4Architecture::X86_64;

    let mut setting = Setting::default();
    setting.set_bool("mcs", true);
    // setting.set_bool("arm-hyp", true);
    println!("{}", setting);

    let project = config.project(&project_id).expect("No such project");
    // let context = WorkspaceContext::create(project_id, "sel4test")?;
    let context = WorkspaceContext::load("sel4test")?;
    let context = BuildContext::load(&context, "sel4test-pc99");
    let context = context?;
    project.init_build(&context, &apps, &config)?;
    context.ninja(&apps)?.status()?;
    project.mq_run(&context, &apps, None)?;

    // apps.repo().arg("init").arg("--help").status()?;
    // let context = context.builds().next().unwrap()?;
    // context.docker(&apps)?.run("/bin/bash").status()?;

    Ok(())
}
