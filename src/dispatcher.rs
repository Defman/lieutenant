use crate::{ArgumentChecker, Command, CommandNode, CommandNodeKind, CommandMeta, Input};
use slab::Slab;
use smallvec::SmallVec;
use std::borrow::Cow;

#[derive(Debug)]
pub enum RegisterError {
    /// Overlapping commands exist: two commands
    /// have an executable node at the same point.
    OverlappingCommands,
    /// Attempted to register an executable command at the root of the command graph.
    ExecutableRoot,
}

#[derive(Copy, Clone, Debug)]
struct NodeKey(usize);

/// Data structure used to dispatch commands.
pub struct CommandDispatcher<C> {
    nodes: Slab<Node<C>>,
    root: NodeKey,
    metas: Vec<CommandMeta>
}

impl<C> Default for CommandDispatcher<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> CommandDispatcher<C> {
    /// Creates a new `CommandDispatcher` with no registered commands.
    pub fn new() -> Self {
        let mut nodes = Slab::new();
        let root = NodeKey(nodes.insert(Node::default()));
        let metas = Vec::new();

        Self { nodes, root, metas }
    }

    /// Registers a command to this `CommandDispatcher`.
    pub fn register(&mut self, command: impl Command<C>) -> Result<(), RegisterError>
    where
        C: 'static,
    {
        self.metas.push(command.meta());
        self.append_node(self.root, command.into_root_node())
    }

    /// Method-chaining function to register a command.
    ///
    /// # Panics
    /// Panics if overlapping commands are detected. Use `register`
    /// to handle this error.
    pub fn with(mut self, command: impl Command<C>) -> Self
    where
        C: 'static,
    {
        self.register(command).unwrap();
        self
    }

    /// Dispatches a command. Returns whether a command was executed.
    ///
    /// Unicode characters are currently not supported. This may be fixed in the future.
    pub fn dispatch(&self, ctx: &mut C, command: &str) -> bool {
        // let parsed = Self::parse_into_arguments(command);

        let mut current_node = self.root;

        let mut input = Input::new(command);

        while !input.empty() {
            // try to find a node satisfying the argument
            let node = &self.nodes[current_node.0];
            
            // TODO: optimize linear search using a hash-array mapped trie
            if let Some((next, next_input)) = node.next.iter().filter_map(|next| {
                let kind = &self.nodes[next.0].kind;
                let mut input = input.clone();

                &input;

                if match kind {
                    NodeKind::Parser(parser) => parser.satisfies(ctx, &mut input),
                    NodeKind::Literal(lit) => lit == input.head(" "),
                    NodeKind::Root => unreachable!("root NodeKind outside the root node?"),
                } {
                    Some((next, input))
                } else {
                    None
                }
            }).next() {
                current_node = *next;
                input = next_input;
            } else {
                return false;
            }
        }

        if let Some(exec) = &self.nodes[current_node.0].exec {
            exec(ctx, command);
            true
        } else {
            false
        }
    }

    pub fn command_meta(&self) -> impl Iterator<Item = &CommandMeta> {
        self.metas.iter()
    }

    fn append_node(
        &mut self,
        dispatcher_current: NodeKey,
        cmd_current: CommandNode<C>,
    ) -> Result<(), RegisterError>
    where
        C: 'static,
    {
        if let Some(exec) = cmd_current.exec {
            let node = &mut self.nodes[dispatcher_current.0];

            if let NodeKind::Root = node.kind {
                return Err(RegisterError::ExecutableRoot);
            }

            match node.exec {
                Some(_) => return Err(RegisterError::OverlappingCommands),
                None => node.exec = Some(exec),
            }
        }

        let cmd_current_kind = &cmd_current.kind;

        // Find a node which has the same parser type as `cmd_current`,
        // or add it if it doesn't exist.
        let found = self.nodes[dispatcher_current.0]
            .next
            .iter()
            .find(|key| &self.nodes[key.0].kind == cmd_current_kind)
            .copied();

        let found = if let Some(found) = found {
            found
        } else {
            // Create new node, then append.
            let new_node = self.nodes.insert(Node::from(cmd_current.kind));

            self.nodes[dispatcher_current.0]
                .next
                .push(NodeKey(new_node));

            NodeKey(new_node)
        };
        cmd_current
            .next
            .into_iter()
            .map(|next| self.append_node(found, next))
            .collect::<Result<(), RegisterError>>()?;

        Ok(())
    }
}

/// Node on the command graph.
struct Node<C> {
    next: SmallVec<[NodeKey; 4]>,
    kind: NodeKind<C>,
    exec: Option<Box<dyn Fn(&mut C, &str)>>,
}

impl<C> Default for Node<C> {
    fn default() -> Self {
        Self {
            next: SmallVec::new(),
            kind: NodeKind::<C>::default(),
            exec: None,
        }
    }
}

impl<C> From<CommandNodeKind<C>> for Node<C> {
    fn from(node: CommandNodeKind<C>) -> Self {
        Node {
            next: SmallVec::new(),
            kind: match node {
                CommandNodeKind::Literal(lit) => NodeKind::Literal(lit),
                CommandNodeKind::Parser(parser) => NodeKind::Parser(parser),
            },
            exec: None,
        }
    }
}

enum NodeKind<C> {
    Literal(Cow<'static, str>),
    Parser(Box<dyn ArgumentChecker<C>>),
    Root,
}

impl<C> PartialEq<CommandNodeKind<C>> for NodeKind<C>
where
    C: 'static,
{
    fn eq(&self, other: &CommandNodeKind<C>) -> bool {
        match (self, other) {
            (NodeKind::Literal(this), CommandNodeKind::Literal(other)) => this.eq(other),
            (NodeKind::Parser(this), CommandNodeKind::Parser(other)) => this.equals(other),
            _ => false,
        }
    }
}

impl<C> Default for NodeKind<C> {
    fn default() -> Self {
        NodeKind::Root
    }
}

#[cfg(test)]
mod tests {
    /*use super::*;
    use bstr::B;
    use smallvec::smallvec;

    #[test]
    fn parse_into_arguments() {
        let test: Vec<(&[u8], SmallVec<[&[u8]; 4]>)> = vec![
            (
                B("test 20 \"this is a string: \\\"Hello world\\\"\""),
                smallvec![B("test"), B("20"), B("this is a string: \"Hello world\"")],
            ),
            (
                B("big inputs cost big programmers with big skills"),
                smallvec![
                    B("big"),
                    B("inputs"),
                    B("cost"),
                    B("big"),
                    B("programmers"),
                    B("with"),
                    B("big"),
                    B("skills"),
                ],
            ),
        ];

        for (input, expected) in test {
            assert_eq!(CommandDispatcher::parse_into_arguments(input), expected);
        }
    }*/
}
