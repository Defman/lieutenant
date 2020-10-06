use super::*;

use crate::graph::{Node, RootNode};
use indexmap::IndexSet;
use regex_to_nfa::regex_to_nfa;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct State {
    pub(crate) table: Vec<Vec<StateId>>,
    pub(crate) class: ByteClassId,
    pub(crate) epsilons: Vec<StateId>,
}

impl Index<u8> for State {
    type Output = Vec<StateId>;
    fn index(&self, index: u8) -> &Self::Output {
        &self.table[index as usize]
    }
}

impl IndexMut<u8> for State {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.table[index as usize]
    }
}

impl State {
    fn empty() -> Self {
        Self {
            ///
            table: vec![vec![]],
            class: ByteClassId(0),
            epsilons: vec![],
        }
    }

    fn push_epsilon(&mut self, to: StateId) {
        self.epsilons.push(to);
    }
}

#[derive(Debug, Clone)]
pub struct NFA {
    // Reduce to a single init state, namely the root
    pub(crate) start: StateId,
    pub(crate) end: StateId,
    /// Represents the nodes in the NFA
    pub(crate) states: Vec<State>,
    /// Sort of represents the edges in the NFA.
    pub(crate) translations: IndexSet<ByteClass>,
}

impl Index<ByteClassId> for NFA {
    type Output = ByteClass;
    fn index(&self, ByteClassId(index): ByteClassId) -> &Self::Output {
        &self.translations[index as usize]
    }
}

impl Index<StateId> for NFA {
    type Output = State;
    fn index(&self, StateId(index): StateId) -> &Self::Output {
        &self.states[index as usize]
    }
}

impl IndexMut<StateId> for NFA {
    fn index_mut(&mut self, StateId(index): StateId) -> &mut Self::Output {
        &mut self.states[index as usize]
    }
}

impl Index<(StateId, Option<u8>)> for NFA {
    type Output = Vec<StateId>;
    fn index(&self, (id, step): (StateId, Option<u8>)) -> &Self::Output {
        let state = &self[id];
        match step {
            Some(b) => &state[self[state.class][b]],
            None => &state.epsilons,
        }
    }
}

impl Index<(StateId, u8)> for NFA {
    type Output = Vec<StateId>;
    fn index(&self, (id, b): (StateId, u8)) -> &Self::Output {
        let state = &self[id];
        &state[self[state.class][b]]
    }
}

impl NFA {
    /// Matches the empty string
    pub(crate) fn empty() -> Self {
        Self {
            start: StateId::of(0),
            states: vec![State::empty()],
            translations: iter::once(ByteClass::empty()).collect(),
            end: StateId::of(0),
        }
    }

    pub(crate) fn literal(lit: &str) -> Self {
        let mut nfa = NFA::empty();
        let end = lit.bytes().fold(nfa.start, |id, c| {
            let next = nfa.push_state();
            let byte_class = ByteClass::from(c);
            nfa.set_transitions(id, byte_class, vec![vec![], vec![next], vec![]]);
            next
        });
        nfa.end = end;
        nfa
    }

    pub(crate) fn single_u8() -> Self {
        let mut nfa = NFA::empty();
        nfa.end = nfa.push_state();
        nfa.set_transitions(
            nfa.start,
            ByteClass::full(),
            vec![vec![], vec![nfa.end], vec![]],
        );
        nfa
    }

    pub(crate) fn push_state(&mut self) -> StateId {
        let id = StateId::of(self.states.len() as u32);
        self.states.push(State::empty());
        id
    }

    // Should return Map<StateId, StateId> can internally be Vec<StateId>
    fn extend(&mut self, other: &Self) -> (StateId, StateId) {
        let offset = self.states.len() as u32;
        let mut states = mem::take(&mut self.states);
        states.extend(other.states.iter().map(|state| {
            let mut state = state.clone();
            state
                .table
                .iter_mut()
                .for_each(|ids| ids.iter_mut().for_each(|id| *id = id.add(offset)));

            state
                .epsilons
                .iter_mut()
                .for_each(|id| *id = id.add(offset));

            let byte_class = &other[state.class];
            let class_id = self.push_class(byte_class.clone());
            state.class = class_id;
            state
        }));

        self.states = states;

        (other.start.add(offset), other.end.add(offset))
    }

