#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Add {
    pub rs1: u8,
    pub rs2: u8,
    pub rd: u8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Sub {
    pub rs1: u8,
    pub rs2: u8,
    pub rd: u8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct AddI {
    pub rs1: u8,
    pub rd: u8,
    // 12 bit sign extended immediate value
    // -2048 to 2047
    pub imm12: i16,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SllI {
    pub rs1: u8,
    pub rd: u8,
    // 5 bit unsigned immediate value
    // 0 to 31
    pub shamt: u8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Load {
    pub rs1: u8,
    pub rd: u8,
    // 12 bit sign extended immediate value
    // -2048 to 2047
    pub imm12: i16,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Instruction {
    ADD(Add),
    ADDI(AddI),
    SUB(Sub),
    SLLI(SllI),
    LB(Load),
    LH(Load),
    LW(Load),
    LBU(Load),
    LHU(Load),
    ECALL,
    EBREAK,
    UNKNOWN,
}
