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
use futures::future::join_all;
use regex::Regex;
use std::{collections::HashMap, ops::Deref, sync::LazyLock};

type Result<T> = std::result::Result<T, DiceError>;

/// Enum representing the different kinds of errors that can occur in this module.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ErrorKind {
    RandomOrgOutOfRange,
    RandomOrgInvalidResponse,
    RandomOrgUnreachable,
    DiceStringInvalidCharacters,
    DiceStringTooManyParts,
    DiceStringInvalidOp,
    DiceStringNumberTooLarge,
    DiceAmountTooLarge,
    DiceTooManySides,
    DiceExprDivisionByZero,
    DiceExprInvalidArgument,
    DiceExprInvalidSides,
    CompoundDiceExprInvalidOpStructure,
    CompoundDiceMultipleRollErrors,
}

/// Structure encapsulating an error that can occur in this module.
#[derive(Clone, Copy, Debug)]
pub struct DiceError {
    /// The kind of error that occurred.
    pub kind: ErrorKind,
}

impl DiceError {
    /// Creates a new `DiceError` instance from the specified kind.
    fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

/// Makes a request to the specified RANDOM.ORG endpoint and processes the received response.
///
/// Mainly useful for encapsulating the logic handling all the different errors that can occur.
///
/// # Arguments
/// * `endpoint` - The RANDOM.ORG endpoint to reach.
///
/// # Returns
/// A `Result` containing a vector of integers if the request was successful, or an `Error` if it
/// failed.
async fn process_randomorg_request(endpoint: String) -> Result<Vec<u16>> {
    if let Ok(res) = reqwest::get(endpoint).await {
        if let Ok(text) = res.text().await {
            let first_char = text.chars().next();
            if first_char.is_some() && first_char.unwrap().is_digit(10) {
                let results = text
                    .lines()
                    .map(|line| line.parse::<u16>())
                    .collect::<Vec<_>>();
                if results.iter().any(|r| r.is_err()) {
                    return Err(DiceError::new(ErrorKind::RandomOrgOutOfRange));
                }
                return Ok(results.into_iter().filter_map(|r| r.ok()).collect());
            } else {
                return Err(DiceError::new(ErrorKind::RandomOrgInvalidResponse));
            }
        } else {
            return Err(DiceError::new(ErrorKind::RandomOrgInvalidResponse));
        }
    } else {
        return Err(DiceError::new(ErrorKind::RandomOrgUnreachable));
    }
}

/// Calls the RANDOM.ORG API to get a sequence of random integers, from the given parameters.
///
/// If the request fails, it falls back to using the `rand` crate to generate pseudo-random
/// numbers.
///
/// # Arguments
/// * `num` - The number of random integers to generate.
/// * `max` - The maximum value of the random integers.
/// * `min` - The minimum value of the random integers.
///
/// # Returns
/// A tuple containing a vector of random integers and a boolean indicating whether the
/// numbers were truly random (i.e., fetched from RANDOM.ORG).
async fn call_randomorg(num: u16, max: u16, min: u16) -> (Vec<u16>, bool) {
    let endpoint = format!(
        "https://www.random.org/integers/?\
        num={}&min={}&max={}&col=1&base=10&format=plain&rnd=new",
        num, min, max
    );

    match process_randomorg_request(endpoint).await {
        Ok(seq) => return (seq, true),
        Err(_) => {
            // Generate pseudo-random numbers:

            use rand::prelude::*;
            let mut rng = rand::rng();

            return (
                Vec::from_iter((0..num).map(|_| rng.random_range(min..=max))),
                false,
            );
        }
    };
}

/// Structure representing an identifier for a die operation.
#[derive(PartialEq, Eq, Hash)]
struct DieOpId<'a>(&'a [&'a str]);

impl<'a> Deref for DieOpId<'a> {
    type Target = [&'a str];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Enum representing the different kinds of die operations.
///
/// See [MapTool's Dice Expressions](https://wiki.rptools.info/index.php/Dice_Expressions) for more
/// information.
#[derive(PartialEq, Eq, Hash, Clone)]
enum DieKind {
    Regular,
    Drop,
    Keep,
    Reroll,
    RerollOnceAndKeep,
    RerollOnceAndChoose,
    Success,
    Explode,
    ExplodingSuccess,
    Open,
    DropHigh,
    KeepLow,
    UpperBound,
    LowerBound,
    AddUpperBound,
    AddLowerBound,
    SubtractUpperBound,
    SubtractLowerBound,
    OpenEnded,
    OpenEndedImplicit,
}

/// Macro for conveniently build the map of die operation IDs to their kinds.
macro_rules! map_ids {
    ($($($id:literal);* => $kind:ident),* $(,)?) => {{
        HashMap::from([
            $((
                DieOpId(&[$($id),*]),
                DieKind::$kind
            ),)*
        ])
    }};
}

/// Global map of die operation IDs (strings) to their kinds (`DieKind`).
static DICE_IDS_MAP: LazyLock<HashMap<DieOpId, DieKind>> = LazyLock::new(|| {
    map_ids! {
            => Regular,
            "d" => Drop,
            "k" => Keep,
            "r" => Reroll,
            "rk" => RerollOnceAndKeep,
            "rc" => RerollOnceAndChoose,
            "s" => Success,
            "e" => Explode,
            "e"; "s" => ExplodingSuccess,
            "es" => ExplodingSuccess,
            "o" => Open,
            "dh" => DropHigh,
            "kl" => KeepLow,
            "u" => UpperBound,
            "l" => LowerBound,
            "a"; "u" => AddUpperBound,
            "a"; "l" => AddLowerBound,
            "s"; "u" => SubtractUpperBound,
            "s"; "l" => SubtractLowerBound,
            "oel"; "h" => OpenEnded,
            "oe" => OpenEndedImplicit,
    }
});

/// Structure representing the result of a dice roll.
#[derive(Clone, Debug)]
pub struct DiceResult {
    /// The sequence of rolled values.
    seq: Vec<u16>,
    /// Whether the numbers were truly random (i.e., fetched from RANDOM.ORG).
    truly_random: bool,
    /// The success threshold for the roll, if applicable.
    success_threshold: Option<u16>, // TODO: This seems arbitrary. Reconsider.
}

impl DiceResult {
    /// Creates a new `DiceResult` instance from its components.
    ///
    /// Intended for internal use only.
    ///
    /// # Arguments
    /// * `seq` - The sequence of rolled values.
    /// * `truly_random` - Whether the numbers were truly random (i.e., fetched from RANDOM.ORG).
    /// * `success_threshold` - The success threshold for the roll, if applicable.
    fn new(seq: Vec<u16>, truly_random: bool, success_threshold: Option<u16>) -> Self {
        Self {
            seq,
            truly_random,
            success_threshold,
        }
    }

    /// Creates a `DiceResult` that represents rolling zero dice.
    ///
    /// The total result for this `DiceResult` will be `0`.
    fn zero() -> Self {
        Self::new(vec![], false, None)
    }
}

impl<T> From<(Vec<T>, bool)> for DiceResult
where
    T: Into<u16>,
{
    fn from(value: (Vec<T>, bool)) -> Self {
        let seq = value.0.into_iter().map(|e| e.into()).collect();
        Self::new(seq, value.1, None)
    }
}

impl<T> From<(Vec<T>, bool, u16)> for DiceResult
where
    T: Into<u16>,
{
    fn from(value: (Vec<T>, bool, u16)) -> Self {
        let seq = value.0.into_iter().map(|e| e.into()).collect();
        Self::new(seq, value.1, Some(value.2))
    }
}

impl std::fmt::Display for DiceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.seq.is_empty() {
            write!(f, "0")?;
        } else {
            write!(
                f,
                "{}",
                self.seq
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;

            if self.seq.len() > 1 {
                if let Some(threshold) = self.success_threshold {
                    let sum = self.seq.iter().filter(|&&x| x >= threshold).count();
                    write!(f, " >= {} = {}", threshold, sum)?;
                } else {
                    let sum = self.seq.iter().sum::<u16>();
                    write!(f, " = {}", sum)?;
                }
            }

            if !self.truly_random {
                write!(f, " [pseudo-random]")?;
            }
        }
        Ok(())
    }
}

/// Structure representing some same-sided dice to be rolled.
///
/// Dice to be rolled can be of different kinds, as specified by
/// [MapTool's Dice Expressions](https://wiki.rptools.info/index.php/Dice_Expressions),
/// defining different operations and results.
pub struct Dice {
    /// The amount of dice to roll.
    amount: u16,
    /// The number of sides of the dice to roll.
    sides: u16,
    /// The kind of die operation to perform.
    kind: DieKind,
    /// Additional arguments for the die operation, if applicable.
    args: Vec<u16>,
}

impl Dice {
    /// Parses a dice string and returns a `Dice` instance.
    ///
    /// For details on the available operations and expected format, see
    /// [MapTool's Dice Expressions](https://wiki.rptools.info/index.php/Dice_Expressions).
    ///
    /// # Arguments
    /// * `text` - The dice string to parse.
    ///
    /// # Returns
    /// A `Result` containing a `Dice` instance if the parsing was successful, or an `Error` if it
    /// failed.
    fn parse(text: &str) -> Result<Dice> {
        // Allow only alphanumeric strings:
        let alphanumeric_regex = Regex::new(r"^[a-z0-9]+$").unwrap();
        if !alphanumeric_regex.is_match(text) {
            return Err(DiceError::new(ErrorKind::DiceStringInvalidCharacters));
        }

        // Roll may be only a number:
        if let Ok(number) = text.parse::<u16>() {
            return Ok(Dice {
                amount: number,
                sides: 1,
                kind: DieKind::Regular,
                args: vec![],
            });
        }

        // Get all the die operations specified:
        let num_regex = Regex::new(r"\d+").unwrap();
        let mut ids = num_regex
            .split(&text)
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();

        // Remove the dice roll (d) op, if found, for convenience (found in all ops):
        let specifies_sides = ids[0] == "d";
        if specifies_sides {
            ids.remove(0);
        }

        // First check on the number of die operation parts specified:
        if ids.len() > 2 {
            return Err(DiceError::new(ErrorKind::DiceStringTooManyParts));
        }

        // Check the remaining op parts are valid:
        let valid_ids = DICE_IDS_MAP
            .keys()
            .map(|x| x.iter())
            .flatten()
            .map(|&x| x)
            .collect::<Vec<_>>();
        if ids.iter().any(|id| !valid_ids.contains(id)) {
            return Err(DiceError::new(ErrorKind::DiceStringInvalidOp));
        }

        // Check the remaining parts correctly conform a valid op, and extract the op:
        let kind = DICE_IDS_MAP
            .get(&DieOpId(&ids))
            .ok_or_else(|| DiceError::new(ErrorKind::DiceStringInvalidOp))?
            .clone();

        // Extract numbers:

        // Dice amount:
        let starts_by_number_regex = Regex::new(r"^\d+").unwrap();
        let starts_by_number_match = starts_by_number_regex.find(&text);
        let starts_by_number = starts_by_number_match.is_some();
        let amount = if let Some(m) = starts_by_number_match {
            m.as_str()
                .parse::<u16>()
                .map_err(|_| DiceError::new(ErrorKind::DiceStringNumberTooLarge))?
        } else {
            1
        };

        // Other numbers in the expression:
        let alpha_regex = Regex::new(r"\D+").unwrap();
        let mut numbers = alpha_regex
            .split(&text)
            .filter(|x| !x.is_empty())
            .map(|x| {
                x.parse::<u16>()
                    .map_err(|_| DiceError::new(ErrorKind::DiceStringNumberTooLarge))
            })
            .collect::<Result<Vec<_>>>()?;
        if numbers.is_empty() {
            assert!(
                !starts_by_number && specifies_sides,
                "Found die different than [d] without any numbers"
            );
            numbers.push(20); // Default sides if not specified.
        }
        assert!(numbers.len() <= 4, "Too many die numbers specified");
        // Remove amount (already processed):
        if starts_by_number {
            numbers.remove(0);
        }

        // Dice sides:
        let sides = if specifies_sides && !numbers.is_empty() {
            numbers.remove(0)
        } else {
            20
        };

        // Remaining numbers are the args of the specific op:
        let mut args = numbers;

        // Add optional args for those ops that support them:
        match kind {
            DieKind::Explode | DieKind::Open => {
                if args.len() == 1 {
                    args.push(u16::MAX);
                }
            }
            DieKind::ExplodingSuccess => {
                if args.len() == 2 {
                    args.insert(1, u16::MAX);
                }
            }
            _ => {}
        };

        // Arbitrary limits checks, so only reasonable amounts of numbers of reasonable size are
        // handled:
        if amount > 50 {
            return Err(DiceError::new(ErrorKind::DiceAmountTooLarge));
        } else if sides > 1000 {
            return Err(DiceError::new(ErrorKind::DiceTooManySides));
        }

        Ok(Dice {
            amount,
            sides,
            kind,
            args,
        })
    }

    /// Regular dice roll and reroll operations.
    async fn regular(&self) -> Result<DiceResult> {
        let threshold = if self.kind == DieKind::Reroll {
            self.args[0]
        } else {
            1
        };
        if threshold > self.sides {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidArgument));
        }

        if self.sides == 1 {
            return Ok((vec![self.amount], true).into());
        }

        let (seq, truly_random) = call_randomorg(self.amount, self.sides, threshold).await;

        return Ok((seq, truly_random).into());
    }

    /// Success operation.
    async fn success(&self) -> Result<DiceResult> {
        let threshold = self.args[0];
        if threshold > self.sides {
            return Ok((vec![0u16], true).into());
        }

        if self.sides == 1 {
            return Ok((vec![self.amount], true).into());
        }

        let res = call_randomorg(self.amount, self.sides, 1).await;

        return Ok((res.0, res.1, threshold).into());
    }

    /// Dice drop and drop high operations.
    async fn drop(&self) -> Result<DiceResult> {
        let drop_count = self.args[0] as usize;
        if drop_count > self.amount as usize {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidArgument));
        }

