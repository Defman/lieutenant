mod and;
mod argument;
mod literal;
mod map;
mod optargument;

pub use and::*;
pub use argument::*;
pub use literal::*;
pub use map::*;
pub use optargument::*;

use crate::Result;
use crate::{
    generic::{Func, Tuple},
    AddToDispatcher, Dispatcher, NodeId,
};

pub trait Parser {
    type Extract: Tuple;

    fn parse<'a, 'b>(&self, input: &'a str) -> Result<(Self::Extract, &'b str)>
    where
        'a: 'b;
}

pub struct Command<P, F> {
    pub(crate) parser: P,
    pub(crate) callback: F,
}

impl<P, F> Command<P, F>
where
    P: Parser,
    F: Func<P::Extract>,
{
    pub fn call(&self, mut input: &str) -> Result<F::Output> {
        let (args, _output) = self.parser.parse(&mut input)?;
        Ok(self.callback.call(args))
    }
}

pub trait CommandBuilder: Parser + Sized {
    fn and<T>(self, other: T) -> And<Self, T> {
        And { a: self, b: other }
    }

    fn arg<T>(self) -> And<Self, Argument<T>> {
        self.and(argument())
    }

    fn opt_arg<T>(self) -> And<Self, OptArgument<T>> {
        self.and(opt_argument())
    }

    fn map<F: Func<Self::Extract>>(self, map: F) -> Map<Self, F> {
        Map { parser: self, map }
    }

    fn build<F: Func<Self::Extract>>(self, callback: F) -> Command<Self, F> {
        Command {
            parser: self,
            callback,
        }
    }
}

impl<T> CommandBuilder for T where T: Parser + Sized {}

impl<P, F> AddToDispatcher for Command<P, F>
where
    P: AddToDispatcher,
{
    fn add_to_dispatcher(&self, parent: Option<NodeId>, dispatcher: &mut Dispatcher) -> NodeId {
        let node_id = self.parser.add_to_dispatcher(parent, dispatcher);
        let node = dispatcher.nodes.get_mut(node_id).unwrap();
        node.command = Some(0);
        //TODO make id not always zero
        node_id
    }
}
