use std::env;
use std::fs;
use dotenv::dotenv;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Mutex};
use lazy_static::lazy_static;

use serde::{Serialize, Deserialize};

use serenity::async_trait;
use serenity::prelude::*;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult};
use serenity::model::gateway::Ready;
use serenity::model::prelude::Message;
use serenity::model::guild::{Emoji, Role};
use serenity::model::channel::ReactionType::{Custom, Unicode};
use serenity::model::id::EmojiId;
use serenity::builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption};
use tracing::{info};
// use tracing_subscriber::FmtSubscriber;

lazy_static! {
    static ref BOT_DATA: Mutex<Container> = Mutex::new(Container::new("save_data.json")/*Container {
        guilds: HashMap::new(),
        messages: HashMap::new(),
    }*/);
}

#[group]
#[commands(ping, selector)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    
    async fn ready(&self, ctx: Context, ready: Ready) {
        // Log at the INFO level. This is a macro from the `tracing` crate.
        println!("{} is connected!", ready.user.name);
        info!("{} is connected!", ready.user.name);

        {
            let container = BOT_DATA.lock().unwrap();
            for (k, v) in &container.messages {
                println!("Key: {}", k);
                println!("Value: {}", serde_json::to_string(&v).expect("Couldn't json the value!"));
            }
        }
        
        for guild_info in ctx.http.get_guilds(None, None).await.expect("Get guilds error!") {
            println!("Guild id: {}", guild_info.id);

            let partial_guild = ctx.http.get_guild(guild_info.id.0).await.expect("Get guild error!");

            let guild = Guild {
                id: partial_guild.id.0,
                emojis: partial_guild.emojis.values().cloned().collect::<Vec<Emoji>>(),
                roles: partial_guild.roles.values().cloned().collect::<Vec<Role>>(),
            };
            
            let mut container = BOT_DATA.lock().unwrap();

            // TODO: Figure out how to use save_data, maybe as a reference
            // Because self is NOT mutable sadly
            container.guilds.insert(guild.id.to_string(), Guild {
                id: partial_guild.id.0,
                emojis: partial_guild.emojis.values().cloned().collect::<Vec<Emoji>>(),
                roles: partial_guild.roles.values().cloned().collect::<Vec<Role>>(),
            });
        }

        {
            let container = BOT_DATA.lock().unwrap();

            for (k, v) in &container.guilds {
                println!("Key: {}", k);
                println!("Value: {}", serde_json::to_string(&v).expect("Couldn't json the value!"));
            }
        }
    }

}

#[derive(Serialize, Deserialize, Clone)]
struct Container {
    path: Option<String>,
    guilds: HashMap<String, Guild>,
    messages: HashMap<String, MessageActions>,
}

impl Container {
    fn new(path: &str) -> Container {
        let json_data = fs::read_to_string(path).expect("Couldn't read the file!");
        let mut container: Container = serde_json::from_str(&json_data).expect("Couldn't json the file!");
        container.path = Some(path.to_string());

        return container;
    }

    fn save(&self) {
        fs::write(&self.path.as_ref().expect("No path on container!"), serde_json::to_string(self).expect("Couldn't turn Container into json!"));
    }

