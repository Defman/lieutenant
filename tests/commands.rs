use lieutenant::automaton::{Find, DFA, NFA};
use lieutenant::command::*;
use lieutenant::graph::*;
use serde::Deserialize;
use std::time;

#[derive(Deserialize)]
struct CommandFile {
    root: CommandNode,
}

#[derive(Deserialize)]
struct CommandNode {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    executable: bool,
    redirects: Vec<String>,
    children: Vec<CommandNode>,
    parser: Option<Parser>,
}

fn f32_min() -> f32 {
    f32::MIN
}

fn f32_max() -> f32 {
    f32::MAX
}

fn f64_min() -> f64 {
    f64::MIN
}

fn f64_max() -> f64 {
    f64::MAX
}

fn i32_min() -> i32 {
    i32::MIN
}

fn i32_max() -> i32 {
    i32::MAX
}

fn default_entity_amount() -> String {
    "multiple".to_owned()
}

fn default_entity_kind() -> String {
    "entities".to_owned()
}

#[derive(Deserialize)]
struct DoubleModifier {
    #[serde(default = "f64_min")]
    min: f64,
    #[serde(default = "f64_max")]
    max: f64,
}

impl Default for DoubleModifier {
    fn default() -> Self {
        Self {
            min: f64_min(),
            max: f64_max(),
        }
    }
}

#[derive(Deserialize)]
struct IntegerModifier {
    #[serde(default = "i32_min")]
    min: i32,
    #[serde(default = "i32_max")]
    max: i32,
}

impl Default for IntegerModifier {
    fn default() -> Self {
        Self {
            min: i32_min(),
            max: i32_max(),
        }
    }
}
#[derive(Deserialize)]
struct FloatModifier {
    #[serde(default = "f32_min")]
    min: f32,
    #[serde(default = "f32_max")]
    max: f32,
}

impl Default for FloatModifier {
    fn default() -> Self {
        Self {
            min: f32_min(),
            max: f32_max(),
        }
    }
}
#[derive(Deserialize)]
struct EntityModifier {
    #[serde(default = "default_entity_amount")]
    amount: String,
    #[serde(rename = "type", default = "default_entity_kind")]
    kind: String,
}

impl Default for EntityModifier {
    fn default() -> Self {
        Self {
            amount: default_entity_amount(),
            kind: default_entity_kind(),
        }
    }
}

fn default_string_modifier() -> String {
    "word".to_owned()
}

#[derive(Deserialize)]
struct StringModifier {
    #[serde(rename = "type", default = "default_string_modifier")]
    kind: String,
}

impl Default for StringModifier {
    fn default() -> Self {
        Self {
            kind: default_string_modifier(),
        }
    }
}

fn default_score_holder_amount() -> String {
    "multiple".to_owned()
}

#[derive(Deserialize)]
struct ScoreHolderModifier {
    #[serde(default = "default_score_holder_amount")]
    amount: String,
}

