use itertools::Itertools;
use plonky2::field::types::Field;

/// Pad the trace to a power of 2.
///
/// # Panics
/// There's an assert that makes sure all columns passed in have the same
/// length.
#[must_use]
pub fn pad_trace<F: Field>(mut trace: Vec<Vec<F>>) -> Vec<Vec<F>> {
    assert!(trace
        .iter()
        .tuple_windows()
        .all(|(a, b)| a.len() == b.len()));
    for col in &mut trace {
        if let (Some(padded_len), Some(&last)) = (col.len().checked_next_power_of_two(), col.last())
        {
            col.extend(vec![last; padded_len - col.len()]);
        }
    }
    trace
}

#[must_use]
pub fn pad_trace_with_default<Row: Default + Clone>(mut trace: Vec<Row>) -> Vec<Row> {
    trace.resize(trace.len().next_power_of_two(), Row::default());
    trace
}

#[must_use]
pub(crate) fn from_u32<F: Field>(x: u32) -> F { Field::from_noncanonical_u64(x.into()) }
