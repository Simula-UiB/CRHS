use vob::Vob;
use crate::code_gen::gsf::GenericShard;

pub mod soc_gen;
pub mod gsf;


pub trait SBoxHandler {
    /// Returns the number of S-boxes in the non-linear layer.
    fn num_sboxes(&self, round: usize) -> usize;

    fn sbox_size_in(&self, round: usize, pos: usize) -> usize;

    fn sbox_size_out(&self, round: usize, pos: usize) -> usize;

    // prefix bt to remind us that this generic shard is based on a bt. Even though the generic shard
    // itself shouldn't behave any different than any other generic shard, it helps remind us of the
    // context we're in. (We don't want something that isn't based on a base table ;) ).
    fn bt_generic_shard(&self, round: usize, pos: usize) -> GenericShard;
}

/// LinearLayerHandler
/// Allows for manipulation of state through application of the linear layer(s).
/// Also contains meta-data about the linear layer(s).
pub trait LLHandler {
    // We expect nr_of_rounds+1 available block_sizes
    fn block_size(&self, round: usize) -> usize;
    // We expect nr_of_rounds call to apply_linear_layer to be valid. This may change if a post last
    // non-linear layer call ever becomes needed.
    fn apply_linear_layer(&self, round: usize, state: Vec<Vob>) -> Vec<Vob>;

}
