use std::marker::PhantomData;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::stark::Stark;
use starky::vars::{StarkEvaluationTargets, StarkEvaluationVars};

use super::columns::{self, LimbKind};
use crate::lookup::{eval_lookups, eval_lookups_circuit};

#[derive(Copy, Clone, Default)]
#[allow(clippy::module_name_repetitions)]
pub struct RangeCheckStark<F, const D: usize> {
    pub _f: PhantomData<F>,
}

impl<F: RichField, const D: usize> RangeCheckStark<F, D> {
    const BASE: usize = 1 << 16;
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for RangeCheckStark<F, D> {
    const COLUMNS: usize = columns::NUM_RC_COLS;
    const PUBLIC_INPUTS: usize = 0;

    /// Given the u32 value and the u16 limbs found in our variables to be
    /// evaluated, perform:
    ///   1. sumcheck between val (u32) and limbs (u16),
    ///   2. rangecheck for limbs.
    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>, {
        for col in [columns::VAL, columns::OP1_FIXED] {
            // Constrain `val` - (`limb_hi` ** base + `limb_lo`) == 0
            let val = vars.local_values[col];
            let filter = vars.local_values[columns::FILTER_START + col];
            let limb_lo = vars.local_values[LimbKind::col(col, LimbKind::Lo)];
            let limb_hi = vars.local_values[LimbKind::col(col, LimbKind::Hi)];
            yield_constr.constraint(
                filter * (val - (limb_lo + limb_hi * P::Scalar::from_canonical_usize(Self::BASE))),
            );

            eval_lookups(
                vars,
                yield_constr,
                LimbKind::col(col, LimbKind::LoPermuted),
                LimbKind::col(col, LimbKind::LoFixedPermuted),
            );
            eval_lookups(
                vars,
                yield_constr,
                LimbKind::col(col, LimbKind::HiPermuted),
                LimbKind::col(col, LimbKind::HiFixedPermuted),
            );
        }
    }

    /// Given the u32 value and the u16 limbs found in our variables to be
    /// evaluated, perform:
    ///   1. sumcheck between val (u32) and limbs (u16),
    ///   2. rangecheck for limbs.
    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        for idx in [columns::VAL, columns::OP1_FIXED] {
            let val = vars.local_values[idx];
            let filter = vars.local_values[columns::FILTER_START + idx];
            let limb_lo = vars.local_values[LimbKind::col(idx, LimbKind::Lo)];
            let limb_hi = vars.local_values[LimbKind::col(idx, LimbKind::Hi)];
            let base = builder.constant_extension(F::Extension::from_canonical_usize(Self::BASE));
            let sum = builder.mul_add_extension(limb_hi, base, limb_lo);
            let val_sum_diff = builder.sub_extension(val, sum);
            let filtered_val_sum_diff = builder.mul_extension(filter, val_sum_diff);
            yield_constr.constraint(builder, filtered_val_sum_diff);

            eval_lookups_circuit(
                builder,
                vars,
                yield_constr,
                LimbKind::col(idx, LimbKind::LoPermuted),
                LimbKind::col(idx, LimbKind::LoFixedPermuted),
            );
            eval_lookups_circuit(
                builder,
                vars,
                yield_constr,
                LimbKind::col(idx, LimbKind::HiPermuted),
                LimbKind::col(idx, LimbKind::HiFixedPermuted),
            );
        }
    }

    fn constraint_degree(&self) -> usize { 3 }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use log::trace;
    use mozak_vm::test_utils::simple_test;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Sample;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::util::log2_strict;
    use starky::stark::Stark;
    use starky::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    use super::*;
    use crate::generation::rangecheck::generate_rangecheck_trace;

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    type S = RangeCheckStark<F, D>;

    /// Generates a trace which contains a value that should fail the range
    /// check.
    fn generate_failing_trace() -> [Vec<GoldilocksField>; columns::NUM_RC_COLS] {
        let record = simple_test(4, &[(0_u32, 0x0073_02b3 /* add r5, r6, r7 */)], &[
            (6, 100),
            (7, 100),
        ]);
        let mut trace = generate_rangecheck_trace::<F>(&record.executed);
        // Manually alter the value here to be larger than a u32.
        trace[0][columns::VAL] = GoldilocksField(u64::from(u32::MAX) + 1_u64);
        trace
    }

    #[test]
    fn test_degree() -> Result<()> {
        let stark = S::default();
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_rangecheck_stark_circuit() -> Result<()> {
        let stark = S::default();
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn test_rangecheck_stark_big_trace() {
        let stark = S::default();
        let inst = 0x0073_02b3 /* add r5, r6, r7 */;

        let mut mem = vec![];
        for i in 0..=u32::from(u16::MAX) {
            mem.push((i * 4, inst));
        }
        let record = simple_test(4, &mem, &[(6, 100), (7, 100)]);

        let trace = generate_rangecheck_trace::<F>(&record.executed);

        let len = trace[0].len();
        let last = F::primitive_root_of_unity(log2_strict(len)).inverse();
        let subgroup =
            F::cyclic_subgroup_known_order(F::primitive_root_of_unity(log2_strict(len)), len);

        for i in 0..len {
            let local_values = trace
                .iter()
                .map(|row| row[i % len])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            let next_values = trace
                .iter()
                .map(|row| row[(i + 1) % len])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let vars = StarkEvaluationVars {
                local_values: &local_values,
                next_values: &next_values,
                public_inputs: &[],
            };

            let mut constraint_consumer = ConstraintConsumer::new(
                vec![F::rand()],
                subgroup[i] - last,
                if i == 0 {
                    GoldilocksField::ONE
                } else {
                    GoldilocksField::ZERO
                },
                if i == len - 1 {
                    GoldilocksField::ONE
                } else {
                    GoldilocksField::ZERO
                },
            );
            stark.eval_packed_generic(vars, &mut constraint_consumer);

            for &acc in &constraint_consumer.constraint_accs {
                if !acc.eq(&GoldilocksField::ZERO) {
                    trace!("constraint error in line {i}");
                }
                assert_eq!(acc, GoldilocksField::ZERO);
            }
        }
    }

    #[test]
    fn test_rangecheck_stark_fail() {
        let stark = S::default();
        let trace = generate_failing_trace();

        let len = trace[0].len();
        let last = F::primitive_root_of_unity(log2_strict(len)).inverse();
        let subgroup =
            F::cyclic_subgroup_known_order(F::primitive_root_of_unity(log2_strict(len)), len);

        let local_values = trace
            .iter()
            .map(|row| row[0])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let next_values = trace
            .iter()
            .map(|row| row[1])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let vars = StarkEvaluationVars {
            local_values: &local_values,
            next_values: &next_values,
            public_inputs: &[],
        };

        let mut constraint_consumer = ConstraintConsumer::new(
            vec![F::rand()],
            subgroup[0] - last,
            GoldilocksField::ONE,
            GoldilocksField::ZERO,
        );
        stark.eval_packed_generic(vars, &mut constraint_consumer);

        // Constraint should not hold, since trace contains a value > u16::MAX.
        assert_ne!(
            constraint_consumer.constraint_accs[0],
            GoldilocksField::ZERO
        );
    }
}