    pub(crate) fn repeat(mut self) -> Self {
        let new_start = self.push_state();
        let new_end = self.push_state();
        let old_start = self.start;
        let old_end = self.end;

        self.start = new_start;
        self.end = new_end;

        let new_start = &mut self[new_start];
        new_start.push_epsilon(new_end);
        new_start.push_epsilon(old_start);

        let old_end = &mut self[old_end];
        old_end.push_epsilon(new_end);
        old_end.push_epsilon(old_start);

        self
    }

    pub(crate) fn _not(mut self) -> Self {
        let len = self.states.len();
        let new_end = self.push_state();
        for (index, state) in self.states.iter_mut().enumerate().take(len) {
            let state_id = StateId::of(index as u32);
            if self.end == state_id {
                continue;
            }
            state.push_epsilon(new_end);
        }
        self.end = new_end;
        self
    }

    // TODO: Map<StateId, StateId>
    pub(crate) fn union(mut self, other: &Self) -> Self {
        let new_start = self.push_state();
        let new_end = self.push_state();
        let old_start = self.start;
        let old_end = self.end;

        self.start = new_start;
        self.end = new_end;

        let (other_start, other_end) = self.extend(other);

        let new_start = &mut self[new_start];
        new_start.push_epsilon(old_start);
        new_start.push_epsilon(other_start);

        let old_end = &mut self[old_end];
        old_end.push_epsilon(new_end);

        let other_end = &mut self[other_end];
        other_end.push_epsilon(new_end);

        self
    }

    // TODO: Map<StateId, StateId>
    pub(crate) fn concat(mut self, other: &Self) -> Self {
        let (other_start, other_end) = self.extend(other);
        let end = self.end;
        let end = &mut self[end];
        end.push_epsilon(other_start);
        self.end = other_end;
        self
    }

    pub(crate) fn _optional(mut self) -> Self {
        let start = self.start;
        let end = self.end;
        let start = &mut self[start];
        start.push_epsilon(end);
        self
    }

    // TODO: FIXME
    pub fn find(&self, input: &str) -> Option<StateId> {
        let mut stack = vec![(self.start, input.as_bytes())];
        while let Some((id, input)) = stack.pop() {
            println!("FIND ID -> {:?}", id);
            if let Some(b) = input.first() {
                println!("id:{:?}  b{:?}", self[id], *b);
                stack.extend(self[(id, *b)].iter().map(|id| (*id, &input[1..])));
            } else if self.end == id {
                return Some(id);
            }
            stack.extend(self[id].epsilons.iter().map(|id| (*id, input)));
        }
        None
    }

    pub(crate) fn epsilon_closure(&self, states: BTreeSet<StateId>) -> BTreeSet<StateId> {
        let mut stack: Vec<_> = states.into_iter().collect();
        let mut states = BTreeSet::new();
        while let Some(q) = stack.pop() {
            states.insert(q);
            for q_e in self[q].epsilons.iter() {
                if states.insert(*q_e) {
                    stack.push(*q_e)
                }
            }
        }
        states
    }

    pub(crate) fn go(&self, states: &BTreeSet<StateId>, b: u8) -> BTreeSet<StateId> {
        states
            .iter()
            .copied()
            .flat_map(|id| self[(id, b)].iter().copied())
            .collect()
    }

    pub(crate) fn push_class(&mut self, class: ByteClass) -> ByteClassId {
        if let Some(id) = self.translations.get_index_of(&class) {
            ByteClassId(id as u16)
        } else {
            let id = ByteClassId(self.translations.len() as u16);
            self.translations.insert(class);
            id
        }
    }

    pub(crate) fn set_transitions<I, A>(
        &mut self,
        id: StateId,
        byte_class: ByteClass,
        transitions: I,
    ) where
        I: IntoIterator<Item = A>,
        A: IntoIterator<Item = StateId>,
    {
        let class_id = self.push_class(byte_class);
        self[id].class = class_id;
        self[id].table = transitions
            .into_iter()
            .map(|ids| ids.into_iter().collect())
            .collect();
    }
}

