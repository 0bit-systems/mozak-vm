#![allow(clippy::too_many_lines)]

use std::fmt::Display;

use anyhow::Result;
use log::Level::Debug;
use log::{debug, log_enabled};
use mozak_runner::elf::Program;
use mozak_runner::vm::ExecutionRecord;
use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;
use plonky2::timed;
use plonky2::util::log2_strict;
use plonky2::util::timing::TimingTree;
use starky::config::StarkConfig;
use starky::proof::StarkProofWithMetadata;
use starky::stark::Stark;

use super::mozak_stark::{MozakStark, TableKind, TableKindArray, TableKindSetBuilder};
use super::proof::AllProof;
use crate::generation::{debug_traces, generate_traces};
use crate::stark::mozak_stark::{all_starks, PublicInputs};

/// Prove the execution of a given [Program]
///
/// ## Parameters
/// `program`: A serialized ELF Program
/// `record`: Non-constrained execution trace generated by the runner
/// `mozak_stark`: Mozak-VM Gadgets
/// `config`: Stark and FRI security configurations
/// `public_inputs`: Public Inputs to the Circuit
/// `timing`: Profiling tool
pub fn prove<F, C, const D: usize>(
    program: &Program,
    record: &ExecutionRecord<F>,
    mozak_stark: &MozakStark<F, D>,
    config: &StarkConfig,
    public_inputs: PublicInputs<F>,
    timing: &mut TimingTree,
) -> Result<AllProof<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>, {
    debug!("Starting Prove");

    let traces_poly_values = timed!(
        timing,
        "Generate Traces",
        generate_traces(program, record, timing)
    );
    debug!("Done with Trace Generation");
    if mozak_stark.debug || std::env::var("MOZAK_STARK_DEBUG").is_ok() {
        debug_traces(&traces_poly_values, mozak_stark, &public_inputs);
    }
    timed!(
        timing,
        "Prove with Traces",
        prove_with_traces(
            mozak_stark,
            config,
            public_inputs,
            &traces_poly_values,
            timing,
        )
    )
}

/// Given the traces generated from [`generate_traces`], prove a [`MozakStark`].
///
/// # Errors
/// Errors if proving fails.
pub fn prove_with_traces<F, C, const D: usize>(
    mozak_stark: &MozakStark<F, D>,
    config: &StarkConfig,
    public_inputs: PublicInputs<F>,
    traces_poly_values: &TableKindArray<Vec<PolynomialValues<F>>>,
    timing: &mut TimingTree,
) -> Result<AllProof<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>, {
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;

    let trace_commitments = timed!(
        timing,
        "Compute trace commitments for each table",
        traces_poly_values
            .clone()
            .with_kind()
            .map(|(trace, table)| {
                timed!(
                    timing,
                    &format!("compute trace commitment for {table:?}"),
                    PolynomialBatch::<F, C, D>::from_values(
                        trace.clone(),
                        rate_bits,
                        false,
                        cap_height,
                        timing,
                        None,
                    )
                )
            })
    );

    let trace_caps = trace_commitments
        .each_ref()
        .map(|c| c.merkle_tree.cap.clone());
    // Add trace commitments to the challenger entropy pool.
    let mut challenger = Challenger::<F, C::Hasher>::new();
    for cap in &trace_caps {
        challenger.observe_cap(cap);
    }
    let starky_cross_table_lookups = mozak_stark
        .cross_table_lookups
        .clone()
        .map(starky::cross_table_lookup::CrossTableLookup::from);
    let (starky_ctl_challenges, starky_ctl_datas) = {
        starky::cross_table_lookup::get_ctl_data::<F, C, D, { TableKind::COUNT }>(
            config,
            &traces_poly_values.0,
            &starky_cross_table_lookups,
            &mut challenger,
            3,
        )
    };

    let proofs = timed!(
        timing,
        "compute all proofs given commitments",
        // TODO: use starky's `prove_with_commitments`
        prove_with_commitments(
            mozak_stark,
            config,
            &public_inputs,
            traces_poly_values,
            &trace_commitments,
            &mut challenger,
            timing,
            &starky_ctl_challenges,
            &starky_ctl_datas,
        )?
    );

    let program_rom_trace_cap = trace_caps[TableKind::Program].clone();
    let elf_memory_init_trace_cap = trace_caps[TableKind::ElfMemoryInit].clone();
    if log_enabled!(Debug) {
        timing.print();
    }
    Ok(AllProof {
        proofs,
        ctl_challenges: starky_ctl_challenges,
        program_rom_trace_cap,
        elf_memory_init_trace_cap,
        public_inputs,
    })
}

