use std::marker::PhantomData;

use crate::{AddToDispatcher, Dispatcher, Node, NodeId, Validator};

use super::{argument, Argument, Parser, Result};

#[derive(Debug)]
pub struct OptArgument<A> {
    pub(crate) argument: PhantomData<A>,
}

impl<A> Clone for OptArgument<A> {
    fn clone(&self) -> Self {
        Self {
            argument: PhantomData::default(),
        }
    }
}

impl<A> Parser for OptArgument<A>
where
    Argument<A>: Parser<Extract = (A,)>,
{
    type Extract = (Option<A>,);

    fn parse<'a, 'b>(&self, input: &'a str) -> Result<(Self::Extract, &'b str)>
    where
        'a: 'b,
    {
        let original = input;
        match argument::<A>().parse(input) {
            Ok(((result,), output)) => Ok(((Some(result),), output)),
            Err(_) => {
                //let input = original;
                Ok(((None,), original))
            }
        }
    }
}

pub fn opt_argument<A>() -> OptArgument<A> {
    OptArgument {
        argument: Default::default(),
    }
}

impl<T: 'static> AddToDispatcher for OptArgument<T>
where
    OptArgument<T>: Validator,
{
    fn add_to_dispatcher(&self, parent: Option<NodeId>, dispatcher: &mut Dispatcher) -> NodeId {
        dispatcher.add(parent, Node::new(self.clone()))
    }
}

impl<T> Validator for OptArgument<T>
where
    Argument<T>: Validator,
{
    fn validate<'a,'b>(&self, input: &'a str) -> (bool, &'b str) where 'a : 'b {
        let (valid, output) = argument::<T>().validate(input);
        if valid {
            (true, output)
        } else {
            (true, input)
        }
    }
}
