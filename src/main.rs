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

mod config;

#[group]
#[commands(ping, muteall, unmuteall, kill, revive, reset)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        ctx.set_activity(Activity::playing("Among Us")).await;
    }
}

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

    for member in voice_channel_members.iter() {
        member.edit(&ctx.http, |em| em.mute(false)).await.unwrap();
    }

    Ok(())
}

#[command]
async fn kill(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;

    Ok(())
}

#[command]
async fn revive(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;

    Ok(())
}

#[command]
async fn reset(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, msg.content.as_str()).await?;

    Ok(())
}
