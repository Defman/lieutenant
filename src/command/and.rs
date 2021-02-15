use anyhow::bail;

use super::{Parser, Result};
use crate::{
    generic::{Combine, CombinedTuples, Tuple},
    AddToDispatcher, Dispatcher, NodeId,
};

use std::fmt;

pub struct And<A, B> {
    pub(crate) a: A,
    pub(crate) b: B,
}

impl<A, B> Parser for And<A, B>
where
    A: Parser,
    <<A as Parser>::Extract as Tuple>::HList: Combine<<<B as Parser>::Extract as Tuple>::HList>,
    B: Parser,
{
    type Extract = CombinedTuples<A::Extract, B::Extract>;

    #[inline]
    fn parse<'a, 'b>(&self, input: &'a str) -> Result<(Self::Extract, &'b str)>
    where
        'a: 'b,
    {
        let (a, input) = self.a.parse(input)?;

        // Removes at least one space.
        let prev_len = input.len();
        let input = input.trim_start();
        if input.len() == prev_len {
            bail!("Filed in midle of And.")
        }

        let (b, input) = self.b.parse(input)?;

        Ok((a.combine(b), input))
    }
}

impl<A, B> fmt::Display for And<A, B>
where
    A: fmt::Display,
    B: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.a.fmt(f)?;
        write!(f, " ")?;
        self.b.fmt(f)?;
        Ok(())
    }
}

impl<A, B> AddToDispatcher for And<A, B>
where
    A: AddToDispatcher,
    B: AddToDispatcher,
{
    fn add_to_dispatcher(&self, parent: Option<NodeId>, dispatcher: &mut Dispatcher) -> NodeId {
        let parent = self.a.add_to_dispatcher(parent, dispatcher);
        self.b.add_to_dispatcher(Some(parent), dispatcher)
    }
}

#[cfg(test)]
mod tests {
    use crate::command::Literal;

    use super::*;

    #[test]
    fn simple() {
        let lit1 = Literal {
            value: String::from("tp"),
        };

        let lit2 = Literal {
            value: String::from("tp"),
        };

        let x = And { a: lit1, b: lit2 };

        let input = "tp tp a";

        if let Ok((_, output)) = x.parse(input) {
            assert_eq!(output.trim_start(), "a");
        } else {
            assert!(false)
        }
    }

    #[test]
    fn simple2() {
        let lit1 = Literal {
            value: String::from("tp"),
        };

        let lit2 = Literal {
            value: String::from("tp"),
        };

        let x = And { a: lit1, b: lit2 };

        let input = &mut "tp tp";

        if let Ok((_, output)) = x.parse(input) {
            assert_eq!(output.trim_start(), "");
        } else {
            assert!(false)
        }
    }

    #[test]
    fn simple3() {
        let lit1 = Literal {
            value: String::from("tp"),
        };

        let lit2 = Literal {
            value: String::from("tp"),
        };

        let x = And { a: lit1, b: lit2 };

        let input = &mut "tptp";

        assert!(x.parse(input).is_err());
    }
}
