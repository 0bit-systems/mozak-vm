use crate::coretypes::Poseidon2HashType;
use crate::sys::poseidon2_hash_no_pad;

#[must_use]
pub fn merklelize(mut hashes_with_addr: Vec<(u32, Poseidon2HashType)>) -> Poseidon2HashType {
    while hashes_with_addr.len() > 1 {
        let mut new_hashes_with_addr = vec![];
        let mut prev_pair = None;
        for (mut current_addr, current_hash) in hashes_with_addr {
            match prev_pair {
                None => prev_pair = Some((current_addr, current_hash)),
                Some((mut prev_addr, prev_hash)) => {
                    current_addr >>= 1;
                    prev_addr >>= 1;
                    if prev_addr == current_addr {
                        new_hashes_with_addr.push((
                            current_addr,
                            poseidon2_hash_no_pad(
                                &(vec![
                                    current_hash.to_le_bytes().to_vec(),
                                    prev_hash.to_le_bytes().to_vec(),
                                ])
                                .into_iter()
                                .flatten()
                                .collect::<Vec<u8>>(),
                            ),
                        ));
                    } else {
                        new_hashes_with_addr
                            .extend(vec![(prev_addr, prev_hash), (current_addr, current_hash)]);
                    }
                    prev_pair = None;
                }
            }
        }
        hashes_with_addr = new_hashes_with_addr;
    }
    let (_root_addr, root_hash) = hashes_with_addr[0];
    root_hash
}

#[cfg(test)]
mod tests {
    use super::merklelize;
    use crate::coretypes::{
        Address, CanonicalEventType, Event, Poseidon2HashType, ProgramIdentifier, StateObject,
    };
    use crate::sys::{CanonicalEventTapeSingle, EventTapeSingle};

    #[test]
    pub fn sample_test_run() {
        let program_id = ProgramIdentifier::default();
        let object = StateObject {
            address: Address::from([1u8; 4]),
            constraint_owner: ProgramIdentifier::default(),
            data: vec![1, 2, 3, 4, 5],
        };

        let new_object = StateObject {
            data: vec![6, 7, 8, 9, 10],
            ..object
        };

        let another_object = StateObject {
            address: Address::from([2u8; 4]),
            constraint_owner: ProgramIdentifier::default(),
            data: vec![1, 2, 3, 4, 5, 6],
        };

        let read_event = Event {
            object,
            operation: CanonicalEventType::Read,
        };

        let write_event = Event {
            object: new_object,
            operation: CanonicalEventType::Write,
        };

        let another_object_read_event = Event {
            object: another_object,
            operation: CanonicalEventType::Read,
        };

        let event_tape = EventTapeSingle {
            id: program_id,
            contents: vec![read_event, write_event, another_object_read_event],
            canonical_repr: Default::default(),
        };

        let canonical_event_tape: CanonicalEventTapeSingle = event_tape.into();
        let root_hash = canonical_event_tape.canonical_hash();
        assert_eq!(root_hash.to_le_bytes(), [
            159, 132, 147, 134, 125, 28, 139, 35, 191, 116, 104, 28, 101, 96, 74, 246, 157, 14, 9,
            53, 55, 174, 28, 120, 129, 39, 217, 11, 93, 190, 58, 124
        ])
    }
    #[test]
    fn sample_merkelize() {
        let hashes_with_addr = vec![
            (200, Poseidon2HashType([1u8; 32])),
            (100, Poseidon2HashType([2u8; 32])),
            (300, Poseidon2HashType([3u8; 32])),
        ];
        println!("{:?}", merklelize(hashes_with_addr).to_le_bytes());
    }
}
