use vob::Vob;

use pathfinder::code_gen::{LLHandler, SBoxHandler};
use pathfinder::code_gen::gsf::GenericShard;

pub mod llb;
pub mod cg;


/// A linear layer handler/cache which implements the LLHandler trait from the 'Differential' crate.
/// Allows for different linear layers for each round.
pub struct LlHandler {
    block_size: Vec<usize>,
    linear_layers: Vec<Box<dyn Fn(Vec<Vob>) -> Vec<Vob>>>,
}

impl LLHandler for LlHandler {
    #[inline]
    fn block_size(&self, round: usize) -> usize {
        *(self.block_size.get(round).unwrap())
    }

    #[inline]
    fn apply_linear_layer(&self, round: usize, state: Vec<Vob<usize>>) -> Vec<Vob<usize>> {
        let ll = self.linear_layers.get(round).unwrap();
        ll(state)
    }
}

pub struct SboxHandler {
    generic_shards: Vec<Vec<GenericShard>>,

}

impl SBoxHandler for SboxHandler {
    #[inline]
    fn num_sboxes(&self, round: usize) -> usize {
        self.generic_shards.get(round).unwrap().len()
    }

    #[inline]
    fn sbox_size_in(&self, round: usize, pos: usize) -> usize {
        self.generic_shards.get(round).unwrap().get(pos).unwrap().size_in()
    }

    #[inline]
    fn sbox_size_out(&self, round: usize, pos: usize) -> usize {
        self.generic_shards.get(round).unwrap().get(pos).unwrap().size_out()
    }

    #[inline]
    fn bt_generic_shard(&self, round: usize, pos: usize) -> GenericShard {
        self.generic_shards.get(round).unwrap().get(pos).unwrap().clone()
    }
}
