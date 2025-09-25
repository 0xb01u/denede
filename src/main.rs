/*
 *  Denedé: Discord bot for generating D&D dice rolls, written in Rust.
 *  Copyright (C) 2023-2025  Bolu <bolu@tuta.io>
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */
mod commands;
mod dice;

use futures::future::join_all;
use regex::Regex;
use serenity::{
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    model::{
        application::{Command, Interaction},
        prelude::*,
    },
    prelude::*,
};
use std::env;

use dice::{CompoundDiceRoll, ErrorKind};

struct Bot;

#[serenity::async_trait]
impl EventHandler for Bot {
    // Process slash commands:
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(ref command) = interaction {
            let cmd_response = match command.data.name.as_str() {
                "roll" => commands::roll::run(&command.data.options()).await,
                "ping" => commands::ping::run(&command.data.options(), &ctx, &interaction).await,
                "license" => commands::license::run(&command.data.options()),
                "code" => commands::code::run(&command.data.options()),
                _ => None,
            };

            if let Some((result, ephemeral)) = cmd_response {
                let data = CreateInteractionResponseMessage::new()
                    .content(result)
                    .ephemeral(ephemeral);
                let builder = CreateInteractionResponse::Message(data);
                if let Err(why) = command.create_response(&ctx.http, builder).await {
                    println!("Could not respond to slash command: {why}");
                }
            }
        }
    }

    // Process text messages => Dice rolls:
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore messages from other bots:
        if msg.author.bot {
            return;
        }

        let dice_expr_regex = Regex::new(r"\[[^\[]+?\]").unwrap();
        let parse_results = dice_expr_regex
            .find_iter(&msg.content)
            .map(|m| CompoundDiceRoll::parse(&m.as_str()[1..m.len() - 1]));
        let dice_results = join_all(parse_results.map(|res| async move {
            match res {
                Ok(d) => d.result().await,
                Err(e) => Err(e),
            }
        }))
        .await;

        // Join all rolls in the corresponding amount of messages:
        let mut response = Vec::new();
        let mut accum_len = 0;
        for roll in dice_results {
            let next_result = if let Ok(result) = roll {
                format!("{}", result)
            } else {
                match roll.err().unwrap().kind {
                    ErrorKind::DiceExprDivisionByZero => {
                        "Zounds! My calculations have fallen into a division by naught!".to_string()
                    }
                    ErrorKind::DiceAmountTooLarge => {
                        "Forgive me, good adventurer; I am overwhelmed by a surplus of dice \
                            in this casting, for I can count but fifty at most!"
                            .to_string()
                    }
                    ErrorKind::DiceTooManySides => {
                        "I beg your pardon, for the die you summoned bears too many faces; \
                            its sides must be fewer than a thousand for my humble reckoning!"
                            .to_string()
                    }
                    ErrorKind::DiceExprInvalidSides => {
                        "The number of sides thou hast named doth not accord with the operation \
                            thou hast chosen."
                            .to_string()
                    }
                    ErrorKind::DiceExprInvalidArgument => {
                        "A flaw dwells in one quality of the operation thou hast invoked. \
                            Prithee, examine the exact requirements anew!"
                            .to_string()
                    }
                    ErrorKind::CompoundDiceMultipleRollErrors => {
                        "More than one of the rolls thou hast named hath yielded an error. \
                            I pray thee, examine each in turn!"
                            .to_string()
                    }
                    _ => continue,
                }
            };

            // If the next result would exceed the message length, send the current response:
            if accum_len + next_result.len() > 2000 {
                let _ = msg.channel_id.say(&ctx.http, response.join("\n")).await;
                response.clear();
                accum_len = 0;
            }

            accum_len += next_result.len();
            response.push(next_result);
        }
        // Send last response:
        let _ = msg.channel_id.say(&ctx.http, response.join("\n")).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        match ready.user.discriminator {
            Some(discriminator) => println!("{}#{discriminator:#?} is connected.", ready.user.name),
            None => println!("{} is connected.", ready.user.name),
        }

        // Register slash commands:
        let commands = Command::set_global_commands(
            &ctx.http,
            vec![
                commands::roll::register(),
                commands::ping::register(),
                commands::license::register(),
                commands::code::register(),
            ],
        )
        .await
        .unwrap();

        println!(
            "Registered the following commands: {:?}",
            commands
                .into_iter()
                .map(|cmd| cmd.name)
                .collect::<Vec<String>>()
        );
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("No tokens?");
    let mut client = Client::builder(
        &token,
        GatewayIntents::default() | GatewayIntents::MESSAGE_CONTENT,
    )
    .event_handler(Bot)
    .await
    .expect("No clients?");

    client.start().await.expect("No work?");
}
