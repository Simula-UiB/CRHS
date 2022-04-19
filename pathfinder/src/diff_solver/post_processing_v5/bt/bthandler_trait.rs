//! Adds support for different S-boxes in the same cipher

use std::collections::BTreeMap;

use crate::diff_solver::post_processing_v5::bt::BaseTable;

pub trait BTHandler: Sync {

    fn nr_of_rounds(&self) -> usize;

    fn bt(&self, round: usize, sbox_pos: usize) -> &BaseTable;

    fn prob_exponents(&self, round: usize, sbox_pos: usize) -> &BTreeMap<usize, usize>;

    fn k(&self, round: usize, sbox_pos: usize) -> f64;

    /// Nr of S-boxes in an S-box layer times the number of bits in each S-box.
    /// OBS, We currently assume that all S-boxes are of the same size, with equal nr of in bits and
    /// out bits.
    /// TODO at some point, allow different size in and out
    fn sbox_layer_size(&self) -> usize;

    // Why is this one an Option, when the rest isn't? TODO
    ///
    fn prob_exponents_for_entry(&self, round: usize, sbox_pos: usize, entry: usize) -> Option<usize>;

}