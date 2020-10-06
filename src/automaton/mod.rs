use std::fmt;
use std::hash::Hash;
use std::iter;
use std::mem;
use std::ops::Range;
use std::ops::{Index, IndexMut};

pub mod dfa;
pub mod nfa;
pub mod nfa_to_dfa;
pub mod regex_to_nfa;

pub use dfa::DFA;
pub use nfa::NFA;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct StateId(u32);

impl StateId {
    fn of(id: u32) -> Self {
        assert!(id < u32::MAX);
        StateId(id)
    }

    fn add(&self, n: u32) -> Self {
        StateId::of(self.0 + n)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ByteClassId(u16);

// The standard libray is about to implement a subset of const generics, when this lands
// we could change the Vec<u8>  to [u8: 256] and get Eq, Ord and Hash.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct ByteClass(pub(crate) Vec<u8>);

impl ByteClass {
    pub(crate) fn empty() -> Self {
        ByteClass(vec![0; 256])
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.iter().all(|t| *t == 0)
    }

    pub(crate) fn full() -> Self {
        ByteClass(vec![1; 256])
    }

}


impl From<u8> for ByteClass {
    fn from(value: u8) -> Self {
        let mut values = [0u8; 256];
        values[value as usize] = 1;
        return Self(values.to_vec());
    }
}

impl fmt::Debug for ByteClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}", self.0.to_vec())?;
        Ok(())
    }
}

impl Index<u8> for ByteClass {
    type Output = u8;
    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<u8> for ByteClass {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

pub trait Find<T> {
    fn find(&self, input: &str) -> Result<T, T>;
}

#[cfg(test)]
mod tests {

    use regex_to_nfa::regex_to_nfa;

    use super::*;

    #[test]
    fn byteclass_from_u8() {
        let bc = ByteClass::from(255u8);
        assert!(bc.0.len() == 256);
        assert!(bc.0[255] == 1);

        let bc = ByteClass::from(0u8);
        assert!(bc.0.len() == 256);
        assert!(bc.0[0] == 1);
    }

    #[test]
    fn abc() {
        let nfa = regex_to_nfa("abc").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("").is_err());
        assert!(dfa.find("abc").is_ok());
        assert!(dfa.find("abca").is_err());
        assert!(dfa.find("abd").is_err());
        assert!(dfa.find("add").is_err());
        assert!(dfa.find("ab").is_err());
        assert!(dfa.find("aab").is_err());
        assert!(dfa.find("ddd").is_err());

        let nfa = regex_to_nfa("aa").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("").is_err());
        assert!(dfa.find("a").is_err());
        assert!(dfa.find("aaa").is_err());
        assert!(dfa.find("aa").is_ok());
    }

    #[test]
    fn abc_repeat() {
        let nfa = regex_to_nfa("(abc)*").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("").is_ok());
        assert!(dfa.find("abc").is_ok());
        assert!(dfa.find("abcabc").is_ok());
        assert!(dfa.find("abca").is_err());
        assert!(dfa.find("abcab").is_err());
        assert!(dfa.find("abcabd").is_err());
    }

    #[test]
    fn minimize() {
        let nfa = regex_to_nfa("(abc)*").unwrap();
        let dfa = DFA::from(nfa);
        let _minimized_dfa = dfa.minimize();
    }
}
