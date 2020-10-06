use std::ops::Range;

use regex_syntax::{hir::ClassUnicodeRange, Parser};

use super::{ByteClass, StateId, NFA};
use anyhow::{bail, Result};

pub(crate) fn regex_to_nfa(regex: &str) -> Result<NFA> {
    let hir = Parser::new().parse(regex)?;
    hir_to_nfa(&hir)
}

impl From<&ClassUnicodeRange> for NFA {
    fn from(range: &ClassUnicodeRange) -> Self {
        let mut start: [u8; 4] = [0; 4];
        range.start().encode_utf8(&mut start);
        let mut end: [u8; 4] = [0; 4];
        range.end().encode_utf8(&mut end);

        let start_len = range.start().len_utf8();
        let end_len = range.end().len_utf8();

        match (start_len, end_len) {
            (1, 1) => {
                //Ascii
                let mut nfa = NFA::empty();
                nfa.end = nfa.push_state();

                let mut transition = [0u8; 256];
                for i in start[0]..=end[0] {
                    transition[i as usize] = 1;
                }
                nfa.set_transitions(
                    nfa.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![nfa.end]],
                );
                return nfa;
            }
            (1, 2) => {
                //match any ascii bigger then or equal to start
                let mut ascii = NFA::empty();
                ascii.end = ascii.push_state();

                let mut transition = [0u8; 256];
                for i in start[0]..128 {
                    transition[i as usize] = 1;
                }
                ascii.set_transitions(
                    ascii.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![ascii.end]],
                );

                //      (2)_
                //     /    \
                // start     end
                //    \     /
                //     (1)-
                let mut byte2 = NFA::empty();
                let states = [byte2.push_state(), byte2.push_state()];
                byte2.end = byte2.push_state();

                //Set transition from start
                let mut transition = [0u8; 256];
                for i in 192..end[0] {
                    transition[i as usize] = 1;
                }
                transition[end[0] as usize] = 2;
                byte2.set_transitions(
                    byte2.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]], vec![states[1]]],
                );

                //Set transition from one to be anything starting with 10xx_xxxx
                transition = [0u8; 256];
                for i in 120..192 {
                    transition[i as usize] = 1;
                }
                byte2.set_transitions(states[0], ByteClass::full(), vec![vec![], vec![byte2.end]]);

                //Set transition from 2 to be any values less then or equal to end[1]
                transition = [0u8; 256];
                for i in 128..=end[1] {
                    transition[i as usize] = 1;
                }
                byte2.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte2.end]],
                );
                return ascii.union(&byte2);
            }
            (1, 3) => {
                //match any ascii bigger then or equal to start
                let mut ascii = NFA::empty();
                ascii.end = ascii.push_state();

                let mut transition = [0u8; 256];
                for i in start[0]..128 {
                    transition[i as usize] = 1;
                }
                ascii.set_transitions(
                    ascii.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![ascii.end]],
                );

                //Match any two byte utf-8
                //
                // start -> 1 -> end
                //
                let mut byte2 = NFA::empty();
                let states = [byte2.push_state()];
                byte2.end = byte2.push_state();

                //start -> 1
                let mut transition = [0u8; 256];
                for i in 192..224 {
                    transition[i] = 1;
                }
                byte2.set_transitions(
                    byte2.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]]],
                );

                // 1 -> end
                transition = [0u8; 256];
                for i in 128..192 {
                    transition[i] = 1;
                }
                byte2.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte2.end]],
                );

                //Match any 3 byte utf-8 less then end
                //
                //         (2) -- (3)
                //       /    \      \
                // start       \      end
                //       \      \   /
                //        (1) - (4)

                // start -> 2    (eq end[0])                              x
                // start -> 1    (lt end[0] and bigger then or equal 224) x
                // 1     -> 4    any in 128..192                          x
                // 2     -> 4    lt end[1]                                x
                // 4     -> end  any in 128..192                          x
                // 2     -> 3    (eq end[1])                              x
                // 3     -> end  (eq or lt end[2])                        x

                let mut byte3 = NFA::empty();
                let states = [
                    byte3.push_state(),
                    byte3.push_state(),
                    byte3.push_state(),
                    byte3.push_state(),
                ];
                byte3.end = byte3.push_state();

                //Start -> 1 (lt end[0])
                //Start -> 2 (eq end[0])
                let mut transition = [0u8; 256];
                transition[end[0] as usize] = 1;
                for i in 224..end[0] {
                    transition[i as usize] = 2;
                }
                byte3.set_transitions(
                    byte3.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[1]], vec![states[0]]],
                );

                //1 -> 4  and 4 -> end  (any valid 2d or 3df byte utf-8, so 10xxxxxx)
                transition = [0u8; 256];
                for i in 128..192 {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[3]]],
                );
                byte3.set_transitions(
                    states[3],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                //2 -> 3 (eq end[1])
                transition = [0u8; 256];
                transition[end[1] as usize] = 1; // 2 -> 3  eq end[1]
                for i in 128..end[1] {
                    transition[i as usize] = 2;
                }
                byte3.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[2]], vec![states[3]]],
                );

                // 3 -> end  (10xxxxxx byte less then end[2])
                transition = [0u8; 256];
                for i in 128..=end[2] {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    states[2],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                return byte3.union(&byte2.union(&ascii));
            }
            (1, 4) => {
                //match any ascii bigger then or equal to start
                let mut ascii = NFA::empty();
                ascii.end = ascii.push_state();

                let mut transition = [0u8; 256];
                for i in start[0]..128 {
                    transition[i as usize] = 1;
                }
                ascii.set_transitions(
                    ascii.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![ascii.end]],
                );

                //Match any two byte utf-8
                //
                // start -> 1 -> end
                //
                let mut byte2 = NFA::empty();
                let states = [byte2.push_state()];
                byte2.end = byte2.push_state();

                //start -> 1
                let mut transition = [0u8; 256];
                for i in 192..224 {
                    transition[i] = 1;
                }
                byte2.set_transitions(
                    byte2.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]]],
                );

                // 1 -> end
                transition = [0u8; 256];
                for i in 128..192 {
                    transition[i] = 1;
                }
                byte2.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte2.end]],
                );

                // Match any 3 byte utf-8
                //
                // start -> 1 -> 2 -> end
                //
                let mut byte3 = NFA::empty();
                let states = [byte3.push_state(), byte3.push_state()];
                byte3.end = byte3.push_state();

                // start -> 1  11100000 -> 11101111   224..240
                let mut transition = [0u8; 256];
                for i in 224..240 {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    byte3.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]]],
                );

                let mut transition = [0u8; 256];
                for i in 128..192 {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[1]]],
                );
                byte3.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                // Match any four byte utf-8 less then end
                //
                //         (2) --- (3) --- (6)
                //       /    \      \       \
                // start       \      \       end
                //       \      \      \    /
                //        (1) - (4) -- (5)-
                //

                // start -> 2    (eq end[0])                               x
                // start -> 1    (lt end[0] and bigger then or equal 224)  x
                // 1     -> 4    any in 128..192
                // 2     -> 4    lt end[1]
                // 2     -> 3    (eq end[1])
                // 3     -> 6    (eq end[2])
                // 6     -> end  (lt eq end[3])
                // 5     -> end  any in 128..192
                // 3     -> 5    (lt end[2])
                // 4     -> 5    any in 128..192

                let mut byte4 = NFA::empty();
                let states = [
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                ];
                byte4.end = byte3.push_state();

                // start -> 2    (eq end[0])                               x
                // start -> 1    (lt end[0] and bigger then or equal 224)  x
                let mut transition = [0u8; 256];
                transition[end[0] as usize] = 1; // eq end[0] -> states[1]
                for n in 240..end[0] {
                    transition[n as usize] = 2; // lt end[0] -> states[0]
                }
                byte4.set_transitions(
                    byte4.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[1]], vec![states[0]]],
                );

                // 2     -> 3    eq end[1]
                // 2     -> 4    lt end[1]
                let mut transition = [0u8; 256];
                transition[end[1] as usize] = 1;
                for i in 128..end[1] {
                    transition[i as usize] = 2;
                }
                byte4.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[2]], vec![states[3]]],
                );

                // 3     -> 6    (eq end[2])
                // 3     -> 5    (lt end[2])
                let mut transition = [0u8; 256];
                transition[end[2] as usize] = 1;
                for i in 128..end[2] {
                    transition[i as usize] = 2;
                }
                byte4.set_transitions(
                    states[2],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[5]], vec![states[4]]],
                );

                // 6     -> end  (lt eq end[3])
                let mut transition = [0u8; 256];
                for i in 128..=end[3] {
                    transition[i as usize] = 1;
                }
                byte4.set_transitions(
                    states[5],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte4.end]],
                );

                // 4     -> 5    any in 128..192
                // 1     -> 4    any in 128..192
                // 5     -> end  any in 128..192
                let mut transition = [0u8; 256];
                for i in 128..=192 {
                    transition[i as usize] = 1;
                }
                byte4.set_transitions(
                    states[3],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[4]]],
                );
                byte4.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[3]]],
                );
                byte4.set_transitions(
                    states[4],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte4.end]],
                );

                return byte4.union(&ascii.union(&byte3.union(&byte2)));
            }
            (2, 1) => unreachable!(),
            (2, 2) => {
                match (start[0] == end[0], start[0] == end[0]) {
                    (false, _) => {
                        //          -1-
                        //        /     \
                        //   start---2--end
                        //        \     /
                        //          -3-

                        // start -> 1  (eq end[0])
                        // start -> 2  start[0]+1..end[0]
                        // start -> 3  (eq start[0])
                        // 1 -> end    (128..=end[1]) lt eq end[1] but valid 2nd byte in utf8
                        // 2 -> end    any  128..192   10xx xxxx
                        // 3 -> end    start[1]..192  gt eq start[1] but valid 2nd byte in utf8

                        let mut byte2 = NFA::empty();
                        let states: [StateId; 3] =
                            [byte2.push_state(), byte2.push_state(), byte2.push_state()];
                        byte2.end = byte2.push_state();

                        // start -> 1  (eq end[0])
                        // start -> 2  start[0]+1..end[0]
                        // start -> 3  (eq start[0])
                        let mut transitions = [0u8; 256];
                        transitions[end[0] as usize] = 1;
                        for i in start[0] + 1..end[0] {
                            transitions[i as usize] = 2;
                        }
                        transitions[start[0] as usize] = 3;
                        byte2.set_transitions(
                            byte2.start,
                            ByteClass(transitions.to_vec()),
                            vec![vec![], vec![states[0]], vec![states[1]], vec![states[2]]],
                        );

                        // 1 -> end    (128..=end[1]) lt eq end[1] but valid 2nd byte in utf8
                        let mut transitions = [0u8; 256];
                        for i in 128..=end[1] {
                            transitions[i as usize] = 1;
                        }
                        byte2.set_transitions(
                            states[0],
                            ByteClass(transitions.to_vec()),
                            vec![vec![], vec![byte2.end]],
                        );

                        // 2 -> end    any  128..192   10xx xxxx
                        let mut transitions = [0u8; 256];
                        for i in 128..192 {
                            transitions[i as usize] = 1;
                        }
                        byte2.set_transitions(
                            states[1],
                            ByteClass(transitions.to_vec()),
                            vec![vec![], vec![byte2.end]],
                        );

                        // 3 -> end    start[1]..192  gt eq start[1] but valid 2nd byte in utf8
                        let mut transitions = [0u8; 256];
                        for i in start[1]..192 {
                            transitions[i as usize] = 1;
                        }
                        byte2.set_transitions(
                            states[2],
                            ByteClass(transitions.to_vec()),
                            vec![vec![], vec![byte2.end]],
                        );

                        return byte2;
                    }
                    (true, false) | (true, true) => {
                        //
                        // start -> 1 -> end;
                        // start -> 1  eq start[0]
                        // 1 -> 2      start[0]..=end[0]

                        let mut byte2 = NFA::empty();
                        let mid = byte2.push_state();
                        byte2.end = byte2.push_state();

                        // start -> 1  eq start[0]
                        let mut transition = [0u8; 256];
                        transition[start[0] as usize] = 1;
                        byte2.set_transitions(
                            byte2.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![mid]],
                        );
                        transition[start[0] as usize] = 0;

                        // 1 -> 2      start[0]..=end[0]
                        for i in start[1]..=end[1] {
                            transition[i as usize] = 1;
                        }
                        byte2.set_transitions(
                            mid,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte2.end]],
                        );

                        return byte2;
                    }
                }
            }
            (2, 3) => {
                //          - 1 -
                //        /       \
                //   start         end
                //        \       /
                //          - 2 -

                // start -> 1   eq start[0]
                // start -> 2   gt start[0]  so start[0]+1..224
                //     2 -> end 128..192
                //     1 -> end gt or eq start[1] and less then 192

                let mut byte2 = NFA::empty();
                let states: [StateId; 2] = [byte2.push_state(), byte2.push_state()];
                byte2.end = byte2.push_state();

                // start -> 1   eq start[0]
                // start -> 2   gt start[0]  so start[0]+1..224
                let mut transition = [0u8; 256];
                transition[start[0] as usize] = 1;
                for i in start[0] + 1..224 {
                    transition[i as usize] = 2;
                }
                byte2.set_transitions(
                    byte2.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]], vec![states[1]]],
                );

                // 2 -> end 128..192
                let mut transition = [0u8; 256];
                for i in 128..192 {
                    transition[i as usize] = 1;
                }
                byte2.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte2.end]],
                );

                // 1 -> end gt or eq start[1] and less then 192
                let mut transition = [0u8; 256];
                for i in start[1]..192 {
                    transition[i as usize] = 1;
                }
                byte2.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte2.end]],
                );

                // Match any 3 byte utf-8 less then end
                //
                //         (2) -- (3)
                //       /    \      \
                // start       \      end
                //       \      \   /
                //        (1) - (4)
                //

                // start -> 2    (eq end[0])                              x
                // start -> 1    (lt end[0] and bigger then or equal 224) x
                // 1     -> 4    any in 128..192                          x
                // 2     -> 4    lt end[1]                                x
                // 4     -> end  any in 128..192                          x
                // 2     -> 3    (eq end[1])                              x
                // 3     -> end  (eq or lt end[2])                        x

                let mut byte3 = NFA::empty();
                let states = [
                    byte3.push_state(),
                    byte3.push_state(),
                    byte3.push_state(),
                    byte3.push_state(),
                ];
                byte3.end = byte3.push_state();

                //Start -> 1 (lt end[0])
                //Start -> 2 (eq end[0])
                let mut transition = [0u8; 256];
                transition[end[0] as usize] = 1;
                for i in 224..end[0] {
                    transition[i as usize] = 2;
                }
                byte3.set_transitions(
                    byte3.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[1]], vec![states[0]]],
                );

                //1 -> 4  and 4 -> end  (any valid 2d or 3df byte utf-8, so 10xxxxxx)
                transition = [0u8; 256];
                for i in 128..192 {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[3]]],
                );
                byte3.set_transitions(
                    states[3],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                //2 -> 3 (eq end[1])
                transition = [0u8; 256];
                transition[end[1] as usize] = 1; // 2 -> 3  eq end[1]
                for i in 128..end[1] {
                    transition[i as usize] = 2;
                }
                byte3.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[2]], vec![states[3]]],
                );

                // 3 -> end  (10xxxxxx byte less then end[2])
                transition = [0u8; 256];
                for i in 128..=end[2] {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    states[2],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                byte2.union(&byte3)
            }
            (2, 4) => {
                //          - 1 -
                //        /       \
                //   start         end
                //        \       /
                //          - 2 -

                // start -> 1   eq start[0]
                // start -> 2   gt start[0]  so start[0]+1..224
                //     2 -> end 128..192
                //     1 -> end gt or eq start[1] and less then 192

                let mut byte2 = NFA::empty();
                let states: [StateId; 2] = [byte2.push_state(), byte2.push_state()];
                byte2.end = byte2.push_state();

                // start -> 1   eq start[0]
                // start -> 2   gt start[0]  so start[0]+1..224
                let mut transition = [0u8; 256];
                transition[start[0] as usize] = 1;
                for i in start[0] + 1..224 {
                    transition[i as usize] = 2;
                }
                byte2.set_transitions(
                    byte2.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]], vec![states[1]]],
                );

                // 2 -> end 128..192
                let mut transition = [0u8; 256];
                for i in 128..192 {
                    transition[i as usize] = 1;
                }
                byte2.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte2.end]],
                );

                // 1 -> end gt or eq start[1] and less then 192
                let mut transition = [0u8; 256];
                for i in start[1]..192 {
                    transition[i as usize] = 1;
                }
                byte2.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte2.end]],
                );

                // Match any 3 byte utf-8
                //
                // start -> 1 -> 2 -> end
                //
                let mut byte3 = NFA::empty();
                let states = [byte3.push_state(), byte3.push_state()];
                byte3.end = byte3.push_state();

                // start -> 1  11100000 -> 11101111   224..240
                let mut transition = [0u8; 256];
                for i in 224..240 {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    byte3.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]]],
                );

                let mut transition = [0u8; 256];
                for i in 128..192 {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[1]]],
                );
                byte3.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                // Match any four byte utf-8 less then end
                //
                //         (2) --- (3) --- (6)
                //       /    \      \       \
                // start       \      \       end
                //       \      \      \    /
                //        (1) - (4) -- (5)-
                //

                // start -> 2    (eq end[0])                               x
                // start -> 1    (lt end[0] and bigger then or equal 224)  x
                // 1     -> 4    any in 128..192
                // 2     -> 4    lt end[1]
                // 2     -> 3    (eq end[1])
                // 3     -> 6    (eq end[2])
                // 6     -> end  (lt eq end[3])
                // 5     -> end  any in 128..192
                // 3     -> 5    (lt end[2])
                // 4     -> 5    any in 128..192

                let mut byte4 = NFA::empty();
                let states = [
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                ];
                byte4.end = byte3.push_state();

                // start -> 2    (eq end[0])                               x
                // start -> 1    (lt end[0] and bigger then or equal 224)  x
                let mut transition = [0u8; 256];
                transition[end[0] as usize] = 1; // eq end[0] -> states[1]
                for n in 240..end[0] {
                    transition[n as usize] = 2; // lt end[0] -> states[0]
                }
                byte4.set_transitions(
                    byte4.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[1]], vec![states[0]]],
                );

                // 2     -> 3    eq end[1]
                // 2     -> 4    lt end[1]
                let mut transition = [0u8; 256];
                transition[end[1] as usize] = 1;
                for i in 128..end[1] {
                    transition[i as usize] = 2;
                }
                byte4.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[2]], vec![states[3]]],
                );

                // 3     -> 6    (eq end[2])
                // 3     -> 5    (lt end[2])
                let mut transition = [0u8; 256];
                transition[end[2] as usize] = 1;
                for i in 128..end[2] {
                    transition[i as usize] = 2;
                }
                byte4.set_transitions(
                    states[2],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[5]], vec![states[4]]],
                );

                // 6     -> end  (lt eq end[3])
                let mut transition = [0u8; 256];
                for i in 128..=end[3] {
                    transition[i as usize] = 1;
                }
                byte4.set_transitions(
                    states[5],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte4.end]],
                );

                // 4     -> 5    any in 128..192
                // 1     -> 4    any in 128..192
                // 5     -> end  any in 128..192
                let mut transition = [0u8; 256];
                for i in 128..=192 {
                    transition[i as usize] = 1;
                }
                byte4.set_transitions(
                    states[3],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[4]]],
                );
                byte4.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[3]]],
                );
                byte4.set_transitions(
                    states[4],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte4.end]],
                );

                return byte2.union(&byte3.union(&byte4));
            }
            (3, 1) => unreachable!(),
            (3, 2) => unreachable!(),
            (3, 3) => {
                match (start[0] == end[0], start[1] == end[1], start[2] == end[2]) {
                    (false, _, _) => {
                        // Match three byte utf-8
                        //
                        //      ---(1)------ (4)-
                        //     /          \      \
                        // start-----2-----5-----end
                        //     \          /      /
                        //      ---(3)---- --(6)-
                        //

                        // start -> 1  eq end[0]
                        // start -> 2  gt start[0] lt end[0]
                        // start -> 3  eq start[0]
                        // 3 -> 6      eq start[1]
                        // 3 -> 5      start[1]+1..192
                        // 2 -> 5      128..192
                        // 5 -> end    128..192
                        // 1 -> 4      eq end[1]
                        // 1 -> 5      128..end[1]
                        // 4 -> end    128..=end[2]
                        // 6 -> end    start[2]..192

                        let mut byte3 = NFA::empty();
                        let states: [StateId; 6] = [
                            byte3.push_state(),
                            byte3.push_state(),
                            byte3.push_state(),
                            byte3.push_state(),
                            byte3.push_state(),
                            byte3.push_state(),
                        ];
                        byte3.end = byte3.push_state();

                        // start -> 1  eq end[0]
                        // start -> 2  gt start[0] lt end[0]
                        // start -> 3  eq start[0]

                        let mut transition = [0u8; 256];
                        transition[end[0] as usize] = 1;
                        for i in start[0] + 1..end[0] {
                            transition[i as usize] = 2;
                        }
                        transition[start[0] as usize] = 3;
                        byte3.set_transitions(
                            byte3.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[0]], vec![states[1]], vec![states[2]]],
                        );

                        // 3 -> 6      eq start[1]
                        // 3 -> 5      start[1]+1..192
                        let mut transition = [0u8; 256];
                        transition[start[1] as usize] = 1;
                        for i in start[1] + 1..192 {
                            transition[i as usize] = 2;
                        }
                        byte3.set_transitions(
                            states[2],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[5]], vec![states[4]]],
                        );

                        // 2 -> 5      128..192
                        // 5 -> end    128..192
                        let mut transition = [0u8; 256];
                        for i in 128..192 {
                            transition[i as usize] = 1;
                        }
                        byte3.set_transitions(
                            states[1],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[4]]],
                        );
                        byte3.set_transitions(
                            states[4],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte3.end]],
                        );

                        // 1 -> 4      eq end[1]
                        // 1 -> 5      128..end[1]
                        let mut transition = [0u8; 256];
                        transition[end[1] as usize] = 1;
                        for i in 128..end[1] {
                            transition[i as usize] = 2;
                        }
                        byte3.set_transitions(
                            states[0],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[3]], vec![states[4]]],
                        );

                        // 4 -> end    128..=end[2]
                        let mut transition = [0u8; 256];
                        for i in 128..=end[2] {
                            transition[i as usize] = 1;
                        }
                        byte3.set_transitions(
                            states[3],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte3.end]],
                        );

                        // 6 -> end    start[2]..192
                        let mut transition = [0u8; 256];
                        for i in start[2]..192 {
                            transition[i as usize] = 1;
                        }
                        byte3.set_transitions(
                            states[5],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte3.end]],
                        );

                        return byte3;
                    }

                    (true, false, _) => {
                        // Match three byte utf-8
                        //
                        //               -(2)-
                        //             /       \
                        // start-----1-----3---end
                        //             \      /
                        //              -(4)-
                        //

                        // start -> 1  eq start[0] assert start[0] == end[0]

                        // 1 -> 2      eq end[1]
                        // 1 -> 3      start[1]+1..end[1]
                        // 1 -> 4      eq start[1]

                        // 2 -> end    128..=end[2]
                        // 3 -> end    128..192
                        // 4 -> end    start[2]..192

                        let mut byte3 = NFA::empty();
                        let states: [StateId; 4] = [
                            byte3.push_state(),
                            byte3.push_state(),
                            byte3.push_state(),
                            byte3.push_state(),
                        ];
                        byte3.end = byte3.push_state();

                        // start -> 1  eq start[0] assert start[0] == end[0]
                        let mut transition = [0u8; 256];
                        transition[start[0] as usize] = 1;
                        byte3.set_transitions(
                            byte3.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[0]]],
                        );

                        // 1 -> 2      eq end[1]
                        // 1 -> 3      start[1]+1..end[1]
                        // 1 -> 4      eq start[1]
                        let mut transition = [0u8; 256];
                        transition[end[1] as usize] = 1;
                        for i in start[1] + 1..end[1] {
                            transition[i as usize] = 2;
                        }
                        transition[start[1] as usize] = 3;
                        byte3.set_transitions(
                            states[0],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[1]], vec![states[2]], vec![states[3]]],
                        );

                        // 2 -> end    128..=end[2]
                        let mut transition = [0u8; 256];
                        for i in 128..=end[2] {
                            transition[i as usize] = 1;
                        }
                        byte3.set_transitions(
                            states[1],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte3.end]],
                        );

                        // 3 -> end    128..192
                        let mut transition = [0u8; 256];
                        for i in 128..=192 {
                            transition[i as usize] = 1;
                        }
                        byte3.set_transitions(
                            states[2],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte3.end]],
                        );

                        // 4 -> end    start[2]..192
                        let mut transition = [0u8; 256];
                        for i in start[2]..192 {
                            transition[i as usize] = 1;
                        }
                        byte3.set_transitions(
                            states[3],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte3.end]],
                        );

                        return byte3;
                    }

                    (true, true, _) => {
                        // start -> 1 -> 2 -> end

                        // start -> 1    eq start[0]  assert start[0] == end[0]
                        //     1 -> 2    start[1]  assert start[1] == end[1]
                        //     2 -> end  start[2]..=end[2]

                        let mut byte3 = NFA::empty();
                        let states: [StateId; 2] = [byte3.push_state(), byte3.push_state()];
                        byte3.end = byte3.push_state();

                        // start -> 1    eq start[0]  assert start[0] == end[0]
                        let mut transition = [0u8; 256];
                        transition[start[0] as usize] = 1;
                        debug_assert!(start[0] == end[0]);
                        byte3.set_transitions(
                            byte3.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[0]]],
                        );

                        // 1 -> 2    eq start[1]  assert start[1] == end[1]
                        let mut transition = [0u8; 256];
                        transition[start[1] as usize] = 1;
                        debug_assert!(start[1] == end[1]);
                        byte3.set_transitions(
                            states[0],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[1]]],
                        );

                        //     2 -> end  start[2]..=end[2]
                        let mut transition = [0u8; 256];
                        for i in start[2]..=end[2] {
                            transition[i as usize] = 1;
                        }
                        byte3.set_transitions(
                            states[1],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte3.end]],
                        );

                        return byte3;
                    }
                }
            }
            (3, 4) => {
                //      --1--3--
                //     /    \   \
                // start--2---4--end
                //

                // start -> 1     eq start[0]
                // start -> 2     gt start[0] so start[0]+1..240
                //     1 -> 3     eq start[1]
                //     1 -> 4     start[1]+1..192
                //     3 -> end   start[2]..192
                //     2 -> 4     any in 128..192
                //     4 -> end   any in 128..192
                let mut byte3 = NFA::empty();
                let states: [StateId; 4] = [
                    byte3.push_state(),
                    byte3.push_state(),
                    byte3.push_state(),
                    byte3.push_state(),
                ];
                byte3.end = byte3.push_state();

                // start -> 1     eq start[0]
                // start -> 2     gt start[0] so start[0]+1..240
                let mut transition = [0u8; 256];
                transition[start[0] as usize] = 1;
                for i in start[0] + 1..240 {
                    transition[i as usize] = 2;
                }
                byte3.set_transitions(
                    byte3.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[0]], vec![states[1]]],
                );

                //     1 -> 3     eq start[1]
                //     1 -> 4     start[1]+1..192
                let mut transition = [0u8; 256];
                transition[start[1] as usize] = 1;
                for i in start[1] + 1..192 {
                    transition[i as usize] = 2;
                }
                byte3.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[2]], vec![states[3]]],
                );

                //     3 -> end   start[2]..192
                let mut transition = [0u8; 256];
                for i in start[2]..192 {
                    transition[i as usize] = 1;
                }
                byte3.set_transitions(
                    states[2],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                //     2 -> 4     any in 128..192
                //     4 -> end   any in 128..192
                let mut transition = [0u8; 256];
                for i in 128..192 {
                    transition[i] = 1;
                }
                byte3.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[3]]],
                );
                byte3.set_transitions(
                    states[3],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte3.end]],
                );

                //
                //        ------4 --- 5 --- 6-
                //       /        \     \     \
                // start --- 1 --- 2 --- 3 --- end

                // start -> 4    eq end[0]
                // start -> 1    128..end[0]
                // 4     -> 5    eq end[1]
                // 4     -> 2    128..end[1]𞗈
                // 3 -> end      128..192

                let mut byte4 = NFA::empty();
                let states: [StateId; 6] = [
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                    byte4.push_state(),
                ];
                byte4.end = byte4.push_state();

                // start -> 4    eq end[0]
                // start -> 1    128..end[0]
                let mut transition = [0u8; 256];
                transition[end[0] as usize] = 1;
                for i in 128..end[0] {
                    transition[i as usize] = 2;
                }
                byte4.set_transitions(
                    byte4.start,
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[3]], vec![states[0]]],
                );

                // 4     -> 5    eq end[1]
                // 4     -> 2    128..end[1]
                let mut transition = [0u8; 256];
                transition[end[1] as usize] = 1;
                for i in 128..end[1] {
                    transition[i as usize] = 2;
                }
                byte4.set_transitions(
                    states[3],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[4]], vec![states[1]]],
                );

                // 5     -> 6    eq end[2]
                // 5     -> 3    128..end[2]
                let mut transition = [0u8; 256];
                transition[end[2] as usize] = 1;
                for i in 128..end[2] {
                    transition[i as usize] = 2;
                }
                byte4.set_transitions(
                    states[4],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[5]], vec![states[2]]],
                );

                // 6     -> end  128..=end[3]
                let mut transition = [0u8; 256];
                for i in 128..=end[3] {
                    transition[i as usize] = 1;
                }
                byte4.set_transitions(
                    states[5],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte4.end]],
                );

                // 1 -> 2        128..192
                // 2 -> 3        128..192
                // 3 -> end      128..192
                let mut transition = [0u8; 256];
                for i in 128..192 {
                    transition[i as usize] = 1;
                }
                byte4.set_transitions(
                    states[0],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[1]]],
                );
                byte4.set_transitions(
                    states[1],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![states[2]]],
                );
                byte4.set_transitions(
                    states[2],
                    ByteClass(transition.to_vec()),
                    vec![vec![], vec![byte4.end]],
                );

                return byte3.union(&byte4);
            }
            (4, 4) => {
                match (
                    start[0] == end[0],
                    start[1] == end[1],
                    start[2] == end[2],
                    start[3] == end[3],
                ) {
                    (true, true, true, _) => {
                        // start -> 1 -> 2 -> 3 -> end

                        // start->1  eq start[0]
                        // 1->2      eq start[1]
                        // 2->3      eq start[2]
                        // 3->end      start[3]..=end[3]

                        let mut byte4 = NFA::empty();
                        let states: [StateId; 3] =
                            [byte4.push_state(), byte4.push_state(), byte4.push_state()];
                        byte4.end = byte4.push_state();

                        // start->1  eq start[0]
                        let mut transition = [0u8; 256];
                        transition[start[0] as usize] = 1;
                        byte4.set_transitions(
                            byte4.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[0]]],
                        );
                        transition[start[0] as usize] = 0;

                        // 1->2      eq start[1]
                        transition[start[1] as usize] = 1;
                        byte4.set_transitions(
                            states[0],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[1]]],
                        );
                        transition[start[1] as usize] = 0;

                        // 2->3      eq start[2]
                        transition[start[2] as usize] = 1;
                        byte4.set_transitions(
                            states[1],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[2]]],
                        );
                        transition[start[2] as usize] = 0;

                        // 3->end      start[3]..=end[3]
                        for i in start[3]..=end[3] {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[2],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );
                        return byte4;
                    }
                    (true, true, false, _) => {
                        //                 - 3 --
                        //                /      \
                        // start -> 1 -> 2 - 4---end
                        //               \       /
                        //                - 5 --

                        // start -> 1   eq start[0]
                        //     1 -> 2   eq start[1]
                        //     2 -> 3   eq end[2]
                        //     2 -> 4   start[2]+1..end[2]
                        //     2 -> 5   eq start[2]
                        //     3 -> end 128..=end[3]
                        //     4 -> end 128..192
                        //     5 -> end start[3]..192

                        let mut byte4 = NFA::empty();
                        let states: [StateId; 5] = [
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                        ];
                        byte4.end = byte4.push_state();

                        // start -> 1   eq start[0]
                        let mut transition = [0u8; 256];
                        transition[start[0] as usize] = 1;
                        byte4.set_transitions(
                            byte4.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[0]]],
                        );
                        transition[start[0] as usize] = 0;

                        //     1 -> 2   eq start[1]
                        transition[start[1] as usize] = 1;
                        byte4.set_transitions(
                            states[0],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[1]]],
                        );
                        transition[start[1] as usize] = 0;

                        //     2 -> 3   eq end[2]
                        //     2 -> 4   start[2]+1..end[2]
                        //     2 -> 5   eq start[2]
                        transition[end[2] as usize] = 1;
                        for i in start[2] + 1..end[2] {
                            transition[i as usize] = 2;
                        }
                        transition[start[2] as usize] = 3;
                        byte4.set_transitions(
                            states[1],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[2]], vec![states[3]], vec![states[4]]],
                        );

                        //     3 -> end 128..=end[3]
                        let mut transition = [0u8; 256];
                        for i in 128..=end[3] {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[2],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        //     4 -> end 128..192
                        let mut transition = [0u8; 256];
                        for i in 128..192 {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[3],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        //     5 -> end start[3]..192
                        let mut transition = [0u8; 256];
                        for i in start[3]..192 {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[4],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        return byte4;
                    }
                    (true, false, _, _) => {
                        //            2 ---- 4 ----
                        //           /    \        \
                        // start -> 1->7->5 - ----end
                        //          \    /         /
                        //           3 ---- 6 ----

                        // start -> 1    eq start[0]
                        //     1 -> 2    eq start[1]
                        //     1 -> 7    start[1]+1..end[1]
                        //     1 -> 3    eq end[1]
                        //     2 -> 4    eq start[2]
                        //     2 -> 5    start[2]+1..192
                        //     3 -> 5    128..end[2]
                        //     3 -> 6    eq end[2]
                        //     4 -> end  start[3]..192
                        //     7 -> 5    128..192
                        //     5 -> end  128..192
                        //     6 -> end  128..=end[3]

                        let mut byte4 = NFA::empty();
                        let states: [StateId; 7] = [
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                        ];
                        byte4.end = byte4.push_state();

                        // start -> 1    eq start[0]
                        let mut transition = [0u8; 256];
                        transition[start[0] as usize] = 1;
                        byte4.set_transitions(
                            byte4.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[0]]],
                        );
                        transition[start[0] as usize] = 0;

                        //     1 -> 2    eq start[1]
                        //     1 -> 7    start[1]+1..end[1]
                        //     1 -> 3    eq end[1]
                        transition[start[1] as usize] = 1;
                        for i in start[1] + 1..end[1] {
                            transition[i as usize] = 2;
                        }
                        transition[end[1] as usize] = 3;
                        byte4.set_transitions(
                            states[0],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[1]], vec![states[6]], vec![states[2]]],
                        );

                        //     2 -> 4    eq start[2]
                        //     2 -> 5    start[2]+1..192
                        let mut transition = [0u8; 256];
                        transition[start[2] as usize] = 1;
                        for i in start[2] + 1..192 {
                            transition[i as usize] = 2;
                        }
                        byte4.set_transitions(
                            states[1],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[3]], vec![states[4]]],
                        );

                        //     3 -> 5    128..end[2]
                        //     3 -> 6    eq end[2]
                        let mut transition = [0u8; 256];
                        for i in 128..end[2] {
                            transition[i as usize] = 1;
                        }
                        transition[end[2] as usize] = 2;
                        byte4.set_transitions(
                            states[2],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[4]], vec![states[5]]],
                        );

                        //     4 -> end  start[3]..192
                        let mut transition = [0u8; 256];
                        for i in start[3]..192 {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[3],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        //     7  -> 5   128..192
                        //     5 -> end  128..192
                        let mut transition = [0u8; 256];
                        for i in 128..192 {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[4],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );
                        byte4.set_transitions(
                            states[6],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[4]]],
                        );

                        //     6 -> end  128..=end[3]
                        let mut transition = [0u8; 256];
                        for i in 128..=end[3] {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[5],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        return byte4;
                    }
                    (false, _, _, _) => {
                        // Match three byte utf-8
                        //
                        //      ---(1)------ (4)-----(8)
                        //     /          \      \     \
                        // start-----2-----5-----7----end
                        //     \          /      /     /
                        //      ---(3)---- --(6)----(9)
                        //

                        let mut byte4 = NFA::empty();
                        let states: [StateId; 9] = [
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                            byte4.push_state(),
                        ];
                        byte4.end = byte4.push_state();

                        // start -> 1  eq end[0]
                        // start -> 2  gt start[0]+1 lt end[0]
                        // start -> 3  eq start[0]
                        let mut transition = [0u8; 256];
                        transition[end[0] as usize] = 1;
                        for i in start[0] + 1..end[0] {
                            transition[i as usize] = 2;
                        }
                        transition[start[0] as usize] = 3;
                        byte4.set_transitions(
                            byte4.start,
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[0]], vec![states[1]], vec![states[2]]],
                        );

                        // 3 -> 6      eq start[1]
                        // 3 -> 5      start[1]+1..192
                        let mut transition = [0u8; 256];
                        transition[start[1] as usize] = 1;
                        for i in start[1] + 1..192 {
                            transition[i as usize] = 2;
                        }
                        byte4.set_transitions(
                            states[2],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[5]], vec![states[4]]],
                        );

                        // 1 -> 4      eq end[1]
                        // 1 -> 5      128..end[1]
                        let mut transition = [0u8; 256];
                        transition[end[1] as usize] = 1;
                        for i in 128..end[1] {
                            transition[i as usize] = 2;
                        }
                        byte4.set_transitions(
                            states[0],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[3]], vec![states[4]]],
                        );

                        // 4 -> 8      eq end[2]
                        // 4 -> 7      128..end[2]
                        let mut transition = [0u8; 256];
                        transition[end[2] as usize] = 1;
                        for i in 128..end[2] {
                            transition[i as usize] = 2;
                        }
                        byte4.set_transitions(
                            states[3],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[7]], vec![states[6]]],
                        );

                        // 2 -> 5      128..192
                        // 5 -> 7      128..192
                        // 7 -> end    128..192
                        let mut transition = [0u8; 256];
                        for i in 128..192 {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[1],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[4]]],
                        );
                        byte4.set_transitions(
                            states[4],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[6]]],
                        );
                        byte4.set_transitions(
                            states[6],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        // 6 -> 9      eq start[2]
                        // 6 -> 7      start[2]+1..192
                        let mut transition = [0u8; 256];
                        transition[start[2] as usize] = 1;
                        for i in start[2] + 1..192 {
                            transition[i as usize] = 2;
                        }
                        byte4.set_transitions(
                            states[5],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![states[8]], vec![states[6]]],
                        );

                        // 9 -> end    start[3]..192
                        let mut transition = [0u8; 256];
                        for i in start[3]..192 {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[8],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        // 8 -> end    128..=end[3]
                        let mut transition = [0u8; 256];
                        for i in 128..=end[3] {
                            transition[i as usize] = 1;
                        }
                        byte4.set_transitions(
                            states[7],
                            ByteClass(transition.to_vec()),
                            vec![vec![], vec![byte4.end]],
                        );

                        return byte4;
                    }
                }
            }

            _ => unreachable!(),
        }
    }
}

