use core::ops::Add;

use mozak_system::system::reg_abi::REG_A1;
use plonky2::field::types::Field;

use crate::columns_view::{columns_view_impl, make_col_map, NumberOfColumns};
use crate::cross_table_lookup::Column;
use crate::stark::mozak_stark::{IoMemoryPrivateTable, IoMemoryPublicTable, Table};

// OK, try memory IO ecall via register stark.

/// Operations (one-hot encoded)
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct Ops<T> {
    /// Binary filter column to represent a RISC-V SB operation.
    pub is_memory_store: T,
    /// Binary filter column to represent a io-read operation.
    pub is_io_store: T,
}

#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct InputOutputMemory<T> {
    /// Clock at memory access.
    pub clk: T,
    /// Address: start-address
    pub addr: T,
    /// Size: size of io-chunk in bytes
    pub size: T,
    /// Value: byte value
    pub value: T,
    /// Operation: io_store/load io_memory_store/load
    pub ops: Ops<T>,
    /// Helper to decrease poly degree
    pub is_lv_and_nv_are_memory_rows: T,
}

columns_view_impl!(InputOutputMemory);
make_col_map!(InputOutputMemory);

impl<T: Clone + Add<Output = T>> InputOutputMemory<T> {
    pub fn is_io(&self) -> T { self.ops.is_io_store.clone() }

    pub fn is_memory(&self) -> T { self.ops.is_memory_store.clone() }

    pub fn is_executed(&self) -> T {
        self.ops.is_io_store.clone() + self.ops.is_memory_store.clone()
    }
}

/// Total number of columns.
pub const NUM_IO_MEM_COLS: usize = InputOutputMemory::<()>::NUMBER_OF_COLUMNS;

/// Columns containing the data which are looked from the CPU table into Memory
/// stark table.
#[must_use]
pub fn data_for_cpu<F: Field>() -> Vec<Column<F>> {
    let mem = col_map().map(Column::from);
    vec![mem.clk, mem.addr, mem.size, mem.ops.is_io_store]
}

// #[must_use]
// pub fn data_for_register<F: Field>() -> Vec<Column<F>> {
//     let mem = col_map().map(Column::from);
//     vec![mem.clk, Column::constant(F::ZERO), mem.addr]
// }

/// Column for a binary filter to indicate a lookup
#[must_use]
pub fn filter_for_cpu<F: Field>() -> Column<F> { col_map().map(Column::from).is_io() }

/// Columns containing the data which are looked from the halfword memory table
/// into Memory stark table.
#[must_use]
pub fn data_for_memory<F: Field>() -> Vec<Column<F>> {
    let mem = col_map().map(Column::from);
    vec![
        mem.clk,
        mem.ops.is_memory_store,
        Column::constant(F::ZERO),
        mem.value,
        mem.addr,
    ]
}

/// Column for a binary filter to indicate a lookup
#[must_use]
pub fn filter_for_memory<F: Field>() -> Column<F> { col_map().map(Column::from).is_memory() }

// Look up a read into register table with:
//
// read at augmented clk
//
// REG_A0 -> public input to ecall type (private or public io read, via)
// (Actually, can be hard-coded from the point of view of the proof; doesn't
// need to be PUBLIC_INPUT read REG_A1 -> addr
// read REG_A2 -> size
//
// filter = is_memory_store
/// TODO: at the moment weonly do addr; look up the rest, too.  Adjust trace
/// generation.
#[must_use]
pub fn register_looking<F: Field>() -> Vec<Table<F>> {
    let mem = col_map().map(Column::from);
    let is_read = || Column::constant(F::ONE);
    let three = F::from_canonical_u8(3);
    let clk = || &mem.clk * three;
    vec![
        IoMemoryPrivateTable::new(
            vec![
                is_read(),
                clk(),
                Column::constant(F::from_canonical_u8(REG_A1)),
                mem.addr.clone(),
            ],
            mem.ops.is_io_store.clone(),
        ),
        IoMemoryPublicTable::new(
            vec![
                is_read(),
                clk(),
                Column::constant(F::from_canonical_u8(REG_A1)),
                mem.addr,
            ],
            mem.ops.is_io_store,
        ),
    ]
}
