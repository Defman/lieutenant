pub mod command;
mod generic;

use std::fmt;

use anyhow::Result;

type NodeId = usize;
type CommandId = usize;

pub trait Validator {
    fn validate<'a, 'b>(&self, input: &'a str) -> (bool, &'b str) 
    where 'a : 'b;
}
pub struct Node {
    validator: Box<dyn Validator>,
    children: Vec<NodeId>,
    command: Option<CommandId>,
}

impl Validator for Node {
    fn validate<'a,'b>(&self, input: &'a str) -> (bool, &'b str) where 'a : 'b {
        self.validator.validate(input)
    }
}

impl Node {
    pub fn new<V: Validator + 'static>(validator: V) -> Self {
        Node {
            validator: Box::new(validator),
            children: vec![],
            command: None,
        }
    }
    pub fn command(mut self, command: CommandId) -> Self {
        self.command = Some(command);
        self
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(validator:???,children:{:?},command:{:?})",
            self.children, self.command
        )
    }
}

#[derive(Default, Debug)]
pub struct Dispatcher {
    root: Vec<NodeId>,
    nodes: Vec<Node>,
}

impl Dispatcher {
    pub fn add(&mut self, parent: Option<NodeId>, node: Node) -> NodeId {
        let node_id = self.nodes.len();
        self.nodes.push(node);

        if let Some(parent_id) = parent {
            self.nodes[parent_id].children.push(node_id);
        } else {
            self.root.push(node_id);
        }
        node_id
    }

    pub fn find(&self, input: &str) -> Option<CommandId> {
        //let mut input = input;

        let mut stack = self
            .root
            .clone()
            .into_iter()
            .map(|child| (input, child))
            .collect::<Vec<_>>();

        println!("First Stack: {:?}", self.root);
        //println!("Non-Root-Nodes: {:?}",self.nodes.iter().enumerate().filter(|(i,_)| self.root.contains(i)).map(|(_,n)| n.command).collect::<Vec<_>>());
        //println!("{:?}",self);

        while let Some((input, node_id)) = stack.pop() {
            println!("Stack: {:?} (input:{}, node_id:{})", stack, input, node_id);

            let node = &self.nodes[node_id];
            match node.validate(input) {
                (true, out) => {
                    println!("input_after_validate_ {}", input);
                    if out.trim_start().is_empty() {
                        if let Some(command_id) = node.command {
                            return Some(command_id);
                        }
                    } else {
                        stack.extend(node.children.iter().map(|child_id| (out.trim_start(), *child_id)));
                    }
                } 
                (false, _) => {}
            }
            // if node.validate(&input) {
            //     println!("input_after_validate_ {}", input);
            //     let input = input.trim_start();
            //     if input.is_empty() {
            //         if let Some(command_id) = node.command {
            //             return Some(command_id);
            //         }
            //     } else {
            //         stack.extend(node.children.iter().map(|child_id| (input, *child_id)));
            //     }
            // }
        }

        None
    }
}

pub trait AddToDispatcher {
    fn add_to_dispatcher(&self, parent: Option<NodeId>, dispatcher: &mut Dispatcher) -> NodeId;
}

#[cfg(test)]
mod tests {

    use crate::command::CommandBuilder;
    use crate::{
        command::{literal, Command},
        AddToDispatcher, Dispatcher,
    };

    #[test]
    fn simple() {
        let command = literal("tp")
            .arg()
            .arg()
            .arg()
            .build(|x: u32, y: u32, z: u32| {
                move |_state: &mut u32| println!("Command call result: {} {} {}", x, y, z)
            });

        (Command::call(&command, "tp 10 11 12").unwrap())(&mut 0);

        let mut dispatcher = Dispatcher::default();
        command.add_to_dispatcher(None, &mut dispatcher);
        let command_id = dispatcher.find("tp 10 11 12");
        assert!(command_id.is_some())
    }
    #[test]
    fn simple_opt() {
        let command = literal("tp")
            .opt_arg()
            .build(|x: Option<u32>| move |_state: &mut u32| println!("{:?}", x));

        (Command::call(&command, "tp 10").unwrap())(&mut 0);

        let mut dispatcher = Dispatcher::default();
        command.add_to_dispatcher(None, &mut dispatcher);

        let command_id = dispatcher.find("tp 10");
        assert!(command_id.is_some())
    }

    #[test]
    fn simple_opt2() {
        let command = literal("tp")
            .opt_arg()
            .arg()
            .build(|x: Option<u32>, y: u32| move |_state: &mut u32| println!("{:?}, {:?}", x, y));

        (Command::call(&command, "tp 10 ").unwrap())(&mut 0);

        let mut dispatcher = Dispatcher::default();
        command.add_to_dispatcher(None, &mut dispatcher);

        let command_id = dispatcher.find("tp 10");
        assert!(command_id.is_some())
    }
}

#[macro_export]
macro_rules! regex_validator {
    ($ident:ty, $regex:literal) => {
        impl Validator for $ident {
            fn validate<'a,'b>(&self, input: &'a str) -> (bool, &'b str) where 'a : 'b {
                use lazy_static::lazy_static;
                use regex::Regex;
                lazy_static! {
                    static ref RE: Regex = Regex::new($regex).unwrap();
                };
                if let Some(m) = RE.find(input) {
                    let input = &input[m.end()..];
                    (true, input)
                } else {
                    (false, input)
                }
            }
        }
    };
}
