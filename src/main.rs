use std::collections::HashMap;
use std::panic;
use std::io::Write;
use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;

use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    model::{
        channel::Message,
        gateway::{Activity, Ready},
        voice::VoiceState,
        id::{GuildId, ChannelId, UserId}
    },
    framework::standard::{
        StandardFramework,
        CommandResult,
        CommandError,
        macros::{
            command,
            group,
            hook
        }
    },
    prelude::TypeMapKey,
    utils::parse_username
};

mod config;
mod among_us;
use among_us::*;

impl TypeMapKey for Games {
    type Value = HashMap<u64, GameInstance>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        ctx.set_activity(Activity::playing("Among Us")).await;
    }

    async fn voice_state_update(&self, _ctx: Context, _gid: Option<GuildId>, _old: Option<VoiceState>, _new: VoiceState) {
        let guild_id = _gid.unwrap();
        let is_leaving = _old.is_some() && _new.channel_id.is_none();
        let is_joining = _old.is_none() && _new.channel_id.is_some();

        if is_leaving {
            let voice_state = _old.unwrap();
            let voice_channel_id = voice_state.channel_id.unwrap();
            let user_id = voice_state.user_id.0;

            // If a game existed for the VC.
            let mut data = _ctx.data.write().await;
            let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
            if let Some(game_instance) = games.get_mut(&voice_channel_id.0) {
                // If leader leaves, free leader position for the VC.
                if game_instance.leader_user_id == user_id {
                    game_instance.leader_user_id = 0;
                    ChannelId(game_instance.recent_text_channel_id).say(&_ctx.http, "The Leader has stepped down. No Leader active.").await.unwrap();
                }

                // Remove player from dead players, if exists.
                if game_instance.dead_players.contains_key(&user_id) {
                    game_instance.dead_players.remove(&user_id);
                }
                
                // If there are no players left in the VC, delete the game.
                let guild = _ctx.cache.guild(guild_id).await.unwrap();
                let voice_channel = guild.channels.get(&voice_channel_id).unwrap();
                let voice_channel_members = voice_channel.members(&_ctx.cache).await.unwrap();
                if voice_channel_members.len() == 0 {
                    ChannelId(game_instance.recent_text_channel_id).say(&_ctx.http, "No Players are left in the Voice Channel. Game Instance deleted.").await.unwrap();
                    games.remove(&voice_channel_id.0);
                }
            }
        } else if is_joining {
            let voice_state = _new;
            let voice_channel_id = voice_state.channel_id.unwrap();
            let user_id = voice_state.user_id.0;

            let mut game_muted = false;

            // If a game existed for the VC, check if it's muted.
            let mut data = _ctx.data.write().await;
            let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
            if let Some(game_instance) = games.get_mut(&voice_channel_id.0) {
                if !game_instance.global_unmute {
                    game_muted = true;
                }
            }

            // If muted, unmute person unless there is a game going on that is muted.
            let member = _ctx.cache.member(guild_id, user_id).await.unwrap();
            if voice_state.mute && !game_muted {
                member.edit(&_ctx.http, |em| em.mute(false)).await.unwrap();
            }
        }
    }
}

