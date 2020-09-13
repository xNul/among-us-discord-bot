use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::gateway::{Activity, Ready};
use serenity::framework::standard::{
    StandardFramework,
    CommandResult,
    macros::{
        command,
        group
    }
};
use serenity::prelude::TypeMapKey;
use serenity::utils::parse_username;

mod config;

struct DeadList;

impl TypeMapKey for DeadList {
    type Value = Vec<u64>;
}

struct CommandUnmuteall;

impl TypeMapKey for CommandUnmuteall {
    type Value = bool;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        ctx.set_activity(Activity::playing("Among Us")).await;
    }
}

#[group]
#[commands(ping, muteall, unmuteall, kill, revive, reset)]
struct General;

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    let mut client = Client::new(config::TOKEN)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<DeadList>(vec![]);
        data.insert::<CommandUnmuteall>(true);
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}

#[command]
async fn muteall(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;
    
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();
    let voice_channel = guild.channels.get(&voice_channel_id).unwrap();
    let voice_channel_members = voice_channel.members(&ctx.cache).await.unwrap();

    for member in voice_channel_members.iter() {
        member.edit(&ctx.http, |em| em.mute(true)).await.unwrap();
    }

    let mut data = ctx.data.write().await;
    let unmuteall = data.get_mut::<CommandUnmuteall>().expect("Expected CommandUnmuteall in TypeMap.");
    *unmuteall = false;

    Ok(())
}

#[command]
async fn unmuteall(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();
    let voice_channel = guild.channels.get(&voice_channel_id).unwrap();
    let voice_channel_members = voice_channel.members(&ctx.cache).await.unwrap();

    let data = ctx.data.read().await;
    let dead = data.get::<DeadList>().unwrap();

    for member in voice_channel_members.iter() {
        let user_id = member.user.id.0;
        if dead.iter().position(|&u| u == user_id).is_none() {
            member.edit(&ctx.http, |em| em.mute(false)).await.unwrap();
        }
    }

    drop(data);
    let mut data = ctx.data.write().await;
    let unmuteall = data.get_mut::<CommandUnmuteall>().expect("Expected CommandUnmuteall in TypeMap.");
    *unmuteall = true;

    Ok(())
}

#[command]
async fn kill(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;

    let unparsed_user_id = msg.content.as_str().split(" ").nth(1).unwrap();
    let user_id = parse_username(unparsed_user_id).unwrap();
    let mut data = ctx.data.write().await;
    let dead = data.get_mut::<DeadList>().expect("Expected DeadList in TypeMap.");
    
    match dead.iter().position(|&u| u == user_id) {
        Some(_) => { println!("{} has already been killed", user_id); },
        None => {
            dead.push(user_id);
            let guild = msg.guild(&ctx.cache).await.unwrap();
            let member = guild.member(&ctx.http, user_id).await.unwrap();
            member.edit(&ctx.http, |em| em.mute(true)).await.unwrap();
        },
    }

    Ok(())
}

#[command]
async fn revive(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;

    let unparsed_user_id = msg.content.as_str().split(" ").nth(1).unwrap();
    let user_id = parse_username(unparsed_user_id).unwrap();
    let mut data = ctx.data.write().await;
    let dead = data.get_mut::<DeadList>().expect("Expected DeadList in TypeMap.");
    
    match dead.iter().position(|&u| u == user_id) {
        Some(index) => { dead.remove(index); },
        None => { println!("{} is already alive", user_id); },
    }

    drop(data);
    let data = ctx.data.read().await;
    let unmuteall = data.get::<CommandUnmuteall>().unwrap();

    if *unmuteall {
        let guild = msg.guild(&ctx.cache).await.unwrap();
        let member = guild.member(&ctx.http, user_id).await.unwrap();
        member.edit(&ctx.http, |em| em.mute(false)).await.unwrap();
    }

    Ok(())
}

#[command]
async fn reset(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;

    let data = ctx.data.read().await;
    let unmuteall = data.get::<CommandUnmuteall>().unwrap();
    let unmuteall_stat = *unmuteall;

    drop(data);
    let mut data = ctx.data.write().await;
    let dead = data.get_mut::<DeadList>().expect("Expected DeadList in TypeMap.");

    if unmuteall_stat {
        let guild = msg.guild(&ctx.cache).await.unwrap();

        for &user_id in dead.iter() {
            let member = guild.member(&ctx.http, user_id).await.unwrap();
            member.edit(&ctx.http, |em| em.mute(false)).await.unwrap();
        }
    }

    *dead = vec![];

    Ok(())
}
