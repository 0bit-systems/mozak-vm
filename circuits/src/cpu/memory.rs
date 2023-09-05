#[cfg(test)]
#[allow(clippy::cast_possible_wrap)]
mod tests {
    use mozak_vm::instruction::{Args, Instruction, Op};
    use mozak_vm::test_utils::{simple_test_code, u32_extra, u8_extra};
    use proptest::prelude::ProptestConfig;
    use proptest::proptest;

    use crate::cpu::stark::CpuStark;
    use crate::stark::mozak_stark::MozakStark;
    use crate::test_utils::ProveAndVerify;

    #[test]
    fn prove_sb_test() {
        let (program, record) = simple_test_code(
            &[
                Instruction {
                    op: Op::SB,
                    args: Args {
                        rs1: 6,
                        rs2: 7,
                        ..Args::default()
                    },
                },
            ],
            &[],
            &[(6, 100), (7, 200)],
        );

        MozakStark::prove_and_verify(&program, &record).unwrap();
    }
    #[test]
    fn prove_mem_read_write_test() {
        let (program, record) = simple_test_code(
            &[
                Instruction {
                    op: Op::SB,
                    args: Args {
                        rs1: 1,
                        rs2: 2,
                        imm: 0,
                        ..Args::default()
                    },
                },
                Instruction {
                    op: Op::LBU,
                    args: Args {
                        rs2: 2,
                        imm: 0,
                        ..Args::default()
                    },
                },
            ],
            &[],
            &[(1, 100), (2, 200)],
        );

        MozakStark::prove_and_verify(&program, &record).unwrap();
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(4))]
        #[test]
        fn prove_sb_proptest(a in u32_extra(), b in u32_extra()) {
            let (program, record) = simple_test_code(
                &[
                    Instruction {
                        op: Op::SB,
                        args: Args {
                            rs1: 6,
                            rs2: 7,
                            ..Args::default()
                        },
                    },
                ],
                &[],
                &[(6, a), (7, b)],
            );

            CpuStark::prove_and_verify(&program, &record).unwrap();
        }

        #[test]
        fn prove_lbu_proptest(a in u32_extra(), b in u32_extra()) {
            let (program, record) = simple_test_code(
                &[
                    Instruction {
                        op: Op::LBU,
                        args: Args {
                            rs1: 6,
                            rs2: 7,
                            ..Args::default()
                        },
                    },
                ],
                &[],
                &[(6, a), (7, b)],
            );

            CpuStark::prove_and_verify(&program, &record).unwrap();
        }
        #[test]
        fn prove_mem_read_write_proptest(offset in u32_extra(), imm in u32_extra(), content in u8_extra()) {
            let (program, record) = simple_test_code(
                &[
                    Instruction {
                        op: Op::SB,
                        args: Args {
                            rs1: 1,
                            rs2: 2,
                            imm,
                            ..Args::default()
                        },
                    },
                    Instruction {
                        op: Op::LBU,
                        args: Args {
                            rs2: 2,
                            imm,
                            ..Args::default()
                        },
                    },
                ],
                &[],
                &[(1, content.into()), (2, offset)],
            );

            CpuStark::prove_and_verify(&program, &record).unwrap();
        }
    }
}