    fn get_guild_role_by_name<S: AsRef<str>>(&self, guild_id: S, role_name: S) -> Option<&Role> {
        match self.guilds.get(guild_id.as_ref()) {
            None => None,
            Some(guild) => guild.get_role_by_name(role_name),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Guild {
    id: u64,
    emojis: Vec<Emoji>,
    roles: Vec<Role>,
}

impl Guild {
    fn get_role_by_name<S: AsRef<str>>(&self, role_name: S) -> Option<&Role> {
        let name = role_name.as_ref();
        for role in &self.roles {
            if role.name.eq(name) { return Some(role); }
        }
        return None;
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct MessageActions {
    id: u64,
    roles: Vec<EmojiRoleMapping>,
}

#[derive(Serialize, Deserialize, Clone)]
struct EmojiRoleMapping {
    emoji: String,
    role: u64,
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt::init();

    dotenv().ok();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    
    /*let mut c = BOT_DATA.lock().unwrap();
    c.guilds = container.guilds.clone();
    c.messages = container.messages.clone();*/

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
        
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
    println!("The bot started");
}

#[command]
async fn roles(ctx: &Context, msg: &Message) -> CommandResult {
    // ~selector roles (:some_emoji_idk: | role name) (:some_other_emoji: | other role name)
    println!("Roles: {}", msg.content);

    let content = msg.content.replace("~selector roles", "").trim().to_string();
    if content.is_empty() {
        msg.reply(&ctx, "Usage: ~selector roles (:some_emoji_idk: | role name) (:some_other_emoji: | other role name)").await?;
        return Ok(());
    }
    let container = BOT_DATA.lock().unwrap().clone();
    let mut message_actions = MessageActions {id: 0, roles: Vec::new()};
    let mut message_parts: Vec<String> = Vec::new();

    let items: Vec<&str> = content.split(") (").collect(); //.iter().map(|s| s.split("|"));
    for mut item in items {
        item = item.trim();

        let string_item: String = item.to_string().replace("(", "").replace(")", "")
            .trim().to_string();
        /*if item.starts_with("(") && item.ends_with(")") {
            println!("Option 1! take: '{}'", item.len());
            string_item = item.chars().skip(1).take(item.len() - 3).collect()
        } else if item.ends_with(")") {
            println!("Option 2!");
            string_item = item.chars().take(item.len() - 1).collect()
        } else if item.starts_with("(") {
            println!("Option 3!");
            string_item = item.chars().skip(1).collect();
        } else {
            println!("Option 4!");
            string_item = item.chars().collect();
        }
        string_item = string_item.trim().to_string();*/
        println!("Item: {}", item);
        println!("String item: {}", string_item);

        let parts: Vec<&str> = string_item.split("|").collect();
        let (emoji, role) = (parts[0].trim(), parts[1].trim());

        // msg.channel_id.send_message(ctx, "").await.expect("Expected the message!");
        println!("Role: '{}'", role);
        let role_id: u64 = match container.get_guild_role_by_name(msg.guild_id.unwrap().0.to_string(), role.to_string()) {
            None => {
                msg.reply(&ctx, "No role with that name!").await.unwrap();
                return Ok(());
            },
            Some(role) => role.id.0,
        };
        message_actions.roles.push(EmojiRoleMapping {
            role: role_id,
            emoji: emoji.to_string(),
        });

        // <@&739501493639577653>
        message_parts.push(format!("You can get <@&{}> if you react with {}", role_id, emoji));
    }

    println!("message_actions: {}", serde_json::to_string(&message_actions).expect("Couldn't json the value!"));

    let message = msg.channel_id.say(&ctx, message_parts.join("\n")).await.unwrap();
    for mapping in &message_actions.roles {
        let emoji = &mapping.emoji;
        println!("Emoji: {}", emoji);
        if emoji.starts_with("<") {
            let parts: &Vec<&str> = &mapping.emoji.split(":").collect();
            let name = parts[1].to_string();
            let id = parts[2].replace(">", "").to_string().parse::<u64>().unwrap();
            println!("Emoji Id: {}", id);
            message.react(&ctx, Custom{animated: false, id: EmojiId(id), name: Some(name)}).await.expect("Error react!");
        } else {
            message.react(&ctx, Unicode(emoji.to_string())).await.unwrap();
        }
    }
    {
        let id: u64 = message.id.0;
        message_actions.id = id;
        let mut container = BOT_DATA.lock().unwrap();
        container.messages.insert(id.to_string(), message_actions);
        container.save();
    }

    msg.reply(&ctx, "Something").await?;

    Ok(())
}

#[command]
#[sub_commands(roles)]
async fn selector(ctx: &Context, msg: &Message) -> CommandResult {
    // ~selector
    msg.reply(&ctx, r#"Usage:
        - selector roles (:some_emoji_idk: | role name) (:some_other_emoji: | other role name)
    "#).await?;
    println!("selector {}", msg.content);

    Ok(())
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


