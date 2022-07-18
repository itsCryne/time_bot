use poise::command;
use crate::{Context, UnitResult};
use chrono_tz::Tz;
use log::warn;
use crate::commands::save_configuration;

#[command(slash_command)]
pub async fn set_timezone(ctx: Context<'_>, #[description = "TZ database name"] tzname: String) -> UnitResult {
    ctx.defer_ephemeral().await?;

    if let Err(why) = tzname.parse::<Tz>() {
        ctx.say(format!("Failed to parse the timezone: {}\nMake sure you use the `TZ database name` from https://en.wikipedia.org/wiki/List_of_tz_database_time_zones", why)).await?;
        return Ok(());
    };

    let data = ctx.data();

    {
        let mut conf = data.configuration.write().await;
        conf.member_timezones.insert(ctx.author().id, tzname.clone());
    }

    if let Err(why) = save_configuration(ctx).await {
        warn!("Failed to save the configuration file! {} changing their timezone to {} will not persist after a restart: {}",
            ctx.author().id,
            tzname,
            why
        )
    }

    ctx.say(format!("Your timezone now is `{}`", tzname)).await?;

    Ok(())
}