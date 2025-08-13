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

use crate::dice::{CompoundDiceRoll, DiceErrorKind};

pub async fn run(options: &[ResolvedOption<'_>]) -> Option<(String, bool)> {
    let ephemeral;
    if let Some(ResolvedOption {
        name: "hidden",
        value: ResolvedValue::Boolean(is_ephemeral),
        ..
    }) = options.last()
    {
        ephemeral = *is_ephemeral;
    } else {
        ephemeral = true;
    };

    let mut dice_expression;
    if let Some(ResolvedOption {
        name: "expression",
        value: ResolvedValue::String(expr),
        ..
    }) = options.first()
    {
        dice_expression = expr.to_owned();
    } else {
        unreachable!("Invoked /roll with no dice expression.");
    };

    // Remove leading and trailing brackets, if present:
    if dice_expression.starts_with('[') && dice_expression.ends_with(']') {
        dice_expression = &dice_expression[1..dice_expression.len() - 1];
    }

    // Process dice expression as it is done in regular messages:
    let dice_roll = match CompoundDiceRoll::parse(&dice_expression) {
        Ok(roll) => roll,
        Err(_) => {
            return Some(("Malformed dice expression.".to_string(), ephemeral));
        }
    };
    let result = dice_roll.result().await;
    let result_str = match result {
        Ok(result) => format!("{}", result),
        Err(e) => match e.kind {
            DiceErrorKind::DiceExprDivisionByZero => {
                "The roll produced a division by zero".to_string()
            }
            DiceErrorKind::DiceExprInvalidArgument => {
                "The specified dice contains at least one argument with an invalid value"
                    .to_string()
            }
            _ => unreachable!("Unexpected error kind: {:?}", e.kind),
        },
    };

    if result_str.len() > 2000 {
        return Some((
            "The result is too long to be displayed in a single message.".to_string(),
            ephemeral,
        ));
    }
    Some((result_str, ephemeral))
}

pub fn register() -> CreateCommand {
    CreateCommand::new("roll")
        .description("Roll dice.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "expression",
                "The dice expression for the dice to roll",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "hidden",
                "Hide the command's response to other users (default = true).",
            )
            .required(false),
        )
}