#[hook]
async fn before_hook(ctx: &Context, msg: &Message, cmd_name: &str) -> bool {
    log::info!("Command \"{}\" sent by \"{}\" in \"{}\"", msg.content, msg.author.tag(), msg.guild_id.unwrap());

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let user_id = msg.author.id.0;
    
    // If user is in a VC and not init, then init and make leader.
    // If user is in a VC and init and leader, run command.
    // If user is in a VC and init and not leader, make leader.
    match voice_states.get(&msg.author.id) {
        Some(voice_state) => {
            let voice_channel_id = voice_state.channel_id.unwrap();
            let mut data = ctx.data.write().await;
            let games = data.get_mut::<Games>().unwrap();
            
            match games.get_mut(&voice_channel_id.0) {
                Some(game_instance) => {
                    match game_instance.leader_user_id {
                        u if u == user_id => {
                            game_instance.recent_text_channel_id = msg.channel_id.0;
                            true
                        },
                        0 => {
                            game_instance.leader_user_id = user_id;
                            game_instance.recent_text_channel_id = msg.channel_id.0;
                            msg.channel_id.say(&ctx.http, "Congratulations, you \
                                are now the Leader of this Game Instance! Only you can mute other \
                                players. To step down, disconnect from the Voice Channel.").await.unwrap();
                            
                            true
                        },
                        _ => {
                            msg.channel_id.say(&ctx.http, "Access denied. Your Game Instance \
                                already has a Leader.").await.unwrap();
                            
                            false
                        }
                    }
                },
                None => {
                    let new_game = GameInstance{
                        leader_user_id: user_id,
                        recent_text_channel_id: msg.channel_id.0,
                        global_unmute: true,
                        dead_players: HashMap::new()
                    };
                    games.insert(voice_channel_id.0, new_game);
                    msg.channel_id.say(&ctx.http, "Created a new Game Instance.").await.unwrap();
                    msg.channel_id.say(&ctx.http, "Congratulations, you \
                        are now the Leader of this Game Instance! Only you can mute \
                        other players. To step down, disconnect from the Voice Channel.")
                        .await.unwrap();
                    
                    true
                }
            }
        },
        None => { 
            if cmd_name != "help" {
                msg.channel_id.say(&ctx.http, "Please enter Voice Chat before \
                    using Game commands.").await.unwrap();
                
                false
            } else {
                true
            }
        }
    }
}

#[hook]
async fn after_hook(ctx: &Context, msg: &Message, _: &str, error: Result<(), CommandError>) {
    if let Err(why) = error {
        msg.channel_id.say(&ctx.http, format!("```Error: {}```", why)).await.unwrap();
        log::warn!("Command \"{}\" sent by \"{}\" in \"{}\" failed with error \"{}\"", msg.content, msg.author.tag(), msg.guild_id.unwrap(), why);
    }
}

#[hook]
async fn unrecognised_command_hook(ctx: &Context, msg: &Message, _: &str) {
    msg.channel_id.say(&ctx.http, "```Error: Unknown command. Use '!help' for more information.```").await.unwrap();
    log::warn!("Command \"{}\" sent by \"{}\" in \"{}\" failed with error \"{}\"", msg.content, msg.author.tag(), msg.guild_id.unwrap(), "Unknown command. Use '!help' for more information.");
}

#[group]
#[commands(help, play, discuss, kill, revive, reset)]
struct General;

