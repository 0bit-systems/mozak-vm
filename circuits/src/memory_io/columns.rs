use core::ops::Add;

use crate::columns_view::{columns_view_impl, make_col_map, NumberOfColumns};
use crate::cross_table_lookup::ColumnX;
use crate::memory::columns::MemoryCtl;

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
    pub fn is_executed(&self) -> T {
        self.ops.is_io_store.clone() + self.ops.is_memory_store.clone()
    }
}

/// Total number of columns.
pub const NUM_IO_MEM_COLS: usize = InputOutputMemory::<()>::NUMBER_OF_COLUMNS;

columns_view_impl!(InputOutputMemoryCtl);
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct InputOutputMemoryCtl<T> {
    pub clk: T,
    pub addr: T,
    pub size: T,
}

type IOCol = ColumnX<InputOutputMemory<i64>>;

/// Columns containing the data which are looked from the CPU table into Memory
/// stark table.
#[must_use]
pub fn data_for_cpu() -> InputOutputMemoryCtl<IOCol> {
    let mem = COL_MAP;
    InputOutputMemoryCtl {
        clk: mem.clk,
        addr: mem.addr,
        size: mem.size,
    }
}

/// Column for a binary filter to indicate a lookup
#[must_use]
pub fn filter_for_cpu() -> IOCol { COL_MAP.ops.is_io_store }

/// Columns containing the data which are looked from the halfword memory table
/// into Memory stark table.
#[must_use]
pub fn data_for_memory() -> MemoryCtl<IOCol> {
    let mem = COL_MAP;

    MemoryCtl {
        clk: mem.clk,
        is_store: mem.ops.is_memory_store,
        is_load: ColumnX::constant(0),
        value: mem.value,
        addr: mem.addr,
    }
}

/// Column for a binary filter to indicate a lookup
#[must_use]
pub fn filter_for_memory() -> IOCol { COL_MAP.ops.is_memory_store }