        if self.sides == 1 {
            return Ok((vec![self.amount - drop_count as u16], true).into());
        }

        let (seq, truly_random) = call_randomorg(self.amount, self.sides, 1).await;

        let mut sorted_seq = seq.clone();
        sorted_seq.sort_unstable();
        let threshold = if self.kind == DieKind::DropHigh {
            sorted_seq[sorted_seq.len() - drop_count]
        } else {
            sorted_seq[drop_count - 1]
        };

        let mut dropped_seq = Vec::with_capacity(seq.len() - drop_count);
        let mut dropped = 0;
        for value in seq {
            let drop_condition = if self.kind == DieKind::DropHigh {
                value >= threshold
            } else {
                value <= threshold
            };
            if drop_condition {
                dropped_seq.push(value);
            } else {
                dropped += 1;
            }

            if dropped == drop_count {
                break;
            }
        }

        return Ok((dropped_seq, truly_random).into());
    }

    /// Dice keep and keep low operations.
    async fn keep(&self) -> Result<DiceResult> {
        let keep_count = self.args[0] as usize;
        if keep_count > self.amount as usize {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidArgument));
        }

        if self.sides == 1 {
            return Ok((vec![keep_count as u16], true).into());
        }

        let (seq, truly_random) = call_randomorg(self.amount, self.sides, 1).await;

        let mut sorted_seq = seq.clone();
        sorted_seq.sort_unstable();
        let threshold = if self.kind == DieKind::KeepLow {
            sorted_seq[keep_count - 1]
        } else {
            sorted_seq[seq.len() - keep_count]
        };

        let mut kept_seq = Vec::with_capacity(keep_count);
        let mut kept = 0;
        for value in seq {
            let keep_condition = if self.kind == DieKind::KeepLow {
                value <= threshold
            } else {
                value >= threshold
            };
            if keep_condition {
                kept_seq.push(value);
                kept += 1;
            }

            if kept == keep_count {
                break;
            }
        }

        return Ok((kept_seq, truly_random).into());
    }

    /// Dice reroll once and keep, and reroll once and choose operations.
    async fn reroll_once(&self) -> Result<DiceResult> {
        let threshold = self.args[0];
        if threshold > self.sides {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidArgument));
        }

        if self.sides == 1 {
            return Ok((vec![self.amount], true).into());
        }

        let (seq, truly_random) = call_randomorg(self.amount, self.sides, 1).await;

        let reroll_count = seq
            .iter()
            .fold(0, |acc, &x| if x < threshold { acc + 1 } else { acc });
        if reroll_count == 0 {
            // No rerolls needed, early exit:
            return Ok((seq.into_iter().collect::<Vec<_>>(), truly_random).into());
        }

        let (seq_reroll, truly_random_reroll) =
            call_randomorg(reroll_count, self.sides, threshold).await;
        assert!(
            seq_reroll.len() == reroll_count as usize,
            "Reroll count mismatch"
        ); // Assert we can unwrap the iterator later.

        let mut reroll_iter = seq_reroll.into_iter();
        let result_seq = seq
            .into_iter()
            .map(|x| {
                if x < threshold {
                    if self.kind == DieKind::RerollOnceAndChoose {
                        x.max(reroll_iter.next().unwrap())
                    } else {
                        reroll_iter.next().unwrap()
                    }
                } else {
                    x
                }
            })
            .collect::<Vec<_>>();

        return Ok((result_seq, truly_random && truly_random_reroll).into());
    }

    /// Dice explode and exploding success operations.
    async fn explode(&self) -> Result<DiceResult> {
        if self.kind == DieKind::ExplodingSuccess {
            let threshold = self.args[1];
            if threshold > self.sides {
                return Err(DiceError::new(ErrorKind::DiceExprInvalidArgument));
            }
        }

        if self.sides == 1 {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidSides));
        }

        let (mut seq, mut truly_random) = call_randomorg(self.amount, self.sides, 1).await;

        let mut last_rolls = seq.clone();
        let limit = self.args[0];
        let mut rerolls_done = 0;
        loop {
            let i = last_rolls.iter().filter(|&&x| x == self.sides).count() as u16;
            if i == 0 || rerolls_done >= limit {
                break;
            }

            let (new_rolls, truly_random_new) = call_randomorg(i, self.sides, 1).await;
            assert!(new_rolls.len() == i as usize, "Explode count mismatch");

            seq.extend(&new_rolls);
            last_rolls = new_rolls.clone();
            truly_random = truly_random && truly_random_new;
            rerolls_done += 1;
        }

        if self.kind == DieKind::ExplodingSuccess {
            let threshold = self.args[1];
            return Ok((seq, truly_random, threshold).into());
        } else {
            return Ok((seq, truly_random).into());
        }
    }

    /// Dice open operation.
    async fn open(&self) -> Result<DiceResult> {
        if self.sides == 1 {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidSides));
        }

        let (mut seq, mut truly_random) = call_randomorg(self.amount, self.sides, 1).await;

        let limit = self.args[0];
        for value in seq.iter_mut() {
            let mut rerolls_done = 0;
            let last_roll = *value;
            while last_roll == self.sides && rerolls_done < limit {
                let (new_rolls, truly_random_new) = call_randomorg(1, self.sides, 1).await;
                assert!(new_rolls.len() == 1, "Open roll count mismatch");

                *value += new_rolls[0];
                truly_random = truly_random && truly_random_new;
                rerolls_done += 1;
            }
        }

        return Ok((seq, truly_random).into());
    }

    /// Dice upper bound, lower bound, add upper bound, add lower bound, subtract upper bound,
    /// and subtract lower bound operations.
    async fn bounded(&self) -> Result<DiceResult> {
        #[derive(PartialEq, Eq)]
        enum BoundKind {
            Upper,
            Lower,
        }
        enum BoundOp {
            None,
            Add,
            Subtract,
        }

        let subkind = match self.kind {
            DieKind::UpperBound | DieKind::AddUpperBound | DieKind::SubtractUpperBound => {
                BoundKind::Upper
            }
            DieKind::LowerBound | DieKind::AddLowerBound | DieKind::SubtractLowerBound => {
                BoundKind::Lower
            }
            _ => unreachable!(),
        };
        let op = match self.kind {
            DieKind::UpperBound | DieKind::LowerBound => BoundOp::None,
            DieKind::AddUpperBound | DieKind::AddLowerBound => BoundOp::Add,
            DieKind::SubtractUpperBound | DieKind::SubtractLowerBound => BoundOp::Subtract,
            _ => unreachable!(),
        };

        let bound = match op {
            BoundOp::None => self.args[0] as i16,
            BoundOp::Add | BoundOp::Subtract => self.args[1] as i16,
        };
        let modifier = match op {
            BoundOp::None => 0,
            BoundOp::Add => self.args[0] as i16,
            BoundOp::Subtract => -((self.args[0]) as i16),
        };

        if self.sides == 1 {
            if subkind == BoundKind::Upper {
                return Ok((
                    vec![(self.amount as i16 * bound.min(modifier + 1).max(0)) as u16],
                    true,
                )
                    .into());
            } else {
                return Ok((
                    vec![(self.amount as i16 * bound.max(modifier + 1)) as u16],
                    true,
                )
                    .into());
            }
        }

        let (seq, truly_random) = call_randomorg(self.amount, self.sides, 1).await;

        let seq = seq
            .iter()
            .map(|&value| {
                if subkind == BoundKind::Upper {
                    (value as i16 + modifier).min(bound).max(0) as u16
                } else {
                    (value as i16 + modifier).max(bound) as u16
                }
            })
            .collect::<Vec<_>>();

        return Ok((seq, truly_random).into());
    }

    /// Dice open-ended operations (both variants).
    async fn open_ended(&self) -> Result<DiceResult> {
        if self.sides == 1 {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidSides));
        }

        let low = self.args[0];
        let high = if self.kind == DieKind::OpenEnded {
            self.args[1]
        } else {
            self.sides + 1 - low
        };
        if high <= low || high > self.sides {
            return Err(DiceError::new(ErrorKind::DiceExprInvalidArgument));
        }

        let (seq, mut truly_random) = call_randomorg(self.amount, self.sides, 1).await;

        let mut result_seq = vec![0; seq.len()];
        for (i, &value) in seq.iter().enumerate() {
            if value >= high {
                loop {
                    let (new_rolls, truly_random_new) = call_randomorg(1, self.sides, 1).await;
                    assert!(new_rolls.len() == 1, "Open-ended roll count mismatch");

                    result_seq[i] += new_rolls[0];
                    truly_random = truly_random && truly_random_new;

                    if new_rolls[0] < high {
                        break;
                    }
                }
            } else if value <= low {
                loop {
                    let (new_rolls, truly_random_new) = call_randomorg(1, self.sides, 1).await;
                    assert!(new_rolls.len() == 1, "Open-ended roll count mismatch");

                    result_seq[i] -= new_rolls[0];
                    truly_random = truly_random && truly_random_new;

                    if new_rolls[0] < high {
                        break;
                    }
                }
            }
        }

        return Ok((result_seq, truly_random).into());
    }

    /// Rolls the dice according to the specified behavior and parameters, and returns the result.
    ///
    /// For details on the expected behavior and parameters, see
    /// [MapTool's Dice Expressions](https://wiki.rptools.info/index.php/Dice_Expressions).
    ///
    /// # Returns
    /// A `DiceResult` with the result of rolling these dice.
    pub async fn roll(&self) -> Result<DiceResult> {
        if self.amount == 0 || self.sides == 0 {
            return Ok(DiceResult::zero());
        }

        match self.kind {
            DieKind::Regular | DieKind::Reroll => self.regular().await,
            DieKind::Success => self.success().await,
            DieKind::Drop | DieKind::DropHigh => self.drop().await,
            DieKind::Keep | DieKind::KeepLow => self.keep().await,
            DieKind::RerollOnceAndKeep | DieKind::RerollOnceAndChoose => self.reroll_once().await,
            DieKind::Explode | DieKind::ExplodingSuccess => self.explode().await,
            DieKind::Open => self.open().await,
            DieKind::UpperBound
            | DieKind::LowerBound
            | DieKind::AddUpperBound
            | DieKind::AddLowerBound
            | DieKind::SubtractUpperBound
            | DieKind::SubtractLowerBound => self.bounded().await,
            DieKind::OpenEnded | DieKind::OpenEndedImplicit => self.open_ended().await,
        }
    }
}

