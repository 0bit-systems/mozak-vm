use mozak_vm::instruction::Op;
use mozak_vm::vm::Row;
use plonky2::hash::hash_types::RichField;

use crate::cpu::columns as cpu_cols;
use crate::utils::{from_, pad_trace};

#[allow(clippy::missing_panics_doc)]
pub fn generate_cpu_trace<F: RichField>(step_rows: &[Row]) -> [Vec<F>; cpu_cols::NUM_CPU_COLS] {
    let mut trace: Vec<Vec<F>> = vec![vec![F::ZERO; step_rows.len()]; cpu_cols::NUM_CPU_COLS];

    for (i, Row { state, aux }) in step_rows.iter().enumerate() {
        trace[cpu_cols::COL_CLK][i] = from_(state.clk);
        trace[cpu_cols::COL_PC][i] = from_(state.get_pc());

        let inst = state.current_instruction();

        trace[cpu_cols::COL_RS1_SELECT[inst.args.rs1 as usize]][i] = F::ONE;
        trace[cpu_cols::COL_RS2_SELECT[inst.args.rs2 as usize]][i] = F::ONE;
        trace[cpu_cols::COL_RD_SELECT[inst.args.rd as usize]][i] = F::ONE;
        trace[cpu_cols::COL_OP1_VALUE][i] = from_(state.get_register_value(inst.args.rs1));
        trace[cpu_cols::COL_OP2_VALUE][i] = from_(state.get_register_value(inst.args.rs2));
        // NOTE: Updated value of DST register is next step.
        trace[cpu_cols::COL_DST_VALUE][i] = from_(aux.dst_val);
        trace[cpu_cols::COL_IMM_VALUE][i] = from_(inst.args.imm);
        trace[cpu_cols::COL_S_HALT][i] = from_(u32::from(aux.will_halt));
        for j in 0..32 {
            trace[cpu_cols::COL_START_REG + j as usize][i] = from_(state.get_register_value(j));
        }

        match inst.op {
            Op::ADD => trace[cpu_cols::COL_S_ADD][i] = F::ONE,
            Op::BEQ => trace[cpu_cols::COL_S_BEQ][i] = F::ONE,
            Op::SUB => trace[cpu_cols::COL_S_SUB][i] = F::ONE,
            Op::ECALL => trace[cpu_cols::COL_S_ECALL][i] = F::ONE,
            #[tarpaulin::skip]
            _ => {}
        }
    }

    // For expanded trace from `trace_len` to `trace_len's power of two`,
    // we use last row `HALT` to pad them.
    let trace = pad_trace(trace, Some(cpu_cols::COL_CLK));

    log::trace!("trace {:?}", trace);
    #[tarpaulin::skip]
    trace.try_into().unwrap_or_else(|v: Vec<Vec<F>>| {
        panic!(
            "Expected a Vec of length {} but it was {}",
            cpu_cols::NUM_CPU_COLS,
            v.len()
        )
    })
}
