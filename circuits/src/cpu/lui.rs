use plonky2::field::packed::PackedField;
use starky::constraint_consumer::ConstraintConsumer;

use super::columns::{COL_DST_VALUE, COL_IMM_VALUE, COL_PC, COL_S_LUI, NUM_CPU_COLS};
use crate::utils::column_of_xs;

pub(crate) fn constraints<P: PackedField>(
    lv: &[P; NUM_CPU_COLS],
    nv: &[P; NUM_CPU_COLS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Since while decoding immediate we set 12 LSB to 0
    // VM puts same value to destination
    yield_constr.constraint(lv[COL_S_LUI] * (lv[COL_DST_VALUE] - lv[COL_IMM_VALUE]));

    // pc ticks up
    // TODO(Matthias): factor this out into a function to be used by most
    // instruction, ie all that are not jumping or branching.
    // NOTE(Matthias): if we are careful, bumping the pc by 4 does not need a range
    // check, because we can statically guarantee that the PC is far from
    // wrapping around in both field and u32.
    let inc: P = column_of_xs(4_u32);
    yield_constr.constraint_transition((lv[COL_S_LUI]) * (nv[COL_PC] - lv[COL_PC] - inc));
}
