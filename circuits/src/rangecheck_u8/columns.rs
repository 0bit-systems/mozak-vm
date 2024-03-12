use crate::columns_view::{columns_view_impl, make_col_map};
use crate::cross_table_lookup::Column;
use crate::rangecheck::columns::RangeCheckCtl;

#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct RangeCheckU8<T> {
    /// The u8 value to be range checked
    pub value: T,

    /// The frequencies for which the accompanying value occur in
    /// the trace. This is m(x) in the paper.
    pub multiplicity: T,
}
columns_view_impl!(RangeCheckU8);
make_col_map!(RangeCheckU8);

#[must_use]
pub fn data() -> RangeCheckCtl<Column> { RangeCheckCtl::new(Column::single(col_map().value)) }

/// Column for a binary filter to indicate whether a row in the
/// [`RangeCheckTable`](crate::cross_table_lookup::RangeCheckTable).
/// contains a non-dummy value to be range checked.
#[must_use]
pub fn filter() -> Column { Column::single(col_map().multiplicity) }
