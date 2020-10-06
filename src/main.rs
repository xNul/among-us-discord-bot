use std::collections::HashMap;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::{
    channel::Message,
    gateway::{Activity, Ready},
    voice::VoiceState,
    id::{GuildId, ChannelId, UserId}
};
use serenity::framework::standard::{
    StandardFramework,
    CommandResult,
    macros::{
        command,
        group,
        hook
    }
};
use serenity::prelude::TypeMapKey;
use serenity::utils::parse_username;

mod config;

struct Games;

#[derive(Debug)]
struct GameInstance {
    leader_user_id: u64,
    recent_text_channel_id: u64,
    global_unmute: bool,
    dead_players: HashMap<u64, bool>
}

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
                    ChannelId(game_instance.recent_text_channel_id).say(_ctx.http, "The Leader has stepped down. No Leader active.").await.unwrap();
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
    let guild = msg.guild(&ctx.cache).await.unwrap(); // bug here sometimes
    let voice_states = guild.voice_states;
    let user_id = msg.author.id.0;
    
    // If user is in a VC and not init, then init and make leader.
    // If user is in a VC and init and leader, run command.
    // If user is in a VC and init and not leader, make leader.
    match voice_states.get(&msg.author.id) {
        Some(voice_state) => {
            let voice_channel_id = voice_state.channel_id.unwrap();
            let mut data = ctx.data.write().await;
            let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
            
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
                            msg.channel_id.say(&ctx.http, "Congratulations, you are now the Leader of this Voice Channel. Only you can mute other players. To step down, disconnect from the Voice Channel.").await.unwrap();
                            true
                        },
                        _ => {
                            msg.channel_id.say(&ctx.http, "Access denied. Your Voice Channel already has a leader.").await.unwrap();
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
                    msg.channel_id.say(&ctx.http, "Congratulations, you are now the leader of this Voice Channel. Only you can mute other players. To step down, disconnect from the Voice Channel.").await.unwrap();
                    true
                }
            }
        },
        None => { 
            if cmd_name != "help" {
                msg.channel_id.say(&ctx.http, "Please enter Voice Chat before using Game commands.").await.unwrap();
                false
            } else {
                true
            }
        }
    }
}

#[group]
#[commands(ping, help, muteall, unmuteall, kill, revive, reset)]
struct General;

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .before(before_hook)
        .configure(|c| c.prefix("!")) // set the bot's prefix to "~"
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
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;

    Ok(())
}

#[command]
async fn muteall(ctx: &Context, msg: &Message) -> CommandResult {
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
    let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    game_instance.global_unmute = false;

    msg.channel_id.say(&ctx.http, "All players have been muted.").await?;

    Ok(())
}

#[command]
async fn unmuteall(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();
    let voice_channel = guild.channels.get(&voice_channel_id).unwrap();
    let voice_channel_members = voice_channel.members(&ctx.cache).await.unwrap();

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &game_instance.dead_players;

    for member in voice_channel_members.iter() {
        let user_id = member.user.id.0;
        let dead_player = dead_players.get(&user_id);
        if dead_player.is_none() {
            member.edit(&ctx.http, |em| em.mute(false)).await.unwrap();
        }
    }

    game_instance.global_unmute = true;

    msg.channel_id.say(&ctx.http, "All players have been unmuted except for those killed.").await?;

    Ok(())
}

#[command]
async fn kill(ctx: &Context, msg: &Message) -> CommandResult {
    let unparsed_user_id = msg.content.as_str().split(" ").nth(1).unwrap();
    let user_id = parse_username(unparsed_user_id).unwrap(); // bug here sometimes
    let user = UserId(user_id).to_user(ctx).await.unwrap();
    let name = match user.nick_in(ctx, msg.guild_id.unwrap()).await {
        Some(nick) => nick,
        None => user.name
    };
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &mut game_instance.dead_players;
    
    match dead_players.get(&user_id) {
        Some(_) => {
            msg.channel_id.say(&ctx.http, format!("{} has already been killed.", name)).await?;
        },
        None => {
            dead_players.insert(user_id, true);
            let guild = msg.guild(&ctx.cache).await.unwrap();
            let member = guild.member(&ctx.http, user_id).await.unwrap();
            member.edit(&ctx.http, |em| em.mute(true)).await.unwrap();

            msg.channel_id.say(&ctx.http, format!("{} has been killed.", name)).await?;
        },
    }

    Ok(())
}

#[command]
async fn revive(ctx: &Context, msg: &Message) -> CommandResult {
    let unparsed_user_id = msg.content.as_str().split(" ").nth(1).unwrap();
    let user_id = parse_username(unparsed_user_id).unwrap();
    let user = UserId(user_id).to_user(ctx).await.unwrap();
    let name = match user.nick_in(ctx, msg.guild_id.unwrap()).await {
        Some(nick) => nick,
        None => user.name
    };
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let voice_states = guild.voice_states;
    let voice_state = voice_states.get(&msg.author.id).unwrap();
    let voice_channel_id = voice_state.channel_id.unwrap();

    let mut data = ctx.data.write().await;
    let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &mut game_instance.dead_players;
    
    match dead_players.get(&user_id) {
        Some(_) => { 
            dead_players.remove(&user_id);
            if game_instance.global_unmute {
                let guild = msg.guild(&ctx.cache).await.unwrap();
                let member = guild.member(&ctx.http, user_id).await.unwrap();
                member.edit(&ctx.http, |em| em.mute(false)).await.unwrap();
            }
            msg.channel_id.say(&ctx.http, format!("{} has been revived.", name)).await?;
        },
        None => {
            msg.channel_id.say(&ctx.http, format!("{} is already alive.", name)).await?;
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
    let games = data.get_mut::<Games>().expect("Expected Games in TypeMap.");
    let game_instance = games.get_mut(&voice_channel_id.0).unwrap();
    let dead_players = &game_instance.dead_players;

    if game_instance.global_unmute {
        let guild = msg.guild(&ctx.cache).await.unwrap();

        for &user_id in dead_players.keys() {
            let member = guild.member(&ctx.http, user_id).await.unwrap();
            member.edit(&ctx.http, |em| em.mute(false)).await.unwrap();
        }
    }

    game_instance.dead_players = HashMap::new();

    msg.channel_id.say(&ctx.http, "The dead have been revived.").await?;

    Ok(())
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "⁣\n**AmongUsBot Info**\n\
        The AmongUsBot can only be used from within a Voice Channel. The first person \
        to type a command while within a Voice Channel, will become the Leader of that \
        Voice Channel. The Leader controls all muting within the channel. To step down \
        as Leader, the Leader must reconnect to the Voice Channel.\nMute status *is \
        not* permanent. As soon as you connect to another Voice Channel, mute status \
        will disappear.\nEach Voice Channel is a separate game session. One will not \
        affect the other. Multiple games can be played *independently and \
        simultaneously* in a server.\n\n\
        **AmongUsBot Commands**\n\
        `!muteall` - Mutes all players in the VC\n\
        `!unmuteall` - Unmutes all players in the VC *except* for those that are dead\n\
        `!kill <@player>` - Mutes the specified player regardless of unmute\n\
        `!revive <@player>` - Unkills a dead player\n\
        `!reset` - Revives all killed players").await?;

    Ok(())
}