impl Default for ScoreHolderModifier {
    fn default() -> Self {
        Self {
            amount: default_score_holder_amount(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "parser", content = "modifier")]
enum Parser {
    #[serde(rename = "brigadier:bool")]
    Bool,
    #[serde(rename = "brigadier:float")]
    Float(Option<FloatModifier>),
    #[serde(rename = "brigadier:double")]
    Double(Option<DoubleModifier>),
    #[serde(rename = "brigadier:integer")]
    Integer(Option<IntegerModifier>),
    #[serde(rename = "minecraft:angle")]
    Angle,
    #[serde(rename = "minecraft:block_pos")]
    BlockPos,
    #[serde(rename = "minecraft:block_predicate")]
    BlockPredicate,
    #[serde(rename = "minecraft:block_state")]
    BlockState,
    #[serde(rename = "minecraft:color")]
    Color,
    #[serde(rename = "minecraft:column_pos")]
    ColumnPos,
    #[serde(rename = "minecraft:component")]
    Component,
    #[serde(rename = "minecraft:dimension")]
    Dimension,
    #[serde(rename = "minecraft:entity_anchor")]
    EntityAnchor,
    #[serde(rename = "minecraft:entity_summon")]
    EntitySummon,
    #[serde(rename = "minecraft:entity")]
    Entity(Option<EntityModifier>),
    #[serde(rename = "brigadier:string")]
    String(Option<StringModifier>),
    #[serde(rename = "minecraft:function")]
    Function,
    #[serde(rename = "minecraft:game_profile")]
    GameProfile,
    #[serde(rename = "minecraft:int_range")]
    IntRange,
    #[serde(rename = "minecraft:item_enchantment")]
    ItemEnchantment,
    #[serde(rename = "minecraft:item_predicate")]
    ItemPredicate,
    #[serde(rename = "minecraft:item_slot")]
    ItemSlot,
    #[serde(rename = "minecraft:item_stack")]
    ItemStack,
    #[serde(rename = "minecraft:message")]
    Message,
    #[serde(rename = "minecraft:mob_effect")]
    MobEffect,
    #[serde(rename = "minecraft:nbt_compound_tag")]
    NbtCompoundTag,
    #[serde(rename = "minecraft:nbt_path")]
    NbtPath,
    #[serde(rename = "minecraft:nbt_tag")]
    NbtTag,
    #[serde(rename = "minecraft:objective")]
    Objective,
    #[serde(rename = "minecraft:objective_criteria")]
    ObjectiveCriteria,
    #[serde(rename = "minecraft:operation")]
    Operation,
    #[serde(rename = "minecraft:particle")]
    Particle,
    #[serde(rename = "minecraft:resource_location")]
    ResourceLocation,
    #[serde(rename = "minecraft:rotation")]
    Rotation,
    #[serde(rename = "minecraft:score_holder")]
    ScoreHolder(Option<ScoreHolderModifier>),
    #[serde(rename = "minecraft:scoreboard_slot")]
    ScoreboardSlot,
    #[serde(rename = "minecraft:swizzle")]
    Swizzle,
    #[serde(rename = "minecraft:team")]
    Team,
    #[serde(rename = "minecraft:time")]
    Time,
    #[serde(rename = "minecraft:uuid")]
    Uuid,
    #[serde(rename = "minecraft:vec2")]
    Vec2,
    #[serde(rename = "minecraft:vec3")]
    Vec3,
}

fn _parser(parser: Parser) -> ParserKind {
    match parser {
        Parser::Bool => ParserKind::Bool,
        Parser::Float(range) => {
            let range = range.unwrap_or_default();
            ParserKind::Float(range.min..=range.max)
        }
        Parser::Double(range) => {
            let range = range.unwrap_or_default();
            ParserKind::Double(range.min..=range.max)
        }
        Parser::Integer(range) => {
            let range = range.unwrap_or_default();
            ParserKind::Integer(range.min..=range.max)
        }
        Parser::Angle => ParserKind::Double(0.0..=360.0),
        Parser::BlockPos => ParserKind::BlockPos,
        Parser::BlockPredicate => ParserKind::BlockPredicate,
        Parser::BlockState => ParserKind::BlockState,
        Parser::Color => ParserKind::Color,
        Parser::ColumnPos => ParserKind::ColumnPos,
        Parser::Component => todo!(),
        Parser::Dimension => ParserKind::Dimension,
        Parser::EntityAnchor => ParserKind::EntityAnchor,
        Parser::EntitySummon => ParserKind::EntitySummon,
        Parser::Entity(modifier) => {
            let modifier = modifier.unwrap_or_default();
            let only_one = match modifier.amount.as_ref() {
                "single" => true,
                "multiple" => false,
                _ => unreachable!(),
            };
            let player_required = match modifier.kind.as_ref() {
                "entities" => false,
                "player" => true,
                _ => unreachable!(),
            };
            ParserKind::Entity {
                only_one,
                player_required,
            }
        }
        Parser::String(modifier) => {
            let modifier = modifier.unwrap_or_default();
            let property = match modifier.kind.as_ref() {
                "word" => StringProperty::SingleWord,
                "phrase" => StringProperty::QuotablePhrase,
                "greedy" => StringProperty::GreedyPhrase,
                _ => unreachable!(),
            };
            ParserKind::String(property)
        }
        Parser::Function => ParserKind::Function,
        Parser::GameProfile => ParserKind::GameProfile,
        Parser::IntRange => ParserKind::IntRange,
        Parser::ItemEnchantment => ParserKind::ItemEnchantment,
        Parser::ItemPredicate => ParserKind::ItemPredicate,
        Parser::ItemSlot => ParserKind::ItemSlot,
        Parser::ItemStack => ParserKind::ItemStack,
        Parser::Message => ParserKind::Message,
        Parser::MobEffect => ParserKind::MobEffect,
        Parser::NbtCompoundTag => ParserKind::NbtCompoundTag,
        Parser::NbtPath => ParserKind::NbtPath,
        Parser::NbtTag => ParserKind::NbtTag,
        Parser::Objective => ParserKind::Objective,
        Parser::ObjectiveCriteria => ParserKind::ObjectiveCritera,
        Parser::Operation => ParserKind::Operation,
        Parser::Particle => ParserKind::Particle,
        Parser::ResourceLocation => ParserKind::ResourceLocation,
        Parser::Rotation => ParserKind::Rotation,
        Parser::ScoreHolder(modifier) => {
            let modifier = modifier.unwrap_or_default();
            let multiple_allowed = match modifier.amount.as_ref() {
                "multiple" => true,
                "single" => false,
                _ => unreachable!(),
            };
            ParserKind::ScoreHolder { multiple_allowed }
        }
        Parser::ScoreboardSlot => ParserKind::ScoreboardSlot,
        Parser::Swizzle => ParserKind::Swizzle,
        Parser::Team => ParserKind::Team,
        Parser::Time => ParserKind::Time,
        Parser::Uuid => ParserKind::Uuid,
        Parser::Vec2 => ParserKind::Vec2,
        Parser::Vec3 => ParserKind::Vec3,
    }
}

fn create_command(root: &mut RootNode<()>, parent: Option<NodeId>, command_node: &CommandNode) {
    let kind = match command_node.kind.as_ref() {
        "root" => unimplemented!("only on root"),
        "literal" => NodeKind::Literal(command_node.name.clone()),
        "argument" => match command_node.name {
            _ => NodeKind::Argument {
                parser: ParserKind::Integer(0..=i32::MAX),
            },
        },
        _ => unimplemented!("should not exist"),
    };

    let mut node = Node::new(kind);

    if command_node.executable {
        node.execute = Some(Box::new(command(|| Ok(()))));
    }

    let graph_node = root.add_node(parent, node);
    for child in &command_node.children {
        create_command(root, Some(graph_node), child);
    }
}

#[test]
fn all_commands() -> Result<(), Box<dyn std::error::Error>> {
    let commands = include_str!("./commands.json");
    let command_file: CommandFile = serde_json::from_str(commands)?;

    let root = command_file.root;

    let mut graph_root = RootNode::new();

    for child in &root.children {
        create_command(&mut graph_root, None, child)
    }

    let start = time::Instant::now();
    let nfa = NFA::from(graph_root);
    println!(
        "nfa took: {}ms",
        time::Instant::now().duration_since(start).as_millis()
    );

    // println!("{:?}", nfa);

    let start = time::Instant::now();
    let dfa = DFA::from(nfa);
    println!(
        "dfa took: {}ms",
        time::Instant::now().duration_since(start).as_millis()
    );

    let dfa = dfa.minimize();

    println!("states: {}", dfa.states.len());

    let size = std::mem::size_of_val(&dfa);
    println!("size: {}", size);

    assert!(dfa.find("tp 10 10 10").is_ok());

    Ok(())
}
