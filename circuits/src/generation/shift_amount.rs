use plonky2::hash::hash_types::RichField;
use itertools::Itertools;

use crate::cpu::columns::CpuColumnsView;
use crate::shift_amount::columns::{Executed, ShiftAmountView, FIXED_SHAMT_RANGE};

// /// Returns the rows for shift instructions.
// #[must_use]
// pub fn filter_shift_trace(step_rows: &[Row]) -> Vec<usize> {
//     step_rows
//         .iter()
//         .enumerate()
//         .filter_map(|(_, row)| {
//             matches!(
//                 row.state.current_instruction().op,
//                 Op::SLL | Op::SRL | Op::SRA
//             )
//         })
//         .map(|(i, _row)| i)
//         .collect()
// }

fn filter_shift_trace<F: RichField>(
    step_rows: &[CpuColumnsView<F>],
) -> impl Iterator<Item = &Executed<F>> + '_ {
    step_rows
        .iter()
        .filter_map(|row| (row.inst.ops.ops_that_shift().into_iter().sum::<F>() != F::ZERO).then_some(&row.bitshift))
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn generate_shift_amount_trace<F: RichField>(
    cpu_trace: &Vec<CpuColumnsView<F>>,
) -> Vec<ShiftAmountView<F>> {
    let executed =
        filter_shift_trace(cpu_trace)
        .map(|&x| x.map(|t| F::to_noncanonical_u64(&t)))
        .sorted_by_key(|Executed {shamt, ..}| *shamt)
        .merge_join_by(FIXED_SHAMT_RANGE, |Executed {shamt, ..}, i| shamt.cmp(i));
    executed.map(|x|
        match x {
            itertools::EitherOrBoth::Right(i) => ShiftAmountView {
                is_executed: 0,
                executed: Executed {
                    shamt: i,
                    multiplier: 1 << i,
                },
            },
            itertools::EitherOrBoth::Left(executed) |
            itertools::EitherOrBoth::Both(executed, _) => ShiftAmountView {
                is_executed: 1,
                executed,
            },
        }.map(F::from_canonical_u64)).collect()
}
