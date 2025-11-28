use anyhow::Result;

pub fn update() -> Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("JohnAnon9771")
        .repo_name("devobox")
        .bin_name("devobox")
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()?
        .update()?;

    println!("Update status: `{}`!", status.version());
    Ok(())
}
