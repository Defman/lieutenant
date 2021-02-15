use crate::{AddToDispatcher, Dispatcher, Node, NodeId, Validator};

use super::{Parser, Result};
use anyhow::bail;

use std::fmt;

/// Matches case ...
#[derive(Debug, Clone)]
pub struct Literal {
    pub(crate) value: String,
}

impl Parser for Literal {
    type Extract = ();

    #[inline]
    fn parse<'a, 'b>(&self, input: &'a str) -> Result<(Self::Extract, &'b str)>
    where
        'a: 'b,
    {
        let mut value_lower = self.value.chars().flat_map(|c| c.to_lowercase());

        let x: Option<usize> = input
            .chars()
            .flat_map(|c| c.to_lowercase())
            .zip(&mut value_lower)
            .try_fold(0, |acc, (x, y)| {
                if x == y {
                    Some(acc + x.len_utf8())
                } else {
                    None
                }
            });
        
        if value_lower.next().is_some() {
            // Then the length of input was shorter then the literal. 
            bail!("")
        }

        // If the next char is not a space then the literal did not match. 
        // unless the rest of input is empty.
        //TODO

        if let Some(end) = x {

            match input.chars().into_iter().nth(end) {
                Some(c) => {
                    if !c.is_whitespace() {
                        bail!("Next char was not whitespace after literal.")
                    } 
                }
                None => {}
            }


            Ok(((), &input[end..]))
        } else {
            bail!("")
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)?;
        Ok(())
    }
}

pub fn literal(literal: &str) -> Literal {
    Literal {
        value: literal.to_owned(),
    }
}

impl Validator for Literal {
    fn validate<'a, 'b>(&self, input: &'a str) -> (bool, &'b str) 
    where 'a : 'b {
        match self.parse(input) {
            Ok((_,out)) => {(true, out)}
            Err(_) => (false, input)
        }
    }
}

impl AddToDispatcher for Literal {
    fn add_to_dispatcher(&self, parent: Option<NodeId>, dispatcher: &mut Dispatcher) -> NodeId {
        dispatcher.add(parent, Node::new(self.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let lit = Literal {
            value: String::from("tp"),
        };

        let input = &mut "tp 10 10 10";


        if let Ok((_, output)) = lit.parse(input) {
            assert_eq!(output.trim_start(), "10 10 10");
        } else {
            assert!(false)
        }
    }

    #[test]
    fn empty() {
        let lit = Literal {
            value: String::from("tp"),
        };

        let input = &mut "";
        assert!(lit.parse(input).is_err());
    }

    #[test]
    fn partial() {
        let lit = Literal {
            value: String::from("tp"),
        };

        let input = &mut "tpme";
        assert!(lit.parse(input).is_err());
    }

    #[test]
    fn longer_literal() {
        let lit = Literal {
            value: String::from("tp"),
        };

        let input = &mut "t";
        assert!(lit.parse(input).is_err());
    }
}
