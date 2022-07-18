use std::sync::Arc;
use std::time::Duration;
use chrono::Timelike;
use log::{error, info, warn};
use poise::serenity_prelude::{Activity, GuildId};
use tokio::time;
use tokio::time::MissedTickBehavior;
use crate::{FContext, UnitResult, UserData};
use chrono_tz::Tz;

async fn manage_roles(
    ctx: poise::serenity_prelude::Context,
    guild_id: GuildId,
    user_data: Arc<UserData>
) {

    let delay = Duration::from_secs(60);
    let mut interval = time::interval(delay);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        interval.tick().await;

        let utc_now = chrono::Utc::now();

        let conf = user_data.configuration.read().await;
        let guild = match ctx.cache.guild(guild_id) {
            None => {
                warn!("Unable to get the guild {} from cache. Did the bot leave the guild?", guild_id);
                continue;
            },
            Some(guild) => guild
        };

        for (user_id, tz_string) in &conf.member_timezones {
            let tz: Tz = tz_string.parse().unwrap();
            let tz_now = utc_now.with_timezone(&tz);

            let mut member = match guild.member(&ctx, user_id).await {
                Ok(member) => member,
                Err(why) => {
                    error!("Failed to get member: {}", why);
                    continue;
                }
            };

            if tz_now.hour() >= conf.start_hour && tz_now.hour() < conf.end_hour {
                  if member.roles.contains(&conf.parent_role_id) && !member.roles.contains(&conf.child_role_id) {
                      if let Err(why) = member.add_role(&ctx, conf.child_role_id).await {
                          error!("Failed to add role to {}: {}", member.user.id, why);
                      }
                  }
            } else {
                if member.roles.contains(&conf.child_role_id) {
                    if let Err(why) = member.remove_role(&ctx, conf.child_role_id).await {
                        error!("Failed to add role to {}: {}", member.user.id, why);
                    }
                }
            }
        }
    }
}

pub async fn event_handler(
    ctx: &poise::serenity_prelude::Context,
    event: &poise::Event<'_>,
    _fctx: FContext<'_>,
    user_data: &Arc<UserData>
) -> UnitResult {
    match event {
        poise::Event::Ready { data_about_bot } => {
            info!("{} is ready", data_about_bot.user.name);
            ctx.set_activity(Activity::playing("with timezones")).await;
            ctx.dnd().await;
        },
        poise::Event::GuildCreate { guild, is_new: _ } => {
            tokio::spawn(manage_roles(ctx.clone(), guild.id, user_data.clone()));
        }
        _ => {}
    }

    Ok(())
}