#[tokio::main]
async fn main() {
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
    
    panic::set_hook(Box::new(|panic_info| {
        log::error!("{}", panic_info);
    }));
    
    let framework = StandardFramework::new()
        .before(before_hook)
        .after(after_hook)
        .unrecognised_command(unrecognised_command_hook)
        .configure(|c| c.prefix("!"))
        .group(&GENERAL_GROUP);

    let mut client = Client::new(config::TOKEN)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<Games>(HashMap::new());
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn play(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();
    let voice_channel = guild.channels.get(&voice_channel_id).unwrap();
    let voice_channel_members = voice_channel.members(&ctx.cache).await?;

    for member in voice_channel_members.iter() {
        member.edit(&ctx.http, |em| em.mute(true)).await?;
    }

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().unwrap();
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    game_instance.global_unmute = false;

    msg.channel_id.say(&ctx.http, "All Players have been muted.").await?;

    Ok(())
}

#[command]
async fn discuss(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();
    let voice_channel = guild.channels.get(&voice_channel_id).unwrap();
    let voice_channel_members = voice_channel.members(&ctx.cache).await?;

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().unwrap();
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &game_instance.dead_players;

    for member in voice_channel_members.iter() {
        let user_id = member.user.id.0;
        let dead_player = dead_players.get(&user_id);
        if dead_player.is_none() {
            member.edit(&ctx.http, |em| em.mute(false)).await?;
        }
    }

    game_instance.global_unmute = true;

    msg.channel_id.say(&ctx.http, "All Players have been unmuted except for those Killed.").await?;

    Ok(())
}

#[command]
async fn kill(ctx: &Context, msg: &Message) -> CommandResult {
    let unparsed_user_id = msg.content.as_str().split(" ").nth(1)
        .ok_or("No Player to Kill. Mention the Player with '!kill @Player'.")?;
    let user_id = parse_username(unparsed_user_id)
        .ok_or("Could not parse User ID. Is it valid? Mention the Player with '!kill @Player'.")?;
    let user = UserId(user_id).to_user(ctx).await?;
    let name = match user.nick_in(ctx, msg.guild_id.unwrap()).await {
        Some(nick) => nick,
        None => user.name
    };
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().unwrap();
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &mut game_instance.dead_players;
    
    match dead_players.get(&user_id) {
        Some(_) => {
            msg.channel_id.say(&ctx.http, format!("{} has already been Killed.", name)).await?;
        },
        None => {
            dead_players.insert(user_id, true);
            let guild = msg.guild(&ctx.cache).await.unwrap();
            let member = guild.member(&ctx.http, user_id).await?;
            member.edit(&ctx.http, |em| em.mute(true)).await?;

            msg.channel_id.say(&ctx.http, format!("{} has been Killed.", name)).await?;
        },
    }

    Ok(())
}

#[command]
async fn revive(ctx: &Context, msg: &Message) -> CommandResult {
    let unparsed_user_id = msg.content.as_str().split(" ").nth(1)
        .ok_or("No Player to Revive. Mention the Player with '!revive @Player'.")?;
    let user_id = parse_username(unparsed_user_id)
        .ok_or("Could not parse User ID. Is it valid? Mention the Player with '!revive @Player'.")?;
    let user = UserId(user_id).to_user(ctx).await?;
    let name = match user.nick_in(ctx, msg.guild_id.unwrap()).await {
        Some(nick) => nick,
        None => user.name
    };
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().unwrap();
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &mut game_instance.dead_players;
    
    match dead_players.get(&user_id) {
        Some(_) => { 
            dead_players.remove(&user_id);
            if game_instance.global_unmute {
                let guild = msg.guild(&ctx.cache).await.unwrap();
                let member = guild.member(&ctx.http, user_id).await?;
                member.edit(&ctx.http, |em| em.mute(false)).await?;
            }
            msg.channel_id.say(&ctx.http, format!("{} has been Revived.", name)).await?;
        },
        None => {
            msg.channel_id.say(&ctx.http, format!("{} has not been Killed.", name)).await?;
        },
    }

    Ok(())
}

#[command]
async fn reset(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().unwrap();
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &game_instance.dead_players;

    if game_instance.global_unmute {
        let guild = msg.guild(&ctx.cache).await.unwrap();

        for &user_id in dead_players.keys() {
            let member = guild.member(&ctx.http, user_id).await?;
            member.edit(&ctx.http, |em| em.mute(false)).await?;
        }
    }

    game_instance.dead_players = HashMap::new();

    msg.channel_id.say(&ctx.http, "Those Killed have been Revived.").await?;

    Ok(())
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "⁣\n**Info**\n\
        The AmongUsBot can only be used from within a Voice Channel. The first person \
        to type a command while within a Voice Channel, will create a Game Instance for \
        that Voice Channel and become the Leader of that Voice Channel's Game Instance. \
        The Leader controls all muting within the channel. To step down as Leader, the \
        Leader must disconnect from the Voice Channel.\nMute status *is not* permanent. \
        As soon as someone connects to another Voice Channel, mute status disappears.\nThere \
        can only be one Game Instance in a Voice Channel. Commands for one will not affect \
        another. Multiple games can be played *independently and simultaneously* in a server.\n\n\
        **Commands**\n\
        `!play` - Mutes all Players in the Voice Chat\n\
        `!discuss` - Unmutes all Players in the Voice Chat *except* for those which are Killed\n\
        `!kill <@Player>` - Kills or Mutes the given Player regardless of Unmute\n\
        `!revive <@Player>` - Revives or Unmutes a Killed player\n\
        `!reset` - Revives all Killed Players\n\n\
        **Credit**\n\
        Developed by nabakin.\n\nIf you like AmongUsBot, please star it on GitHub. If you \
        want AmongUsBot in your server, an invite link can be found on GitHub as well. Thanks \
        for using my bot! https://github.com/nabakin/among-us-discord-bot")
        .await?;

    Ok(())
}