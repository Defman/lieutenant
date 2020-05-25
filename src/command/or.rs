use super::{Either, Input, Parser, ParserBase};

#[derive(Clone, Copy, Debug)]
pub struct Or<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<T, U> ParserBase for Or<T, U>
where
    T: Parser,
    U: Parser,
{
    type Extract = (Either<T::Extract, U::Extract>,);

    fn parse<'i>(&self, input: &mut Input<'i>) -> Option<Self::Extract> {
        let first = self.first.parse(&mut input.clone());
        first
            .map(|v| Either::A(v))
            .or_else(|| self.second.parse(input).map(|v| Either::B(v)))
            .map(|e| (e,))
    }
}
