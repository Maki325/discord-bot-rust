use std::env;
use std::fs;
use dotenv::dotenv;
use std::time::Duration;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use serenity::async_trait;
use serenity::prelude::*;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};
use serenity::model::gateway::Ready;
use serenity::model::prelude::Message;
use serenity::model::guild::{Emoji, Role};
use serenity::builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption};
use tracing::{info};
// use tracing_subscriber::FmtSubscriber;

#[group]
#[commands(ping)]
struct General;

struct Handler {
    save_data: Container,
}

#[async_trait]
impl EventHandler for Handler {

    async fn ready(&self, ctx: Context, ready: Ready) {
        // Log at the INFO level. This is a macro from the `tracing` crate.
        println!("{} is connected!", ready.user.name);
        info!("{} is connected!", ready.user.name);

        for (k, v) in &self.save_data.messages {
            println!("Key: {}", k);
            println!("Value: {}", serde_json::to_string(&v).expect("Couldn't json the value!"));
        }

        for guild_info in ctx.http.get_guilds(None, None).await.expect("Get guilds error!") {
            println!("Guild id: {}", guild_info.id);

            let partial_guild = ctx.http.get_guild(guild_info.id.0).await.expect("Get guild error!");

            let guild = Guild {
                id: partial_guild.id.0,
                emojis: partial_guild.emojis.values().cloned().collect::<Vec<Emoji>>(),
                roles: partial_guild.roles.values().cloned().collect::<Vec<Role>>(),
            };
            
            // TODO: Figure out how to use save_data, maybe as a reference
            // Because self is NOT mutable sadly
            &self.save_data.guilds.insert(guild.id.to_string(), Guild {
                id: partial_guild.id.0,
                emojis: partial_guild.emojis.values().cloned().collect::<Vec<Emoji>>(),
                roles: partial_guild.roles.values().cloned().collect::<Vec<Role>>(),
            });
        }
    }

}

#[derive(Serialize, Deserialize)]
struct Container {
    guilds: HashMap<String, Guild>,
    messages: HashMap<String, MessageActions>,
}

#[derive(Serialize, Deserialize)]
struct Guild {
    id: u64,
    emojis: Vec<Emoji>,
    roles: Vec<Role>,
}

#[derive(Serialize, Deserialize)]
struct MessageActions {
    id: u64,
    roles: Vec<EmojiRoleMapping>,
}

#[derive(Serialize, Deserialize)]
struct EmojiRoleMapping {
    emoji: u64,
    role: u64,
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt::init();

    dotenv().ok();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    let json_data = fs::read_to_string("save_data.json").expect("Couldn't read the file!");
    let container: Container = serde_json::from_str(&json_data).expect("Couldn't json the file!");

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
        
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler {save_data: container})
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
    println!("The bot started");
}

fn create_option(label: &str, value: &str) -> CreateSelectMenuOption {
    let mut smo = CreateSelectMenuOption::default();
    smo.label(label);
    smo.value(value);
    smo
}

fn create_select_menu() -> CreateSelectMenu {
    let mut sm = CreateSelectMenu::default();
    sm.custom_id("Custom select menu id!");
    sm.placeholder("Placeholder!");
    sm.options(|o| o.add_option(create_option("Label", "Value")));
    sm
}

fn create_action_row() -> CreateActionRow {
    let mut ar = CreateActionRow::default();
    ar.add_select_menu(create_select_menu());
    ar
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    // msg.reply(ctx, "Pong!").await?;
    msg.delete(ctx).await;

    let m = msg.channel_id.send_message(&ctx, |m| {
        m.content("Pong! :O")
            .components(|c| c.add_action_row(create_action_row()))
    })
    .await
    .unwrap();

    let result =
        match m.await_component_interaction(&ctx).timeout(Duration::from_secs(60 * 3)).await {
            Some(res) => res,
            None => {
                m.reply(&ctx, "Timed out!").await.unwrap();
                return Ok(());
            },
        };

    m.reply(&ctx, format!("You selected: {}", result.data.values.get(0).unwrap())).await.unwrap();

    // TODO: Create a message with reactions: https://docs.rs/serenity/0.11.5/serenity/builder/struct.CreateMessage.html#method.reactions
    // TODO: Add `reaction_add` and `reaction_remove` methods into the EventHandler: https://docs.rs/serenity/0.11.5/serenity/prelude/trait.EventHandler.html#method.reaction_add
    // TODO: Map the message id of the created message so you can know which message we need to listen to!


    /*msg.channel_id.send_message(&ctx, |m| {
        m.content("Pong! :O")
        .components(|c| c.add_action_row(|ar: CreateActionRow| { ar.add_select_menu(|sm: CreateSelectMenu| {
            sm.custom_id("custom select menu")
                .placeholder("Placeholder!")
                .options(|os: CreateSelectMenuOptions| {
                    os.add_option(|o| o.label("label").value("value"))
                })

        }).build() }))
    }).await?;*/

    // Delete the orig message or there will be dangling components
    m.delete(&ctx).await.unwrap();

    return Ok(())
}


