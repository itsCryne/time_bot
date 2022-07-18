mod commands;
mod events;

use std::collections::HashMap;
use std::process::exit;
use std::sync::Arc;
use log::{error, info, Level};
use poise::{Framework, FrameworkBuilder, FrameworkOptions, PrefixFrameworkOptions};
use poise::serenity_prelude::{GatewayIntents, RoleId, UserId};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::commands::register;
use crate::commands::set_timezone::set_timezone;
use serde_with::{serde_as, DisplayFromStr};
use crate::events::event_handler;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type UnitResult = Result<(), Error>;
pub type Context<'a> = poise::Context<'a, Arc<UserData>, Error>;
pub type FContext<'a> = poise::FrameworkContext<'a, Arc<UserData>, Error>;

#[serde_as]
#[derive(Deserialize, Serialize)]
struct Configuration {
    start_hour: u32,
    end_hour: u32,
    parent_role_id: RoleId,
    child_role_id: RoleId,
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    member_timezones: HashMap<UserId, String>
}
pub struct UserData {
    configuration: RwLock<Configuration>,
}

async fn setup_user_data() -> Result<UserData, Error> {
    let configuration_string = match std::fs::read_to_string("conf/conf.toml") {
        Ok(configuration) => configuration,
        Err(why) => {
            error!("Failed to read the `conf/conf.toml`-File: {}", why);
            return Err(Box::new(why));
        }
    };

    let configuration = match toml::from_str(configuration_string.as_str()) {
        Ok(configuration) => configuration,
        Err(why) => {
            error!("Failed to deserialize the configuration: {}", why);
            return Err(Box::new(why));
        }
    };

    Ok(UserData {
        configuration: RwLock::new(configuration)
    })
}

fn setup_logger() -> UnitResult {
    fern::Dispatch::new()
        .format(
            |callback, args, record| {
                let current_time = chrono::Local::now().format(
                    "%X.%3f %d.%m.%Y"
                ).to_string();

                callback.finish(format_args!(
                    "[{}][{}][{}] {}",
                    current_time,
                    record.level(),
                    record.target(),
                    args
                ))
            }
        )
        .filter(|meta| {
            meta.level() <= Level::Warn || meta.target().contains("time_bot")
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() -> UnitResult {
    if let Err(why) = setup_logger() {
        panic!("Failed to setup logging: {}", why);
    }

    let token = match std::fs::read_to_string("conf/token") {
        Ok(token) => token,
        Err(why) => {
            error!("Failed to read the `conf/token`-File: {}", why);
            return Ok(());
        }
    };

    let user_data = match setup_user_data().await {
        Ok(user_data) => user_data,
        Err(why) => {
            error!("Failed to setup the user-data: {}", why);
            return Ok(());
        }
    };

    let framework: Arc<Framework<Arc<UserData>, Error>> = match FrameworkBuilder::default()
        .token(token)
        .intents(GatewayIntents::all())
        .options(FrameworkOptions {
            commands: vec![
                register(),
                set_timezone()
            ],
            listener: |ctx, event, fctx, user_data| Box::pin(
                event_handler(ctx, event, fctx, user_data)
            ),
            prefix_options: PrefixFrameworkOptions {
                mention_as_prefix: true,
                prefix: Some(String::from(".")),
                ..Default::default()
            },
            command_check: Some(|ctx| Box::pin(async move {
                if let Some(member) = ctx.author_member().await {
                    let data = ctx.data();
                    let conf = data.configuration.read().await;
                    return Ok(member.roles.contains(&conf.parent_role_id))
                }
                Ok(false)
            })),
            ..Default::default()
        }

        )
        .user_data_setup(|_, _, _| Box::pin(async {
            Ok(Arc::new(user_data))
        }))
        .build().await {
        Ok(framework) => framework,
        Err(why) => {
            error!("Failed to build the framework: {}", why);
            return Ok(());
        }
    };

    let shard_manager = framework.shard_manager();
    tokio::spawn(async move {
        if let Err(why) = tokio::signal::ctrl_c().await {
            error!("Error while listening to CTRL+C signals: {}", why);
            exit(1);
        }
        info!("Received CTRL+C - Exiting");
        shard_manager.lock().await.shutdown_all().await;
    });

    info!("Starting the bot!");
    if let Err(why) = framework.start().await {
        error!("Error while starting/running the bot: {}", why);
        return Ok(());
    }

    Ok(())
}
