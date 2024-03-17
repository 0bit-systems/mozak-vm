use crate::columns_view::{columns_view_impl, make_col_map};
use crate::cross_table_lookup::ColumnX;

columns_view_impl!(RegisterInit);
make_col_map!(RegisterInit);
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct RegisterInit<T> {
    /// The 'address' that indexes into 1 of our 32 registers. Should only
    /// take values 0-31, so this column should be a running sum
    /// from 0 to 31 (inclusive).
    pub reg_addr: T,

    /// Value of the register.
    pub value: T,

    /// Binary column that marks a register as used to include in cross table
    /// lookups against `RegisterStark`'s `is_init` column. This also serves as
    /// an implicit range check on our register addresses.
    ///
    /// In our design, r0 should always be unused, so it's always 0.
    /// The other registers (r1-r31) should all be 1.
    pub is_looked_up: T,
}

columns_view_impl!(RegisterInitCtl);
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct RegisterInitCtl<T> {
    pub addr: T,
    pub value: T,
}

type RegisterInitColumn = ColumnX<RegisterInit<i64>>;

#[must_use]
pub fn data_for_register() -> RegisterInitCtl<RegisterInitColumn> {
    let reg = COL_MAP;
    RegisterInitCtl {
        addr: reg.reg_addr,
        value: reg.value,
    }
}

#[must_use]
pub fn filter_for_register() -> RegisterInitColumn { COL_MAP.is_looked_up }