fn hir_to_nfa(hir: &regex_syntax::hir::Hir) -> Result<NFA> {
    match hir.kind() {
        regex_syntax::hir::HirKind::Empty => Ok(NFA::single_u8()),
        regex_syntax::hir::HirKind::Literal(lit) => match lit {
            regex_syntax::hir::Literal::Unicode(uni) => Ok(NFA::literal(&uni.to_string())),
            regex_syntax::hir::Literal::Byte(byte) => Ok(NFA::literal(&byte.to_string())),
        },
        regex_syntax::hir::HirKind::Class(class) => {
            match class {
                regex_syntax::hir::Class::Unicode(uni) => {


                    let mut itr = uni.ranges().iter();

                    //Always has atleast one value
                    let mut nfa = NFA::from(itr.next().unwrap());
                    while itr.len() > 0 {
                        nfa = nfa.union(&NFA::from(itr.next().unwrap()));
                    }

                    Ok(nfa)
                }
                regex_syntax::hir::Class::Bytes(byte) => {

                    let mut itr = byte.ranges().iter();                        
                    let range = itr.next().unwrap();
                    let mut nfa = NFA::from(
                        Range {
                            start: range.start(),
                            end: range.end(),
                        });

                    while itr.len() > 0 {
                        let range = itr.next().unwrap();
                         nfa = nfa.union(&NFA::from(Range {
                            start: range.start(),
                            end: range.end(),
                        }));
                    }

                    Ok(nfa)
                }
            }
        }
        regex_syntax::hir::HirKind::Anchor(x) => match x {
            regex_syntax::hir::Anchor::StartLine => bail!("We dont suport StartLine symbols!"),
            regex_syntax::hir::Anchor::EndLine => bail!("We dont suport EndLine symbols!"),
            regex_syntax::hir::Anchor::StartText => bail!("We dont suport StartText symbol!"),
            regex_syntax::hir::Anchor::EndText => bail!("We dont suport EndText symbol!"),
        },
        regex_syntax::hir::HirKind::WordBoundary(boundary) => {
            match boundary {
                regex_syntax::hir::WordBoundary::Unicode => {
                    todo!() // I dont know if we need to suport this
                }
                regex_syntax::hir::WordBoundary::UnicodeNegate => {
                    todo!() // I dont know if we need to suport this
                }
                regex_syntax::hir::WordBoundary::Ascii => {
                    todo!() // I dont know if we need to suport this
                }
                regex_syntax::hir::WordBoundary::AsciiNegate => {
                    todo!() // I dont know if we need to suport this
                }
            }
        }
        regex_syntax::hir::HirKind::Repetition(x) => {
            if x.greedy {
                let nfa = hir_to_nfa(&x.hir)?;
                Ok(nfa.repeat())
            } else {
                bail!("We dont suport non greedy patterns")
            }
        }
        regex_syntax::hir::HirKind::Group(group) => {
            //TODO i dont know how we are suposed to interprite an empty
            //hir/nfa in this case. Should it maybe be a no-op?
            hir_to_nfa(&group.hir)
        }
        regex_syntax::hir::HirKind::Concat(cats) => {
            let mut nfas = cats.iter().map(|hir| hir_to_nfa(hir));
            let mut fst = nfas.next().unwrap()?;
            for nfa in nfas {
                fst = fst.concat(&nfa?);
            }
            Ok(fst)
        }

        regex_syntax::hir::HirKind::Alternation(alts) => {
            let mut nfas = alts.iter().map(|hir| hir_to_nfa(hir));
            let mut fst = nfas.next().unwrap()?;
            for nfa in nfas {
                fst = fst.union(&nfa?);
            }
            Ok(fst)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::{Find, DFA};

    fn is_between(from: char, x: char, to: char) -> bool {
        // let mut bot = [0u8;4];
        // from.encode_utf8(&mut bot);
        // let mut mid = [0u8;4];
        // x.encode_utf8(&mut mid);
        // let mut top = [0u8;4];
        // to.encode_utf8(&mut top);

        // if from.len_utf8() < x.len_utf8() && x.len_utf8() < to.len_utf8() {
        //     return  true;
        // }

        return (from as u32) <= (x as u32) && (x as u32) <= (to as u32);
    }

    #[test]
    fn abc() {
        let nfa = regex_to_nfa("abc").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("").is_err());
        assert!(dfa.find("abc").is_ok());
    }

    #[test]
    fn simple() {
        let nfa = regex_to_nfa("[a]").unwrap();
        assert!(nfa.start != nfa.end);

        let dfa = DFA::from(nfa);
        assert!(dfa.find(" ").is_err());
        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("").is_err());
    }

    #[test]
    fn range11() {
        let nfa = regex_to_nfa("[a-z]").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("`").is_err());
        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("z").is_ok());
        assert!(dfa.find("{").is_err());
        assert!(dfa.find("").is_err());
        assert!(dfa.find(" ").is_err());
    }

    #[test]
    fn range12() {
        let nfa = regex_to_nfa("[r-¬]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("q").is_err());
        assert!(dfa.find("r").is_ok());
        assert!(dfa.find("s").is_ok());
        assert!(dfa.find("x").is_ok());
        assert!(dfa.find("«").is_ok());
        assert!(dfa.find("ª").is_ok());
        assert!(dfa.find("¬").is_ok());
        assert!(dfa.find("®").is_err());
    }

    #[test]
    fn range13() {
        let nfa = regex_to_nfa("[b-ॐ]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_err());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("c").is_ok());
        assert!(dfa.find("¬").is_ok());
        assert!(dfa.find("¥").is_ok());
        assert!(dfa.find("ए").is_ok());
        assert!(dfa.find("ॏ").is_ok());
        assert!(dfa.find("ॐ").is_ok());
        assert!(dfa.find("॑").is_err());

        let nfa = regex_to_nfa("[b-ᥑ]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_err());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("c").is_ok());
        assert!(dfa.find("¬").is_ok());
        assert!(dfa.find("¥").is_ok());
        assert!(dfa.find("ए").is_ok());
        assert!(dfa.find("ॏ").is_ok());
        assert!(dfa.find("ॐ").is_ok());
        assert!(dfa.find("॑").is_ok());

        assert!(dfa.find("ᥑ").is_ok());
        assert!(dfa.find("ᥒ").is_err());
    }

    #[test]
    fn range14() {
        let nfa = regex_to_nfa("[b-🤡]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_err());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("c").is_ok());
        assert!(dfa.find("¬").is_ok());
        assert!(dfa.find("¥").is_ok());
        assert!(dfa.find("ए").is_ok());
        assert!(dfa.find("ॏ").is_ok());
        assert!(dfa.find("ॐ").is_ok());
        assert!(dfa.find("॑").is_ok());
        assert!(dfa.find("𞨨").is_ok());
        assert!(dfa.find("🣧").is_ok());
        assert!(dfa.find("🐖").is_ok());
        assert!(dfa.find("𞤡").is_ok());
        assert!(dfa.find("🤠").is_ok());
        assert!(dfa.find("🤡").is_ok());
        assert!(dfa.find("🤢").is_err());

        assert!(dfa.find("ॉ").is_ok());
        assert!(dfa.find("ऽ").is_ok());
        assert!(dfa.find("ऴ").is_ok());
        assert!(dfa.find("ॐ").is_ok());
        assert!(dfa.find("ख़").is_ok());

        assert!(dfa.find("🤖").is_ok());
        assert!(dfa.find("🟣").is_ok());
        assert!(dfa.find("🗨").is_ok());
        assert!(dfa.find("👵").is_ok());
        assert!(dfa.find("🎃").is_ok());
        assert!(dfa.find("🌶").is_ok());
        assert!(dfa.find("🌐").is_ok());
        assert!(dfa.find("䒗").is_ok());
        assert!(dfa.find("").is_err());
        assert!(dfa.find("ϲ").is_ok());
        assert!(dfa.find("ϫ").is_ok());
        assert!(dfa.find("ζ").is_ok());
        assert!(dfa.find("Θ").is_ok());

        assert!(dfa.find("#").is_err());
        assert!(dfa.find("1").is_err());
        assert!(dfa.find("8").is_err());
        assert!(dfa.find("`").is_err());
    }

    #[test]
    fn range22() {
        let nfa = regex_to_nfa("[Ð-Ð]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("Ï").is_err());
        assert!(dfa.find("Ð").is_ok());
        assert!(dfa.find("Ñ").is_err());

        let nfa = regex_to_nfa("[Ð-Ñ]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("Ï").is_err());
        assert!(dfa.find("Ð").is_ok());
        assert!(dfa.find("Ñ").is_ok());
        assert!(dfa.find("Ò").is_err());

        let nfa = regex_to_nfa("[Ð-Ò]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("Ï").is_err());
        assert!(dfa.find("Ð").is_ok());
        assert!(dfa.find("Ñ").is_ok());
        assert!(dfa.find("Ò").is_ok());
        assert!(dfa.find("Ó").is_err());

        let nfa = regex_to_nfa("[ΰ-ϱ]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("ϯ").is_ok())
    }
    #[test]
    fn range23() {
        let from = "Ϣ";
        let to = "ॐ";
        let nfa = regex_to_nfa("[Ϣ-ॐ]").unwrap();
        let dfa = DFA::from(nfa);

        let mut start: [u8; 4] = [0; 4];
        from.chars().nth(0).unwrap().encode_utf8(&mut start);
        let mut end: [u8; 4] = [0; 4];
        to.chars().nth(0).unwrap().encode_utf8(&mut end);

        assert!(dfa.find(" ").is_err());
        assert!(dfa.find("r").is_err());
        assert!(dfa.find("s").is_err());
        assert!(dfa.find("~").is_err());
        assert!(dfa.find("|").is_err());
        assert!(dfa.find("ƒ").is_err());

        assert!(dfa.find("ƒ").is_err());
        assert!(dfa.find("®").is_err());
        assert!(dfa.find("«").is_err());
        assert!(dfa.find("ª").is_err());
        assert!(dfa.find("z").is_err());

        assert!(dfa.find("ॉ").is_ok());
        assert!(dfa.find("ऽ").is_ok());
        assert!(dfa.find("ऴ").is_ok());
        assert!(dfa.find("ॐ").is_ok());
        assert!(dfa.find("Ϣ").is_ok());
        assert!(dfa.find("Ϣ ").is_err());
        assert!(dfa.find("ख़").is_err());
    }

    #[test]
    fn range33() {
        // true true true
        let nfa = regex_to_nfa("[ଳ-ଳ]").unwrap();
        let dfa = DFA::from(nfa);
        assert!('ଳ'.len_utf8() == 3);
        assert!('ଳ' != 'ଲ');
        assert!(dfa.find("ଲ").is_err());
        assert!(dfa.find("ଳ").is_ok());
        assert!(dfa.find("ଵ").is_err());

        // true true false
        let nfa = regex_to_nfa("[ଳ-ଵ]").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("ଲ").is_err());
        assert!(dfa.find("ଳ").is_ok());
        assert!(dfa.find("ଵ").is_ok());
        assert!(dfa.find("ଶ").is_err());

        // true false true
        let nfa = regex_to_nfa("[શ-ଶ]").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("੶").is_err());
        assert!(dfa.find("શ").is_ok());
        //assert!(dfa.find("ଵ").is_ok());
        assert!(dfa.find("ଶ").is_ok());
        assert!(dfa.find("૵").is_ok());
        assert!(dfa.find("ૺ").is_ok());
        assert!(dfa.find("୶").is_err());
        //assert!(dfa.find("ଷ").is_err());
        assert!(dfa.find("ᬶ").is_err());
        assert!(dfa.find("ષ").is_ok());

        // true false false
        let nfa = regex_to_nfa("[ષ-ହ]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("શ").is_err());
        assert!(dfa.find("੷").is_err());
        assert!(dfa.find("ષ").is_ok());
        assert!(dfa.find("૷").is_ok());
        assert!(dfa.find("૷").is_ok());
        assert!(dfa.find("સ").is_ok());
        assert!(dfa.find("ସ").is_ok());
        assert!(dfa.find("૾").is_ok());
        assert!(dfa.find("ହ").is_ok());
        assert!(dfa.find("଺").is_err());
        assert!(dfa.find("୹").is_err());
        assert!(dfa.find("ஹ").is_err());

        // false true true
        let nfa = regex_to_nfa("[ᬹ-㬹]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("ହ").is_err());
        assert!(dfa.find("ᬹ").is_ok());
        assert!(dfa.find("⬹").is_ok());
        assert!(dfa.find("㬹").is_ok());
        assert!(dfa.find("䬹").is_err());

        // false true false
        let nfa = regex_to_nfa("[㬼-䬽]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("ହ").is_err());
        assert!(dfa.find("㬼").is_ok());
        assert!(dfa.find("䬽").is_ok());
        assert!(dfa.find("䬽").is_ok());
    }

    #[test]
    fn range34() {
        let nfa = regex_to_nfa("[㬹-😈]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("㬹").is_ok());
        assert!(dfa.find("㬺").is_ok());
        assert!(dfa.find("㭹").is_ok());
        assert!(dfa.find("䬹").is_ok());
        assert!(dfa.find("嬹").is_ok());

        assert!(dfa.find("𞗈").is_ok());
        assert!(dfa.find("🗈").is_ok());
        assert!(dfa.find("😇").is_ok());
        assert!(dfa.find("😈").is_ok());
        assert!(dfa.find("😉").is_err());
        assert!(dfa.find("🙈").is_err());
        assert!(dfa.find("𠘈").is_err());
        assert!(dfa.find("񟘈").is_err());
    }

    #[test]
    fn range44() {
        // true true true true
        let nfa = regex_to_nfa("[😈-😈]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("😆").is_err());
        assert!(dfa.find("😇").is_err());
        assert!(dfa.find("😈").is_ok());
        assert!(dfa.find("😉").is_err());
        assert!(dfa.find("😊").is_err());

        let nfa = regex_to_nfa("[𿿾-𿿾]").unwrap();
        let dfa = DFA::from(nfa);
        assert!(dfa.find("𿿿").is_err());

        // true true true false
        let nfa = regex_to_nfa("[😇-😉]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("😆").is_err());
        assert!(dfa.find("😇").is_ok());
        assert!(dfa.find("😈").is_ok());
        assert!(dfa.find("😉").is_ok());
        assert!(dfa.find("😊").is_err());

        // true true false true
        let nfa = regex_to_nfa("[😈-🚈]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("🗈").is_err());
        assert!(dfa.find("😈").is_ok());
        assert!(dfa.find("🙈").is_ok());
        assert!(dfa.find("🙋").is_ok());
        assert!(dfa.find("🚈").is_ok());
        assert!(dfa.find("🛈").is_err());

        // true true false false
        let nfa = regex_to_nfa("[😈-🚊]").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("🗈").is_err());
        assert!(dfa.find("😈").is_ok());
        assert!(dfa.find("🙈").is_ok());
        assert!(dfa.find("🙋").is_ok());
        assert!(dfa.find("🚈").is_ok());
        assert!(dfa.find("🚊").is_ok());
        assert!(dfa.find("🚋").is_err());
        assert!(dfa.find("🛊").is_err());

        // true false true true
        let nfa = regex_to_nfa("[😈-𡘈]").unwrap();
        // println!("FIND: {:?}",nfa.find("𠘈"));
        // println!("{:?}",nfa);
        let dfa = DFA::from(nfa);

        assert!(dfa.find("🗈").is_err());
        assert!(dfa.find("😈").is_ok());
        assert!(dfa.find("😉").is_ok());
        assert!(dfa.find("𠘈").is_ok());
        assert!(dfa.find("𡘈").is_ok());

        // true false true fals

        // true false false true

        // true false false false

        // false true true true

        // false true true false

        // false true false true

        // false true false false
        let nfa = regex_to_nfa("[😈-򟚊]").unwrap();
        // println!("FIND: {:?}",nfa.find("𠘈"));
        // println!("{:?}",nfa);
        let dfa = DFA::from(nfa);

        assert!(dfa.find("🗈").is_err());
        assert!(dfa.find("😈").is_ok());
        assert!(dfa.find("😉").is_ok());
        assert!(dfa.find("򟚊").is_ok());
        // false false true true

        // flase false true false

        // false false false true

        // false false false false
    }

    #[test]
    fn test_all() {
        let test_cases = [
            // ascii
            'a', 'b', 'c', 'd', 'z', // 2byte
            'ϱ', 'ϯ', 'Ϯ', 'ϯ', 'ϰ', 'ΰ', 'ί', 'ή', 'έ', 'Ͱ', 'ͱ', 'Ͳ', 'Ͱ', 'ඌ', //3 byte
            'ଵ', 'ଶ', 'ଷ', 'ସ', '୵', '୶', '୷', 'வ', 'ஶ', 'ஷ', 'ᬵ', '㬵', '㭵', '㮵', '㯵', '㬶',
            '㬷', '䬷', //4 byte
            '😈', '😉', '😊', '🙈', '🚈', '🚉', '🙉', '🙇', '😌', '򟘈', '󟘈', '󜘈', '󄖄',
            // edgecase ascii
            ' ', '', '', '}', '~', '', // edgecase 2byte
            'À', 'Á', 'Â', 'Ã', 'ß', 'Þ', 'Ý', '߿', '߾', '޿', // edgecase 3byte
            'က', 'ခ', 'ဂ', '၀', 'ႀ', // edgecase 4 byte
            '񀀀', '𿿿', '𿿾', '𿾿', '𾿿', '󿿿',
        ];

        for from in test_cases.iter() {
            for to in test_cases.iter() {
                if from > to {
                    continue;
                }

                let regex = format!("[{}-{}]", from, to);
                let nfa = regex_to_nfa(regex.as_str()).unwrap();
                let dfa = DFA::from(nfa);

                for value in test_cases.iter() {
                    let string = value.to_string();
                    if is_between(*from, *value, *to) || is_between(*to, *value, *from) {
                        if !(dfa.find(&string).is_ok()) {
                            println!("ERROR from:{} to:{} value:{}", from, to, value);
                            assert!(false);
                        }
                    } else {
                        if !(dfa.find(&string).is_err()) {
                            println!(
                                "ERROR from:{} to:{} value:{} should fail but does not",
                                from, to, value
                            );
                            assert!(false);
                        }
                    }
                }
            }
        }
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
    fn union() {
        let nfa = regex_to_nfa("a|b").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find("").is_err());
        assert!(dfa.find(" ").is_err());
    }


    #[test]
    fn dot_internals() {

        let from1:char = char::from(0x0);
        let to1:char = char::from(0x9);
        
        let regex1 = format!("[{}-{}]",from1,to1);
        let nfa = regex_to_nfa(&regex1).unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find(&from1.to_string()).is_ok());
        assert!(dfa.find(&to1.to_string()).is_ok());
        assert!(dfa.find(" ").is_err());
        assert!(dfa.find("").is_err());

        let from2:char = char::from(0xB);
        let to2:char = "\u{10ffff}".chars().next().unwrap();

        let regex2 = format!("[{}-{}]",from2,to2);
        let nfa = regex_to_nfa(&regex2).unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find(&from2.to_string()).is_ok());
        assert!(dfa.find(&to2.to_string()).is_ok());
        assert!(dfa.find(" ").is_ok());
        

        // Do union
        let regex3 = format!("[{}-{}]|[{}-{}]",from1,to1,from2,to2);
        println!("{}",regex3);
        let nfa = regex_to_nfa(&regex1).unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("").is_err());
        assert!(dfa.find("a").is_ok());
        

        //ClassUnicodeRange { start: "0xB", end: "\u{10ffff}" }]
    }

    #[test]
    fn dot() {
        let nfa = regex_to_nfa(".").unwrap();
        let dfa = DFA::from(nfa);

        assert!(dfa.find("aa").is_err());
        assert!(dfa.find("").is_err());
        assert!(dfa.find("a").is_ok());
        assert!(dfa.find("b").is_ok());
        assert!(dfa.find(" ").is_ok());
        assert!(dfa.find("\n").is_err());
        assert!(dfa.find("ab").is_err());
        assert!(dfa.find("  ").is_err());
    }
}