/// Compute proof for a single STARK table, with lookup data.
///
/// # Errors
/// Errors if FRI parameters are wrongly configured, or if
/// there are no z polys, or if our
/// opening points are in our subgroup `H`,
#[allow(clippy::too_many_arguments)]
pub(crate) fn prove_single_table<F, C, S, const D: usize>(
    stark: &S,
    config: &StarkConfig,
    trace_poly_values: &[PolynomialValues<F>],
    trace_commitment: &PolynomialBatch<F, C, D>,
    challenger: &mut Challenger<F, C::Hasher>,
    public_inputs: &[F],
    timing: &mut TimingTree,
    starky_ctl_challenges: &starky::lookup::GrandProductChallengeSet<F>,
    starky_ctl_data: &starky::cross_table_lookup::CtlData<'_, F>,
    // Of course, we need to match the output, too.
    // Ok, looks doable.
) -> Result<starky::proof::StarkProofWithMetadata<F, C, D>>
where
    F: RichField + Extendable<D> + Copy + Eq + core::fmt::Debug,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D> + Display, {
    let degree = trace_poly_values[0].len();
    let degree_bits = log2_strict(degree);
    let fri_params = config.fri_params(degree_bits);
    let rate_bits = config.fri_config.rate_bits;
    let cap_height = config.fri_config.cap_height;
    assert!(
        fri_params.total_arities() <= degree_bits + rate_bits - cap_height,
        "FRI total reduction arity is too large.",
    );

    // Clear buffered outputs.
    let init_challenger_state = challenger.compact();

    starky::prover::prove_with_commitment(
        stark,
        config,
        trace_poly_values,
        trace_commitment,
        Some(starky_ctl_data),
        Some(starky_ctl_challenges),
        challenger,
        public_inputs,
        timing,
    )
    .map(|proof_with_pis| StarkProofWithMetadata {
        proof: proof_with_pis.proof,
        init_challenger_state,
    })
}

