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
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandOptionType, ResolvedOption, ResolvedValue};

pub fn run(options: &[ResolvedOption]) -> Option<(String, bool)> {
    let ephemeral;
    if let Some(ResolvedOption {
        value: ResolvedValue::Boolean(is_ephemeral),
        ..
    }) = options.first()
    {
        ephemeral = *is_ephemeral;
    } else {
        ephemeral = true;
    }

    Some((
        "My source code can be found here: https://codeberg.org/bolu/denede".to_string(),
        ephemeral,
    ))
}

pub fn register() -> CreateCommand {
    CreateCommand::new("code")
        .description("Get a link to this bot's source code.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "hidden",
                "Hide the command's response to other users (default = true).",
            )
            .required(false),
        )
}