/// Enum representing the arithmetic operations that can be performed on dice rolls.
#[derive(PartialEq, Eq, Clone)]
enum DiceArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl From<&str> for DiceArithmeticOp {
    fn from(op: &str) -> Self {
        match op {
            "+" => DiceArithmeticOp::Add,
            "-" => DiceArithmeticOp::Subtract,
            "*" => DiceArithmeticOp::Multiply,
            "/" => DiceArithmeticOp::Divide,
            _ => unreachable!("Invalid arithmetic operation: {}", op),
        }
    }
}

/// Enum representing the number resulting from a compound dice roll (`CompoundDiceRoll`), which
/// can be either an integer or a floating-point number (the latter only in the case of division).
pub enum RollNumber {
    /// Represents an integer number.
    Int(i32),
    /// Represents a floating-point number.
    Float(f64),
}

impl std::fmt::Display for RollNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RollNumber::Int(num) => write!(f, "{}", num),
            RollNumber::Float(num) => write!(f, "{:.2}", num),
        }
    }
}

/// Structure representing the result of a compound dice roll, including all partial results and
/// the final result of combining them.
pub struct CompoundDiceResult {
    /// The results of each individual dice roll in the compound.
    individuals: Vec<DiceResult>,
    /// The final result of the compound dice roll.
    total: RollNumber,
}

