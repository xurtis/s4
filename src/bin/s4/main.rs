use anyhow::Result;
use s4::{
    Apps, BuildContext, Config, Context, PlatformId, ProjectId, VariationId, WorkspaceContext,
};

fn main() -> Result<()> {
    let config = Config::load()?;

    // println!("{:#?}", config);

    let apps = Apps::try_new(config.defaults())?;
    // apps.docker()?.update()?;

    let project_id: ProjectId = "sel4test".into();
    let platform_id: PlatformId = "pc99".into();
    let variation_id: VariationId = "nehalem".into();
    let arch = s4::Sel4Architecture::X86_64;

    let mut setting = config.variation_setting(&project_id, &platform_id, &variation_id, arch);
    setting.set_bool("mcs", true);
    // setting.set_bool("arm-hyp", true);
    println!("{}", setting);

    let project = config.project(&project_id).expect("No such project");
    // let context = WorkspaceContext::create(project_id, "sel4test")?;
    let context = WorkspaceContext::load("sel4test")?;
    // project.init(context.workspace_root(), &apps)?;
    let context = BuildContext::create(context, setting, "sel4test-pc99");
    let context = context?;
    project.init_build(&context, &apps, &config)?;

    let context = Box::new(context).workspace();

    for build in context.builds() {
        let build = build?;
        build.ninja(&apps)?.status()?;
    }

    // apps.repo().arg("init").arg("--help").status()?;
    // let context = context.builds().next().unwrap()?;
    // context.docker(&apps)?.run("/bin/bash").status()?;

    Ok(())
}
