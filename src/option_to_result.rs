use std::collections::HashMap;

use serenity::{
    model::{
        guild::{Guild, Member},
        channel::GuildChannel,
        voice::VoiceState,
        id::{GuildId, ChannelId}
    }
};

use crate::among_us::*;

pub trait ToResult<T> {
    fn to_result(self) -> Result<T, &'static str>;
}

impl ToResult<Guild> for Option<Guild> {
    fn to_result(self) -> Result<Guild, &'static str> {
        self.ok_or("Guild could not be found")
    }
}

impl ToResult<GuildId> for Option<GuildId> {
    fn to_result(self) -> Result<GuildId, &'static str> {
        self.ok_or("Guild ID could not be found")
    }
}

impl<'a> ToResult<&'a VoiceState> for Option<&'a VoiceState> {
    fn to_result(self) -> Result<&'a VoiceState, &'static str> {
        self.ok_or("Voice State could not be found")
    }
}

impl<'a> ToResult<&'a GuildChannel> for Option<&'a GuildChannel> {
    fn to_result(self) -> Result<&'a GuildChannel, &'static str> {
        self.ok_or("Channel could not be found")
    }
}

impl ToResult<Vec<Member>> for Option<Vec<Member>> {
    fn to_result(self) -> Result<Vec<Member>, &'static str> {
        self.ok_or("Members could not be found")
    }
}

impl ToResult<ChannelId> for Option<ChannelId> {
    fn to_result(self) -> Result<ChannelId, &'static str> {
        self.ok_or("Channel ID could not be found")
    }
}

impl<'a> ToResult<&'a mut HashMap<u64, GameInstance>> for Option<&'a mut HashMap<u64, GameInstance>> {
    fn to_result(self) -> Result<&'a mut HashMap<u64, GameInstance>, &'static str> {
        self.ok_or("Games object could not be found")
    }
}

impl<'a> ToResult<&'a mut GameInstance> for Option<&'a mut GameInstance> {
    fn to_result(self) -> Result<&'a mut GameInstance, &'static str> {
        self.ok_or("Games Instance object could not be found")
    }
}