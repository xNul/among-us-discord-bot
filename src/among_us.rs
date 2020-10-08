use std::collections::HashMap;
use serenity::prelude::TypeMapKey;

pub struct Games;

impl TypeMapKey for Games {
    type Value = HashMap<u64, GameInstance>;
}

#[derive(Debug)]
pub struct GameInstance {
    pub leader_user_id: u64,
    pub recent_text_channel_id: u64,
    pub global_unmute: bool,
    pub dead_players: HashMap<u64, bool>
}