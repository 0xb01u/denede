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

use crate::dice::{CompoundDiceRoll, ErrorKind};

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
    let result = match CompoundDiceRoll::parse(&dice_expression) {
        Ok(roll) => roll.result().await,
        Err(e) => Err(e),
    };
    let result_str = if let Ok(value) = result {
        format!("{}", value)
    } else {
        match result.err().unwrap().kind {
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
            _ => "The dice formula thou hast set is ill‑formed. \
               I beseech thee, check it once more!"
                .to_string(),
        }
    };

    if result_str.len() > 2000 {
        return Some((
            "I pray forgiveness, yet the outcome doth exceed the bounds of a single missive, \
            and thus I cannot lay it before thee in one scroll."
                .to_string(),
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
