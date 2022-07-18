pub mod set_timezone;

use std::ops::Deref;
use crate::{Context, UnitResult};
use poise::command;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[command(prefix_command, owners_only)]
pub async fn register(ctx: Context<'_>) -> UnitResult {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

pub async fn save_configuration(ctx: Context<'_>) -> UnitResult {
    let data = ctx.data();
    let conf = data.configuration.read().await;

    let conf_string = toml::to_string(&conf.deref())?;

    let mut file = OpenOptions::new().write(true).truncate(true).open("conf/conf.toml").await?;
    file.write(conf_string.as_bytes()).await?;

    Ok(())
}