use anyhow::Result;

use crate::generic::Func;

use super::Parser;

pub struct Map<P, F> {
    pub(crate) parser: P,
    pub(crate) map: F,
}

impl<P, F> Parser for Map<P, F>
where
    P: Parser,
    F: Func<P::Extract>,
{
    type Extract = (F::Output,);

    fn parse<'a, 'b>(&self, input: &'a str) -> Result<(Self::Extract, &'b str)>
    where
        'a: 'b,
    {
        let (args, input) = self.parser.parse(input)?;
        Ok(((self.map.call(args),), input))
    }
}
