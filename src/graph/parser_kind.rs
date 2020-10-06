use crate::automaton::{NFA};
use crate::error::Result;
use crate::values::Value;
use std::ops::RangeInclusive;

#[derive(Debug, PartialEq)]
pub enum StringProperty {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

/// Describes a parser from [this list](https://wiki.vg/Command_Data#Parsers).
/// Doesn't actually provide a means to parse functions—this is only
/// used to build the Declare Commands packet.
#[derive(Debug, PartialEq)]
pub enum ParserKind {
    Bool,
    Double(RangeInclusive<f64>),
    Float(RangeInclusive<f32>),
    Integer(RangeInclusive<i32>),
    String(StringProperty),
    Entity {
        /// Whether only one entity is allowed.
        only_one: bool,
        /// Whether only players will be included.
        player_required: bool,
    },
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    ChatComponent,
    Message,
    JsonNbt,
    NbtPath,
    Objective,
    ObjectiveCritera,
    Operation,
    Particle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder {
        /// Whether more than one entity will be allowed.
        multiple_allowed: bool,
    },
    Swizzle,
    Team,
    ItemSlot,
    ResourceLocation,
    MobEffect,
    Function,
    EntityAnchor,
    Range {
        decimals_allowed: bool,
    },
    IntRange,
    FloatRange,
    ItemEnchantment,
    EntitySummon,
    Dimension,
    Uuid,
    NbtTag,
    NbtCompoundTag,
    Time,
}

impl ParserKind {
    pub fn parse(&self, input: &str) -> Result<Value> {
        use ParserKind::*;
        match self {
            IntRange => Ok(Value::I32(input.parse::<i32>().unwrap())),
            _ => unimplemented!(),
        }
    }
}

impl<'a> From<&'a ParserKind> for NFA {
    fn from(_parser: &'a ParserKind) -> Self {
        // lazy_static! {
        //     static ref WORD: NFA = NFA::from(&Pattern::WORD);
        //     static ref BOOL: NFA = NFA::from(&Pattern::Alt(&[pattern!("true"), pattern!("false")]));
        //     static ref DIGIT: NFA = NFA::from(&Pattern::DIGIT);
        //     static ref SIGN: NFA = NFA::from(&Pattern::alt(&[pattern!("+"), pattern!("-")]));
        //     static ref NUMBER: NFA = DIGIT.clone().concat(&DIGIT.clone().repeat());
        //     static ref INTEGER: NFA = SIGN.clone().concat(&NUMBER);
        //     static ref DOT: NFA = NFA::from(&pattern!("."));
        //     static ref FLOAT: NFA = INTEGER.clone().union(&INTEGER.clone().concat(DOT));
        // }

        // match parser {
        //     ParserKind::Bool => BOOL.clone(),
        //     ParserKind::String(StringProperty::SingleWord) => WORD.clone(),

        //     // ParserKind::Double(_) => concat(&[optional(&pattern!("-"))]),
        //     // ParserKind::String(StringProperty::GreedyPhrase) => Pattern::concat(&[Pattern::WORD, Pattern::SPACE]).repeat(),
        //     _ => todo!(),
        // }
            
        todo!()
    }
}