impl From<Range<u8>> for NFA {
    fn from(range: Range<u8>) -> Self {
        let mut buffer = [0; 4];
        let mut classes = vec![ByteClass::empty(); 4];
        for c in range {
            let bytes = char::from(c).encode_utf8(&mut buffer);
            for (i, b) in bytes.bytes().enumerate() {
                if i + 1 < char::from(c).len_utf8() {
                    classes[i][b] = 2;
                } else {
                    classes[i][b] = 1;
                }
            }
        }

        let mut nfa = NFA::empty();
        let mut id = nfa.start;

        let classes: Vec<_> = classes
            .into_iter()
            .take_while(|class| !class.is_empty())
            .collect();

        let end = StateId::of(classes.len() as u32);

        for class in classes {
            let next_id = nfa.push_state();
            if next_id == end {
                nfa.set_transitions(id, class, vec![vec![], vec![end], vec![]])
            } else {
                nfa.set_transitions(id, class, vec![vec![], vec![end], vec![next_id]]);
            }
            id = next_id;
        }

        nfa.end = id;
        nfa
    }
}

// TODO: move
fn from_root<Ctx>(root: &RootNode<Ctx>, node: &Node<Ctx>) -> NFA {
    let mut nfa = NFA::from(&node.kind);

    let mut children = node
        .children
        .iter()
        .map(|node| from_root(root, &root[*node]));

    if let Some(first) = children.next() {
        nfa = nfa.concat(&regex_to_nfa("\\s\\s*").unwrap());
        nfa = nfa.concat(&children.fold(first, |acc, nfa| acc.union(&nfa)));
    }

    nfa
}

//TODO: move
impl<Ctx> From<RootNode<Ctx>> for NFA {
    fn from(root: RootNode<Ctx>) -> Self {
        let mut nfa = NFA::empty();

        let mut children = root
            .children
            .iter()
            .map(|node| from_root(&root, &root[*node]));

        if let Some(first) = children.next() {
            nfa = nfa.concat(&children.fold(first, |acc, nfa| acc.union(&nfa)));
        }

        nfa
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex_to_nfa::regex_to_nfa;
    #[test]
    fn abc() {
        let nfa = regex_to_nfa("abc").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("").is_err());
        assert!(dfa.find("abc").is_ok());
    }

