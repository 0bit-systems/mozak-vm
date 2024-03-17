use itertools::Itertools;
use mozak_runner::poseidon2::MozakPoseidon2;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::columns_view::{columns_view_impl, make_col_map, NumberOfColumns};
use crate::linear_combination::Column;
use crate::poseidon2::columns::STATE_SIZE;
use crate::poseidon2_sponge::columns::Poseidon2Sponge;

// FIXME: MozakPoseidon2::DATA_CAPACITY_PER_FIELD_ELEMENT;
pub const BYTES_COUNT: usize = MozakPoseidon2::DATA_CAPACITY_PER_FIELD_ELEMENT;

#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct Poseidon2PreimagePack<F> {
    pub clk: F,
    pub addr: F,
    pub bytes: [F; MozakPoseidon2::DATA_CAPACITY_PER_FIELD_ELEMENT],
    pub is_executed: F,
}

columns_view_impl!(Poseidon2PreimagePack);
make_col_map!(Poseidon2PreimagePack);

pub const NUM_POSEIDON2_PREIMAGE_PACK_COLS: usize = Poseidon2PreimagePack::<()>::NUMBER_OF_COLUMNS;

impl<F: RichField> From<&Poseidon2Sponge<F>> for Vec<Poseidon2PreimagePack<F>> {
    fn from(value: &Poseidon2Sponge<F>) -> Self {
        if (value.ops.is_init_permute + value.ops.is_permute).is_one() {
            assert!(
                MozakPoseidon2::FIELD_ELEMENTS_RATE < STATE_SIZE,
                "Packing RATE should be less than STATE_SIZE"
            );
            // FIXME: 8 should be MozakPoseidon2::FE_RATE
            let preimage: [F; MozakPoseidon2::FIELD_ELEMENTS_RATE] = value.preimage
                [..MozakPoseidon2::FIELD_ELEMENTS_RATE]
                .try_into()
                .expect("Should succeed since preimage can't be empty");
            let mut base_address = value.input_addr;
            let mut index = 0;
            // For each FE of preimage we have BYTES_COUNT bytes
            let result = preimage
                .iter()
                .map(|fe| {
                    // Note: assumed `to_be_bytes`, otherwise another side of the array should be
                    // taken
                    let bytes: Vec<_> = fe.clone().to_canonical_u64().to_be_bytes()
                        [MozakPoseidon2::BYTES_PER_FIELD_ELEMENT
                            - MozakPoseidon2::DATA_CAPACITY_PER_FIELD_ELEMENT..]
                        .into_iter()
                        .map(|e| F::from_canonical_u8(*e))
                        .collect();
                    let addr = base_address;
                    let i = index;
                    index+=1;
                    base_address = base_address + F::from_canonical_u64( u64::try_from(MozakPoseidon2::DATA_CAPACITY_PER_FIELD_ELEMENT).expect("Cast from usize to u64 for MozakPoseidon2::BYTES_PER_FIELD_ELEMENT should succeed"));
                    Poseidon2PreimagePack {
                        clk: value.clk,
                        addr,
                        bytes: <[F; MozakPoseidon2::DATA_CAPACITY_PER_FIELD_ELEMENT]>::try_from(
                            bytes,
                        )
                        .unwrap(),
                        is_executed: F::ONE - value.is_padded[i],
                    }
                })
                .collect_vec();
            println!("poseidon-value: {:?}", value);
            println!("preimage-result: {:?}", result);
            return result;
        }
        vec![]
    }
}

#[must_use]
pub fn data_for_poseidon2_sponge<F: Field>() -> Vec<Column<F>> {
    let data = col_map().map(Column::from);
    vec![
        data.clk,
        data.addr,
        Column::<F>::reduce_with_powers(&data.bytes, F::from_canonical_u16(1 << 8)),
    ]
}

#[must_use]
pub fn filter_for_poseidon2_sponge<F: Field>() -> Column<F> {
    col_map().map(Column::from).is_executed
}

#[must_use]
pub fn data_for_input_memory<F: Field>(index: u8) -> Vec<Column<F>> {
    assert!(
        usize::try_from(index).unwrap() < BYTES_COUNT,
        "poseidon2-preimage data_for_input_memory: index can be 0..{:?}",
        BYTES_COUNT
    );
    let data = col_map().map(Column::from);
    vec![
        data.clk,
        Column::constant(F::ZERO),               // is_store
        Column::constant(F::ONE),                // is_load
        data.bytes[index as usize].clone(),      // value
        data.addr + F::from_canonical_u8(index), // address
    ]
}

#[must_use]
pub fn filter_for_input_memory<F: Field>() -> Column<F> { col_map().map(Column::from).is_executed }
