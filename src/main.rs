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
use serenity::model::channel::{Reaction, ReactionType};
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
    
    async fn guild_role_create(&self, _ctx: Context, role: Role) {
        let mut container = BOT_DATA.lock().unwrap();
        let guild_id = role.guild_id.0.to_string();
        let guild = match container.guilds.get_mut(&guild_id) {
            Some(guild) => guild,
            None => return,
        };
        guild.roles.push(role.clone());
        container.save();
    }

    async fn guild_role_update(&self, _ctx: Context, _old_data_if_available: Option<Role>, role: Role) {
        let mut container = BOT_DATA.lock().unwrap();
        let guild_id = role.guild_id.0.to_string();
        let guild = match container.guilds.get_mut(&guild_id) {
            Some(guild) => guild,
            None => return,
        };
        
        for index in 0..guild.roles.len() {
            if role.id.0 == guild.roles[index].id.0 {
                guild.roles.remove(index);
                break;
            }
        }
        guild.roles.push(role);
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let message_id = reaction.message_id.0.to_string();
        let message_action = match BOT_DATA.lock().unwrap().messages.get(&message_id) {
            Some(message_action) => message_action.clone(),
            None => return,
        };

        let mut member = reaction.guild_id.expect("Couldn't get guild")
            .member(&ctx, reaction.user_id.expect("Couldn't get user"))
            .await.expect("Couldn't get member");
        match message_action.get_role_from_emoji(reaction.emoji) {
            Some(role_id) => member.add_role(&ctx, role_id).await.expect("Couldn't give the role to the user!"),
            None => return,
        };
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        let message_id = reaction.message_id.0.to_string();
        let message_action = match BOT_DATA.lock().unwrap().messages.get(&message_id) {
            Some(message_action) => message_action.clone(),
            None => return,
        };

        let mut member = reaction.guild_id.expect("Couldn't get guild")
            .member(&ctx, reaction.user_id.expect("Couldn't get user"))
            .await.expect("Couldn't get member");
        match message_action.get_role_from_emoji(reaction.emoji) {
            Some(role_id) => member.remove_role(&ctx, role_id).await.expect("Couldn't take the role from the user!"),
            None => return,
        };
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        info!("{} is connected!", ready.user.name);

        for guild_info in ctx.http.get_guilds(None, None).await.expect("Get guilds error!") {
            let partial_guild = ctx.http.get_guild(guild_info.id.0).await.expect("Get guild error!");

            let guild = Guild {
                id: partial_guild.id.0,
                emojis: partial_guild.emojis.values().cloned().collect::<Vec<Emoji>>(),
                roles: partial_guild.roles.values().cloned().collect::<Vec<Role>>(),
            };
            
            let mut container = BOT_DATA.lock().unwrap();

            container.guilds.insert(guild.id.to_string(), Guild {
                id: partial_guild.id.0,
                emojis: partial_guild.emojis.values().cloned().collect::<Vec<Emoji>>(),
                roles: partial_guild.roles.values().cloned().collect::<Vec<Role>>(),
            });
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
        fs::write(&self.path.as_ref().expect("No path on container!"), serde_json::to_string(self).expect("Couldn't turn Container into json!")).expect("Couldn't save container to file!");
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

impl MessageActions {
    fn get_role_from_emoji(&self, reaction: ReactionType) -> Option<u64> {
        let emoji = match reaction {
            Custom {name, id, animated: _} => format!("<:{}:{}>", name.expect("Name of Custom emoji expected!"), id.0),
            Unicode(emoji) => emoji,
            _ => return None,
        };
        for mapping in &self.roles {
            if mapping.emoji.eq(&emoji) { return Some(mapping.role); }
        }
        return None;
    }
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
        
    let intents = GatewayIntents::all();
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

        let parts: Vec<&str> = string_item.split("|").collect();
        let (emoji, role) = (parts[0].trim(), parts[1].trim());

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

        message_parts.push(format!("You can get <@&{}> if you react with {}", role_id, emoji));
    }

    let message = msg.channel_id.say(&ctx, message_parts.join("\n")).await.unwrap();
    for mapping in &message_actions.roles {
        let emoji = &mapping.emoji;
        if emoji.starts_with("<") {
            let parts: &Vec<&str> = &mapping.emoji.split(":").collect();
            let name = parts[1].to_string();
            let id = parts[2].replace(">", "").to_string().parse::<u64>().unwrap();
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
    msg.delete(ctx).await?;

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