impl std::fmt::Display for CompoundDiceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.individuals.len() > 1 {
            write!(
                f,
                "{}; = {}",
                self.individuals
                    .iter()
                    .map(|res| res.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
                self.total
            )?;
        } else {
            write!(f, "{}", self.individuals[0])?;
        }

        Ok(())
    }
}

/// Structure representing a compound dice roll, which consist of one or more independent
/// same-sided dice rolls, and the arithmetic operations that combine all of them into a single
/// result.
pub struct CompoundDiceRoll {
    /// The dice to be rolled.
    dice: Vec<Dice>,
    /// The arithmetic operations to be performed on the rolled dice.
    ops: Vec<DiceArithmeticOp>,
}

impl CompoundDiceRoll {
    /// Parses a text into a `CompoundDiceRoll` instance.
    ///
    /// For details on the expected format, see
    /// [MapTool's Dice Expressions](https://wiki.rptools.info/index.php/Dice_Expressions).
    ///
    /// # Arguments
    /// * `text` - The text to parse.
    ///
    /// # Returns
    /// A `Result` containing a `CompoundDiceRoll` instance if the text represents a valid dice roll,
    /// or an `Error` if it does not.
    pub fn parse(text: &str) -> Result<CompoundDiceRoll> {
        // Remove all whitespace from the text and set all charactes to lowercase:
        let whitespace_regex = Regex::new(r"\s+").unwrap();
        let mut text = whitespace_regex.replace_all(text, "").to_lowercase();

        // Collapse redundant unary arithmetic operations:
        let redundant_pluses_regex = Regex::new(r"\+\++").unwrap();
        let redundant_minuses_regex = Regex::new(r"(\-\-)+").unwrap();
        // Collapse plus-minus combinations:
        let negative_change_regex = Regex::new(r"\-\+").unwrap();
        let positive_change_regex = Regex::new(r"\+\-").unwrap();
        loop {
            let new_text = redundant_pluses_regex.replace_all(&text, "+");
            let new_text = redundant_minuses_regex.replace_all(&new_text, "+");
            let new_text = negative_change_regex.replace_all(&new_text, "-");
            let new_text = positive_change_regex.replace_all(&new_text, "-");
            if new_text == text {
                break;
            }
            text = new_text.to_string();
        }

        // Handle trailing (end) unary arithmetic operations:
        if text.ends_with('+') {
            // Remove empty split from Dice Expressions:
            text.pop();
        } else if text.ends_with('-') {
            // Remove empty split from Dice Expressions, and negate the final result:
            text.pop();
            text = text
                .chars()
                .map(|c| {
                    if c == '+' {
                        '-'
                    } else if c == '-' {
                        '+'
                    } else {
                        c
                    }
                })
                .collect();
            if !text.starts_with('-') && !text.starts_with('+') {
                text = format!("-{}", text);
            }
        }

        // Handle other trailing arithmetic operations:
        if text.ends_with('*') || text.ends_with('/') {
            return Err(DiceError::new(
                ErrorKind::CompoundDiceExprInvalidOpStructure,
            ));
        }

        // Extract and parse every dice:
        let arithmetic_ops = Regex::new(r"[\+\-\*\/]").unwrap();
        let mut dice_exprs = arithmetic_ops.split(&text);

        if text.starts_with('+') || text.starts_with('-') {
            // Remove empty split from Dice Expressions:
            dice_exprs.next();
        }

        // Error if empty Dice Expressions are found (which means two incompatible arithmetic ops
        // were specified next to each other):
        let dice_exprs = dice_exprs.collect::<Vec<_>>();
        if dice_exprs.iter().any(|expr| expr.is_empty()) {
            return Err(DiceError::new(
                ErrorKind::CompoundDiceExprInvalidOpStructure,
            ));
        }

        let dice = dice_exprs
            .into_iter()
            .map(|expr| Dice::parse(expr))
            .collect::<Result<Vec<_>>>()?;

        // Extract and parse the arithmetic operations:
        let ops = arithmetic_ops
            .find_iter(&text)
            .map(|cap| cap.as_str().into())
            .collect::<Vec<_>>();

        Ok(CompoundDiceRoll { dice, ops })
    }

