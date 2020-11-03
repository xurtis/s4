use anyhow::Result;
use s4::{Apps, BuildContext, Config, Context, ProjectId, WorkspaceContext};

fn main() -> Result<()> {
    let config = Config::load()?;

    // println!("{:#?}", config);

    let apps = Apps::try_new(config.defaults())?;
    // apps.docker()?.update()?;

    // let project_id: ProjectId = "sel4test".to_owned().into();

    // let project = config.project(&project_id).expect("No such project");
    // let context = WorkspaceContext::create(project_id, "sel4test")?;
    let context = WorkspaceContext::load("sel4test")?;
    // project.init(context.workspace_root(), &apps)?;
    // let context = BuildContext::create(context, "sel4test-build")?;
    // project.init_build(&context, &apps)?.status()?;
    // context.ninja(&apps)?.status()?;

    for build in context.builds() {
        let build = build?;
        build.ninja(&apps)?.status()?;
    }

    apps.repo().arg("init").arg("--help").status()?;
    let context = context.builds().next().unwrap()?;
    context.docker(&apps)?.run("/bin/bash").status()?;

    Ok(())
}