    #[test]
    fn abc_repeat() {
        let nfa = regex_to_nfa("(abc)*").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("").is_ok());
        assert!(dfa.find("abc").is_ok());
        assert!(dfa.find("abcaabc").is_err());
        assert!(dfa.find("a").is_err());
        assert!(dfa.find("abd").is_err());
        assert!(dfa.find("abcd").is_err());
        assert!(dfa.find("abcabc").is_ok());
        assert!(dfa.find("abcabcabc").is_ok());
    }

    #[test]
    fn one_of_abc() {
        let nfa = regex_to_nfa("(aa|bb)").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("aa").is_ok());
        assert!(dfa.find("bb").is_ok());
        assert!(dfa.find("aabb").is_err());
        assert!(dfa.find("aa|bb").is_err());
        assert!(dfa.find("aabb").is_err());
        assert!(dfa.find("").is_err());
    }

    #[test]
    fn literal_star() {
        let nfa = regex_to_nfa("\\*").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("*").is_ok());
        assert!(dfa.find("a").is_err());
    }

    #[test]
    fn dot() {
        let nfa = regex_to_nfa(".").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("").is_err());
        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find(" ").is_ok());
        assert!(dfa.find("\n").is_err());
        assert!(dfa.find("ab").is_err());
        assert!(dfa.find("  ").is_err());
    }

    #[test]
    fn number() {
        let nfa = regex_to_nfa("[0-9]").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("a").is_err());
        assert!(dfa.find("0").is_ok());
        assert!(dfa.find("5").is_ok());
        assert!(dfa.find("9").is_ok());

        let nfa = regex_to_nfa(r"\d").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_err());
        assert!(dfa.find("0").is_ok());
        assert!(dfa.find("5").is_ok());
        assert!(dfa.find("9").is_ok());
    }

    #[test]
    fn abc_range() {
        let nfa = regex_to_nfa("[abc]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("c").is_ok());

        assert!(dfa.find("d").is_err());
        assert!(dfa.find("👺").is_err());
    }

    #[test]
    fn details() {
        let nfa = regex_to_nfa("👺").unwrap();
        //let dfa = DFA::from(nfa);
        assert!(nfa.find("👺").is_some());

        let nfa = regex_to_nfa("[👺]").unwrap();
        //        let dfa = DFA::from(nfa);

        //let a = '👺';
        //println!("{}",a.len_utf8());
        assert!(nfa.find("👺").is_some());
    }

    #[test]
    fn not_abc_range() {
        let nfa = regex_to_nfa("[^abc]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_err());
        assert!(dfa.find("b").is_err());
        assert!(dfa.find("c").is_err());

        assert!(dfa.find("d").is_ok());
        assert!(dfa.find("👺").is_ok());
    }

    #[test]
    fn char_range() {
        let nfa = regex_to_nfa("[a-z]").unwrap();
        let dfa = DFA::from(nfa);

        for c in 'a'..='z' {
            assert!(dfa.find(&c.to_string()).is_ok())
        }

        for c in 'A'..='Z' {
            assert!(dfa.find(&c.to_string()).is_err())
        }

        assert!(dfa.find("æ").is_err());
        assert!(dfa.find("ø").is_err());
        assert!(dfa.find("å").is_err());
    }

    #[test]
    fn range_overlap() {
        let nfa = regex_to_nfa("[a-cb-e]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("c").is_ok());
        assert!(dfa.find("d").is_ok());
        assert!(dfa.find("e").is_ok());
        assert!(dfa.find("f").is_err());
    }

    #[test]
    fn range_disjoint() {
        let nfa = regex_to_nfa("[a-cx-z]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("c").is_ok());
        assert!(dfa.find("x").is_ok());
        assert!(dfa.find("y").is_ok());
        assert!(dfa.find("z").is_ok());
        assert!(dfa.find("9").is_err());
    }

    #[test]
    fn range_concat() {
        let nfa = regex_to_nfa("[a-z][a-z]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("aa").is_ok());
        assert!(dfa.find("bb").is_ok());
        assert!(dfa.find("cc").is_ok());
        assert!(dfa.find("ab").is_ok());
        assert!(dfa.find("ac").is_ok());
        assert!(dfa.find("ag").is_ok());
        assert!(dfa.find("a9").is_err());
        assert!(dfa.find("a").is_err());
        assert!(dfa.find("a ").is_err());
        assert!(dfa.find("aaa").is_err());
    }

    #[test]
    fn word() {
        let nfa = regex_to_nfa(r"\w").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("ww ").is_ok());
        assert!(dfa.find("a").is_ok());
    }

    #[test]
    fn test_union() {
        let nfa1 = regex_to_nfa(r"a").unwrap();
        let nfa2 = regex_to_nfa(r"b").unwrap();
        let nfa = nfa1.union(&nfa2);
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("c").is_err());
        assert!(dfa.find("").is_err());
    }

    //     #[test]
    //     fn abc() {
    //         let nfa_abc = NFA::from(&pattern!("abc"));

    //         assert!(!nfa_abc.find(""));
    //         assert!(nfa_abc.find("abc"));
    //         assert!(!nfa_abc.find("abcabc"));
    //         assert!(!nfa_abc.find("a"));
    //     }

    //     #[test]
    //     fn abc_abc() {
    //         let nfa_abc_abc = NFA::from(&pattern!("abc" "abc"));

    //         assert!(!nfa_abc_abc.find(""));
    //         assert!(!nfa_abc_abc.find("a"));
    //         assert!(!nfa_abc_abc.find("abc"));
    //         assert!(nfa_abc_abc.find("abcabc"));
    //         assert!(!nfa_abc_abc.find("abcabcabc"));
    //     }

    //     #[test]
    //     fn abc_repeat() {
    //         let nfa_abc_repeat = NFA::from(&pattern!("abc"*));

    //         assert!(nfa_abc_repeat.find(""));
    //         assert!(nfa_abc_repeat.find("abc"));
    //         assert!(nfa_abc_repeat.find("abcabc"));
    //         assert!(!nfa_abc_repeat.find("a"));
    //         assert!(!nfa_abc_repeat.find("b"));
    //     }

    //     #[test]
    //     fn abc_abc_repeat() {
    //         let nfa_abc = NFA::from(&pattern!("abc"));

    //         let nfa_abc_abc = nfa_abc.clone().concat(&nfa_abc);
    //         let nfa_abc_abc_repeat = nfa_abc_abc.repeat();

    //         assert!(nfa_abc_abc_repeat.find(""));
    //         assert!(!nfa_abc_abc_repeat.find("abc"));
    //         assert!(nfa_abc_abc_repeat.find("abcabc"));
    //         assert!(!nfa_abc_abc_repeat.find("abcabcabc"));
    //     }

    //     #[test]
    //     fn abc_u_def() {
    //         let nfa_abc = NFA::from(&pattern!("abc"));
    //         let nfa_def = NFA::from(&pattern!("def"));
    //         let nfa_abc_u_def = nfa_abc.union(&nfa_def);

    //         assert!(nfa_abc_u_def.find("abc"));
    //         assert!(nfa_abc_u_def.find("def"));
    //         assert!(!nfa_abc_u_def.find(""));
    //         assert!(!nfa_abc_u_def.find("abcd"));
    //     }

    //     #[test]
    //     fn one_of_abc() {
    //         let nfa_abc = NFA::from(&pattern!(["abc"]));

    //         assert!(!nfa_abc.find(""));
    //         assert!(nfa_abc.find("a"));
    //         assert!(nfa_abc.find("b"));
    //         assert!(nfa_abc.find("c"));
    //         assert!(!nfa_abc.find("aa"));
    //         assert!(!nfa_abc.find("ab"));
    //         assert!(!nfa_abc.find("ac"));
    //     }

    //     #[test]
    //     fn unicode() {
    //         let nfa = NFA::from(&pattern!(["æøå🌏"]));

    //         assert!(!nfa.find(" "));
    //         assert!(!nfa.find(""));
    //         assert!(nfa.find("æ"));
    //         assert!(nfa.find("ø"));
    //         assert!(nfa.find("å"));
    //         assert!(nfa.find("🌏"));
    //         assert!(!nfa.find("a"));
    //         assert!(!nfa.find("b"));
    //         assert!(!nfa.find("c"));
    //     }

    //     #[test]
    //     fn unicode_repeat() {
    //         let nfa = NFA::from(&pattern!(["æøå"]*));

    //         assert!(nfa.find("æå"));
    //         assert!(nfa.find("øæ"));
    //         assert!(nfa.find("åø"));
    //         assert!(!nfa.find("ab"));
    //         assert!(!nfa.find("bc"));
    //         assert!(!nfa.find("cd"));
    //     }

    //     #[test]
    //     fn alt() {
    //         let nfa = NFA::from(&Pattern::alt(&[pattern!("abc"), pattern!("def")]));

    //         assert!(nfa.find("abc"));
    //         assert!(nfa.find("def"));
    //     }

    //     #[test]
    //     fn tp() {
    //         let empty = NFA::empty();
    //         let tp = pattern!("tp");
    //         let tp_alt = NFA::from(&Pattern::alt(&[tp]));
    //         let empty_tp_alt = empty.concat(&tp_alt);
    //     }

    //     #[test]
    //     fn number() {
    //         let digit = NFA::from(&Pattern::one_of("0123456789"));
    //         let digit_many = digit.clone().repeat();
    //         let digit_many_one = digit.concat(&digit_many);

    //         let nfa = digit_many_one;

    //         assert!(nfa.find("0"));
    //         assert!(nfa.find("1"));
    //         assert!(nfa.find("9"));
    //         assert!(nfa.find("99"));
    //         assert!(nfa.find("129385123901238189"));
    //         assert!(!nfa.find("abasdasd"));
    //         assert!(!nfa.find(""));
    //         assert!(!nfa.find(" "));
    //         assert!(!nfa.find("ab123abasd"));
    //         assert!(!nfa.find("123123a"));
    //         assert!(!nfa.find("a123123"));
    //     }

    //     #[test]
    //     fn space() {
    //         let nfa = NFA::from(&Pattern::SPACE_MANY_ONE);

    //         assert!(!nfa.find(""));
    //         assert!(nfa.find(" "));
    //         assert!(nfa.find("  "));
    //         assert!(nfa.find("   "));
    //         assert!(!nfa.find("abc "));
    //         assert!(!nfa.find(" abc"));
    //     }

    //     #[test]
    //     fn integer_space_integer() {
    //         let integer_nfa = NFA::from(&pattern!(["0123456789"]["0123456789"]*));
    //         let space_nfa = NFA::from(&Pattern::SPACE_MANY_ONE);

    //         let integer_space_integer_nfa = integer_nfa.clone().concat(&space_nfa).concat(&integer_nfa);

    //         let nfa = integer_space_integer_nfa;

    //         assert!(nfa.find("10 10"));
    //         assert!(!nfa.find(" 10 10"));
    //         assert!(nfa.find("10    10"));
    //         assert!(!nfa.find("a10 10"));
    //         assert!(!nfa.find("10 10 "));
    //     }
}