    /// Rolls all the dice in this `CompoundDiceRoll` instance, combines the results according to the
    /// arithmetic operations specified, and returns the final result.
    ///
    /// # Returns
    /// The result of this `CompoundDiceRoll`.
    pub async fn result(&self) -> Result<CompoundDiceResult> {
        // Roll all the dice:
        let rolls = join_all(self.dice.iter().map(|dice| dice.roll())).await;
        // Handle roll errors:
        let errors = rolls
            .clone()
            .into_iter()
            .filter_map(|res| res.err())
            .collect::<Vec<_>>();
        if errors.len() > 0 {
            if errors.len() == 1 {
                // Only one dice roll, return its error:
                return Err(errors[0]);
            }
            return Err(DiceError::new(ErrorKind::CompoundDiceMultipleRollErrors));
        }
        // Get all the sums of all the dice rolls:
        let individuals = rolls
            .into_iter()
            .filter_map(|res| res.ok())
            .collect::<Vec<_>>(); // Unwrap roll results.
        let results = individuals
            .iter()
            .map(|res| res.seq.iter().sum())
            .collect::<Vec<_>>();

        // Extract the arithmetic operations list:
        let mut ops = self.ops.clone();
        let has_divide = ops.contains(&DiceArithmeticOp::Divide); // For later.
        if ops.len() < results.len() {
            // No first op specified.
            // Fold init value is zero, the first result will be added to that:
            ops.insert(0, DiceArithmeticOp::Add);
        }

        // Zip the results and operations together, for folding later:
        let res_op_pairs = results.into_iter().zip(ops.into_iter()).collect::<Vec<_>>();

        // Check for division by zero and return an error if applicable:
        if res_op_pairs.contains(&(0, DiceArithmeticOp::Divide)) {
            return Err(DiceError::new(ErrorKind::DiceExprDivisionByZero));
        }

        // Compute the final result based on the arithmetic operations:
        let total = if has_divide {
            // If there is a division operation, we need to return a float:
            RollNumber::Float(
                res_op_pairs
                    .into_iter()
                    .fold(0.0, |acc, (result, op)| match op {
                        DiceArithmeticOp::Add => acc + result as f64,
                        DiceArithmeticOp::Subtract => acc - result as f64,
                        DiceArithmeticOp::Multiply => acc * result as f64,
                        DiceArithmeticOp::Divide => {
                            if result == 0 {
                                unreachable!("Division by zero in dice roll result");
                            }
                            acc / result as f64
                        }
                    }),
            )
        } else {
            // If there is no division operation, we can return an integer:
            RollNumber::Int(
                res_op_pairs
                    .into_iter()
                    .fold(0, |acc, (result, op)| match op {
                        DiceArithmeticOp::Add => acc + result as i32,
                        DiceArithmeticOp::Subtract => acc - result as i32,
                        DiceArithmeticOp::Multiply => acc * result as i32,
                        _ => unreachable!("Division is not supported in integer dice rolls"),
                    }),
            )
        };

        Ok(CompoundDiceResult { individuals, total })
    }
}
