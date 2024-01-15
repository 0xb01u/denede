/*
 *  Denedé: Discord bot for generating D&D dice rolls, written in Rust.
 *  Copyright (C) 2023-2024  Bolu <bolu@tuta.io>
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
use serenity::all::Context;
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse};
use serenity::model::application::{CommandOptionType, Interaction, ResolvedOption, ResolvedValue};
use serenity::model::timestamp::Timestamp;

pub async fn run(options: &[ResolvedOption<'_>], ctx: &Context, interaction: &Interaction) -> Option<(String, bool)> {
    if let Interaction::Command(command) = interaction {

        let ephemeral: bool;
        if let Some(ResolvedOption {
            value: ResolvedValue::Boolean(is_ephemeral), ..
        }) = options.first() {
            ephemeral = *is_ephemeral;
        } else {
            ephemeral = true;
        }
        let interaction_date_sent = *interaction.id().created_at();
        let mut now = *Timestamp::now();
        // Take only milliseconds, omit nanoseconds (Discord timestamps only measure up to milliseconds):
        now = now.replace_nanosecond(now.millisecond() as u32 * 1_000_000).unwrap();

        let data = CreateInteractionResponseMessage::new().content(format!("Pong.\nReception latency: {}", now - interaction_date_sent)).ephemeral(ephemeral);
        let builder = CreateInteractionResponse::Message(data);
        if let Err(why) = command.create_response(&ctx.http, builder).await {
            println!("Cannot respond to ping command: {why}");
        }

        // Wait for response to be sent and compute roundtrip latency:
        let response = command.get_response(&ctx.http).await.unwrap();
        let response_date_sent = *response.id.created_at();
        let edit = EditInteractionResponse::new().content(format!("Pong.\nReception latency: {}\nRoundtrip latency: {}", now - interaction_date_sent, response_date_sent - interaction_date_sent));
        if let Err(why) = command.edit_response(&ctx.http, edit).await {
            println!("Cannot edit ping response: {why}");
        }
    }
    None
}

pub fn register() -> CreateCommand {
    CreateCommand::new("ping").description("Get information about the bot's response latency").add_option(
        CreateCommandOption::new(CommandOptionType::Boolean, "hidden", "Hide the command's response to other users")
            .required(false),
    )
}

