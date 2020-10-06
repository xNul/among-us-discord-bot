use std::collections::HashMap;

pub struct Games;

#[derive(Debug)]
pub struct GameInstance {
    pub leader_user_id: u64,
    pub recent_text_channel_id: u64,
    pub global_unmute: bool,
    pub dead_players: HashMap<u64, bool>
}