/// Given the traces generated from [`generate_traces`] along with their
/// commitments, prove a [`MozakStark`].
///
/// # Errors
/// Errors if proving fails.
#[allow(clippy::too_many_arguments)]
pub fn prove_with_commitments<F, C, const D: usize>(
    mozak_stark: &MozakStark<F, D>,
    config: &StarkConfig,
    public_inputs: &PublicInputs<F>,
    traces_poly_values: &TableKindArray<Vec<PolynomialValues<F>>>,
    trace_commitments: &TableKindArray<PolynomialBatch<F, C, D>>,
    challenger: &mut Challenger<F, C::Hasher>,
    timing: &mut TimingTree,
    starky_ctl_challenges: &starky::lookup::GrandProductChallengeSet<F>,
    starky_ctl_datas: &[starky::cross_table_lookup::CtlData<'_, F>; TableKind::COUNT],
) -> Result<TableKindArray<starky::proof::StarkProofWithMetadata<F, C, D>>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>, {
    let cpu_skeleton_stark = [public_inputs.entry_point];
    let public_inputs = TableKindSetBuilder::<&[_]> {
        cpu_skeleton_stark: &cpu_skeleton_stark,
        ..Default::default()
    }
    .build();

    // Clear buffered outputs.
    challenger.compact();
    Ok(all_starks!(mozak_stark, |stark, kind| {
        let mut challenger = challenger.clone();
        prove_single_table(
            stark,
            config,
            &traces_poly_values[kind],
            &trace_commitments[kind],
            &mut challenger,
            public_inputs[kind],
            timing,
            starky_ctl_challenges,
            &starky_ctl_datas[kind as usize],
        )
        .unwrap()
    }))
}

#[cfg(test)]
mod tests {

    use mozak_runner::code;
    use mozak_runner::instruction::{Args, Instruction, Op};
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use plonky2::hash::poseidon2::Poseidon2Hash;
    use plonky2::plonk::config::{GenericHashOut, Hasher};

    use crate::stark::mozak_stark::MozakStark;
    use crate::test_utils::{create_poseidon2_test, Poseidon2Test, ProveAndVerify};

    #[test]
    fn prove_halt() {
        let (program, record) = code::execute([], &[], &[]);
        MozakStark::prove_and_verify(&program, &record).unwrap();
    }

    #[test]
    fn prove_lui() {
        let lui = Instruction {
            op: Op::ADD,
            args: Args {
                rd: 1,
                imm: 0x8000_0000,
                ..Args::default()
            },
        };
        let (program, record) = code::execute([lui], &[], &[]);
        assert_eq!(record.last_state.get_register_value(1), 0x8000_0000);
        MozakStark::prove_and_verify(&program, &record).unwrap();
    }

    #[test]
    fn prove_lui_2() {
        let (program, record) = code::execute(
            [Instruction {
                op: Op::ADD,
                args: Args {
                    rd: 1,
                    imm: 0xDEAD_BEEF,
                    ..Args::default()
                },
            }],
            &[],
            &[],
        );
        assert_eq!(record.last_state.get_register_value(1), 0xDEAD_BEEF,);
        MozakStark::prove_and_verify(&program, &record).unwrap();
    }

    #[test]
    fn prove_beq() {
        let (program, record) = code::execute(
            [Instruction {
                op: Op::BEQ,
                args: Args {
                    rs1: 0,
                    rs2: 1,
                    imm: 42, // branch target
                    ..Args::default()
                },
            }],
            &[],
            &[(1, 2)],
        );
        assert_eq!(record.last_state.get_pc(), 8);
        MozakStark::prove_and_verify(&program, &record).unwrap();
    }

    fn test_poseidon2(test_data: &[Poseidon2Test]) {
        let (program, record) = create_poseidon2_test(test_data);
        for test_datum in test_data {
            let output: Vec<u8> = (0..32_u8)
                .map(|i| {
                    record
                        .last_state
                        .load_u8(test_datum.output_start_addr + u32::from(i))
                })
                .collect();
            let mut data_bytes = test_datum.data.as_bytes().to_vec();
            // VM expects input len to be multiple of RATE bits
            data_bytes.resize(data_bytes.len().next_multiple_of(8), 0_u8);
            let data_fields: Vec<GoldilocksField> = data_bytes
                .iter()
                .map(|x| GoldilocksField::from_canonical_u8(*x))
                .collect();
            assert_eq!(output, Poseidon2Hash::hash_no_pad(&data_fields).to_bytes());
        }
        MozakStark::prove_and_verify(&program, &record).unwrap();
    }

    #[test]
    fn prove_poseidon2() {
        test_poseidon2(&[Poseidon2Test {
            data: "💥 Mozak-VM Rocks With Poseidon2".to_string(),
            input_start_addr: 1024,
            output_start_addr: 2048,
        }]);
        test_poseidon2(&[Poseidon2Test {
            data: "😇 Mozak is knowledge arguments based technology".to_string(),
            input_start_addr: 1024,
            output_start_addr: 2048,
        }]);
        test_poseidon2(&[
            Poseidon2Test {
                data: "💥 Mozak-VM Rocks With Poseidon2".to_string(),
                input_start_addr: 512,
                output_start_addr: 1024,
            },
            Poseidon2Test {
                data: "😇 Mozak is knowledge arguments based technology".to_string(),
                input_start_addr: 1024 + 32,
                // make sure input and output do not overlap with
                // earlier call
                output_start_addr: 2048,
            },
        ]);
    }
}
