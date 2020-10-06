# Among Us Discord Bot

Hands-free Discord bot to help people in your Discord server play Among Us. Your users manage the game, the Among Us Discord Bot helps them play.

## Features

- Mutes players while not discussing and while dead.
- A single player manages each game, automatically selected.
- A game per voice channel. Dozens of games running simultaneously.
- Safe server muting. Leaving any game will automatically unmute you.
- Simple and intuitive commands built for the game.
- Asynchronous and built in Rust for the best performance.
- Only needs two permissions to operate.
- Basic error handling and logging

## My Bot Invite

Want to invite my AmongUsBot to your Discord server? [Here's the link!](https://discord.com/api/oauth2/authorize?client_id=754473493201944586&permissions=4196352&scope=bot) Use `!help` to get started.

## Planned Features

- Reaction controls
- Predefined text channel for each voice channel
- Retain mutes given as punishment
- DMing commands
- Document/revise codebase
- Command Atomicity
- Command to change prefix for Guild
- Go back and drop Ctx as early as possible for better async

## Building

Before you are able to build the bot, you must create a bot application with Discord and grab the token. Once you have the it, create a file called `config.rs` and add this code to it:

```rust
pub static TOKEN: &'static str = "<discord bot token here>";
```

Afterwards, place the `config.rs` under the `src` folder.

Now you use Cargo, the Rust package manager, to build the executable. Once you have installed Cargo, clone this repository, add your `config.rs`, and run:

```
cargo build --release
```

the resulting executable will be under `target/release/`.