use core::ops::Add;

use plonky2::field::types::Field;

use crate::columns_view::{columns_view_impl, make_col_map};
use crate::linear_combination::Column;
#[cfg(feature = "enable_register_starks")]
use crate::stark::mozak_stark::{RegisterTable, Table};

columns_view_impl!(Ops);
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct Ops<T> {
    /// Binary filter column that marks a row as the initialization of
    /// a register.
    pub is_init: T,

    /// Binary filter column that marks a row as a register read.
    pub is_read: T,

    /// Binary filter column that marks a row as a register write.
    pub is_write: T,
}

#[must_use]
pub fn init<T: Field>() -> Ops<T> {
    Ops {
        is_init: T::ONE,
        ..Default::default()
    }
}

#[must_use]
pub fn read<T: Field>() -> Ops<T> {
    Ops {
        is_read: T::ONE,
        ..Default::default()
    }
}

#[must_use]
pub fn write<T: Field>() -> Ops<T> {
    Ops {
        is_write: T::ONE,
        ..Default::default()
    }
}

/// Create a dummy [`Ops`]
///
/// We want these 3 filter columns = 0,
/// so we can constrain `is_used = is_init + is_read + is_write`.
#[must_use]
pub fn dummy<T: Field>() -> Ops<T> { Ops::default() }

columns_view_impl!(Register);
make_col_map!(Register);
/// [`Design doc for RegisterSTARK`](https://www.notion.so/0xmozak/Register-File-STARK-62459d68aea648a0abf4e97aa0093ea2?pvs=4#0729f89ddc724967ac991c9e299cc4fc)
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct Register<T> {
    /// The register 'address' that indexes into 1 of our 32 registers.
    /// Should only take values 0-31, so this column should be a running sum
    /// from 0 to 31 (inclusive). Note that this isn't the same as memory
    /// address.
    pub addr: T,

    /// Value of the register at time (in clk) of access.
    pub value: T,

    /// Augmented clock at register access. This is calculated as:
    /// augmented_clk = clk * 2 for register reads, and
    /// augmented_clk = clk * 2 + 1 for register writes,
    /// to ensure that we do not write to the register before we read.
    pub clk: T,

    /// Columns that indicate what action is taken on the register.
    pub ops: Ops<T>,
}

/// We create a virtual column known as `is_used`, which flags a row as
/// being 'used' if any one of the ops columns are turned on.
/// This is to differentiate between real rows and padding rows.
impl<T: Add<Output = T> + Clone> Register<T> {
    pub fn is_used(self) -> T { self.ops.is_init + self.ops.is_read + self.ops.is_write }

    pub fn is_rw(self) -> T { self.ops.is_read + self.ops.is_write }

    // See, if we want to add a Mul constraint, we need to add a Mul trait bound?
    // Or whether we want to keep manual addition and clone?
    pub fn augmented_clk(self) -> T { self.clk.clone() + self.clk + self.ops.is_write }
}

#[must_use]
pub fn data_for_register_init<F: Field>() -> Vec<Column<F>> { Column::singles([col_map().addr]) }

#[must_use]
pub fn filter_for_register_init<F: Field>() -> Column<F> { Column::from(col_map().ops.is_init) }

#[cfg(feature = "enable_register_starks")]
#[must_use]
pub fn cpu_looked<F: Field>() -> Table<F> {
    let reg: Register<Column<F>> = col_map().map(Column::from);
    let ops = col_map().map(Column::from).ops;
    RegisterTable::new(
        vec![
            Column::ascending_sum(col_map().ops),
            reg.clk,
            reg.addr,
            reg.value,
        ],
        ops.is_read + ops.is_write,
    )
}

#[cfg(feature = "enable_register_starks")]
#[must_use]
pub fn rangecheck_looking<F: Field>() -> Vec<Table<F>> {
    let lv = col_map().map(Column::single);
    let nv = col_map().map(Column::single_next);
    vec![RegisterTable::new(
        vec![nv.clone().augmented_clk() - lv.augmented_clk()],
        nv.is_rw(),
    )]
}
