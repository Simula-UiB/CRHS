
//! Generic Shard Factory

use std::collections::{BTreeMap, HashMap};
use std::iter::Iterator;

use vob::Vob;

use crush::soc::bdd::Bdd as Shard;
use crush::soc::Id;
use crush::soc::utils::{BddSpec, LevelSpec, NodeSpec};

use crate::diff_solver::post_processing_v5::BaseTable;

#[derive(Debug, Clone)]
pub struct GenericShard {
    size_in: usize,
    size_out: usize,
    shard: Shard,
}

impl GenericShard {
    #[inline]
    pub fn new(table: &BaseTable, size_in: usize, size_out: usize) -> Self {
        Self {
            size_in,
            size_out,
            shard: Self::make_generic_shard(table, size_in, size_out),
        }
    }

    #[inline]
    pub fn size_in(&self) -> usize {
        self.size_in
    }

    #[inline]
    pub fn size_out(&self) -> usize {
        self.size_out
    }

    // Note, caller must ensure that all LHSs are of valid (and equal) length.
    #[inline]
    pub fn into_specific<T>(mut self, in_lhss: &mut T, out_lhss: &mut T, id: Id) -> Shard
        where
            T: Iterator<Item = Vob>,
    {
        self.shard.set_id(id);
        // Update the LHSs for in bits
        for i in 0..self.size_in {
            self.shard.set_lhs_level_from_vob(i, in_lhss.next().unwrap());
        }

        // Update the LHSs for out bits
        for i in self.size_in..self.size_in+self.size_out {
            self.shard.set_lhs_level_from_vob(i, out_lhss.next().unwrap());
        }

        self.shard
    }



    fn make_generic_shard(table: &BaseTable, size_in: usize, size_out: usize) -> Shard {
        // === A note on the "nodes", and how they work: ===
        // A node here is based on the NodeSpec struct from crush::utils, which roughly is
        // (my_id; e0_id, e1_id). e0 (e1) is the child node at the end of the 0-(1-)edge. A '0' represents
        // 'no child', meaning that the id '0' is a reserved 'key id' (="keyword" of sorts).
        //
        // I believe the use of '0' as 'no child' is used as backwards compatibility with the '.bdd'
        // format. (I believe .bdd was initially designed for the original CRHS research tool, back
        // when BDD's still were the term used also for CRHS's and SoC's.)
        //
        // I mention this to give an understanding for why Option isn't used, as None would be a very
        // natural choice for 'no child', both here and in Crush.
        // === ===


        // ================= Fill top half ===================
        // This will make the full span of 2^in_length paths, meaning that we need in_length + 1 levels
        // of nodes. The nodes on the first 'in_length' levels will be complete (have both e0 and e1
        // children set), while the nodes on the last level will have no children set.

        // Next available id for child
        let mut next_child = 2;
        // Next available id for parent
        let mut parent_id = 1;
        let mut level_arena = BTreeMap::new();
        let mut node_arena = HashMap::new();

        for lvl in 0..size_in {
            for _ in 0..2_u32.pow(lvl as u32) {
                node_arena.insert(parent_id, (next_child, next_child + 1));
                let lvel = level_arena.entry(lvl).or_insert(Vec::new());
                lvel.push(parent_id);
                parent_id += 1;
                next_child += 2;
            }
        }
        // Fill last level (last level contains the end nodes of the path, and therefore points to no
        // children of their own). These nodes will be the "start nodes" for the out-paths.
        for c_id in 2_u32.pow(size_in as u32)..next_child {
            node_arena.insert(c_id, (0, 0));
            let lvel = level_arena.entry(size_in).or_insert(Vec::new());
            lvel.push(c_id);
        }

        // ============= Top Levels Filled ====================

        // =========== Fill Remaining Levels ====================
        // Row index yields the in-path, which should now be present in the node_arena
        // Column index yields the out-path, and needs to be connected to the corresponding in-path.
        // As per usual for a BaseTable (DDT, LAT, etc), an in-path can yield an out-path if the entry
        // at table[row idx][col idx] is non-zero.

        // The end node of an in-path will be the start node of an corresponding out-path.
        // See notes on fn rev() for the idea of how to find the "start node" for the output path.
        let offset = 2_u32.pow(size_in as u32);
        let sink_id = next_child;
        next_child += 1;
        let sink_depth = size_in + size_out;


        for row_idx in 0..table.nr_of_rows() {
            let row = table.row(row_idx).unwrap();

            for col_idx in 0..row.len() {
                let is_connected = row.get(col_idx).unwrap();

                // The entry is 0, in-value CANNOT yield out-value => continue
                if *is_connected == 0 {
                    continue;
                }

                // OBS, remember that lsb is rightmost now.
                // (I skip using rev() so that I don't have to deal with the surplus leading 0's).
                let out_path = format!("{:0>w$b}", col_idx, w = size_out);

                let mut child_depth = size_in + 1; // FIXME off by one?

                // Path to walk
                let mut edges = out_path.chars().rev();
                // Start node
                let mut parent_id = offset + Self::rev_nr_bits(row_idx, size_in);
                let mut parent_node = node_arena.get_mut(&parent_id).unwrap();

                // The current edge will be used in several places
                let mut current_edge = edges.next().unwrap()
                    .to_digit(2).unwrap();

                // Get child of start node
                let mut child_id = match current_edge {
                    0 => parent_node.0,
                    1 => parent_node.1,
                    _ => panic!("Somehow a radix 2 became non binary"),
                };


                // Walk the existing path for as long as it exists
                while child_id != 0 {
                    // Update child depth
                    child_depth += 1;
                    // Update parent
                    parent_id = child_id;
                    parent_node = node_arena.get_mut(&parent_id).unwrap();
                    // Update edge
                    current_edge = edges.next()
                        // Shouldn't be able to walk the path all the way to sink!
                        .expect("Did we unexpectedly reach/pass the sink node?")
                        .to_digit(2).unwrap();
                    // Update child
                    child_id = match current_edge {
                        0 => parent_node.0,
                        1 => parent_node.1,
                        _ => panic!("Somehow a radix 2 became non binary"),
                    };
                }

                // ======= End of existing path =======

                // Building the remainder of the path:
                // Current state: parent id != 0, but the child along current_edge is 0 => child_id = 0;
                // => parent_node.current_edge = 0 => Need to make next child.

                if child_depth == sink_depth {
                    match current_edge {
                        0 => parent_node.0 = sink_id,
                        1 => parent_node.1 = sink_id,
                        _ => panic!("Somehow a radix 2 became non binary"),
                    };
                    continue;
                }


                // Insert id of next child into parent
                match current_edge {
                    0 => parent_node.0 = next_child,
                    1 => parent_node.1 = next_child,
                    _ => panic!("Somehow a radix 2 became non binary"),
                };

                // Make the rest
                loop {
                    match edges.next() {
                        // We have another edge to create a child node at the end of:
                        Some(e) => {
                            // Update current edge
                            current_edge = e.to_digit(2).unwrap();

                            // "Make" the child
                            match current_edge {
                                0 => {
                                    node_arena.insert(next_child, (next_child + 1, 0));
                                    let lvel = level_arena.entry(child_depth).or_insert(Vec::new());
                                    lvel.push(next_child);
                                },
                                1 => {
                                    node_arena.insert(next_child, (0, next_child + 1));
                                    let lvel = level_arena.entry(child_depth).or_insert(Vec::new());
                                    lvel.push(next_child);
                                },
                                _ => panic!("Somehow a radix 2 became non binary"),
                            };
                            child_depth += 1;
                            next_child += 1;
                        },
                        None => {
                            // Last parent points to a non-existing child when it should point to sink
                            // Connect parent to sink instead:
                            let parent = node_arena.get_mut(&(next_child - 1)).unwrap();
                            match current_edge {
                                0 => parent.0 = sink_id,
                                1 => parent.1 = sink_id,
                                _ => panic!("Somehow a radix 2 became non binary"),
                            };
                            break;
                        }
                    }
                }
            }
        }

        // Push sink into arena
        node_arena.insert(sink_id, (0, 0));
        level_arena.insert(sink_depth, vec![sink_id]);

        // === The DAG is now finished, time to make it into a shard ===
        // Remember to reduce it afterwards!

        let mut level_specs = Vec::new();
        for level in level_arena.values() {
            let mut rhs = Vec::new();
            for node_id in level.iter() {
                let (e0, e1) = node_arena.get(node_id).unwrap();
                let node = NodeSpec::new(Id::new(node_id.clone() as usize),
                                         Id::new(e0.clone() as usize),
                                         Id::new(e1.clone() as usize),
                );
                rhs.push(node);
            }
            level_specs.push(LevelSpec::new(vec![], rhs));
        }

        let mut shard_spec = BddSpec::new(Id::new(0), level_specs);
        // We're using a dummy for nr of vars. It is up to the user of the this generic shard to update
        // both the LHS, but also the nr of vars, in accordance with the cipher they use.
        let mut shard = crush::soc::utils::build_bdd_from_spec(&mut shard_spec, 1);
        shard.remove_all_dead_ends_start(sink_depth - 1);
        shard.remove_orphans_start(1);
        shard.merge_equals_node_start(sink_depth - 1); // TODO verify

        // let path = &["out_results", "generic_shard_test.dot"].iter().collect();
        // crush::soc::utils::print_bdd_to_graphviz(&shard, &path);

        shard
    }

    /// Let's call the end node of the path given by the 'in-value' to the S-box for the "start node" of
    /// the path given by the 'out-value' of the S-box. We would like to easily find this node.
    /// If the LSB had been at level 'in_length', we could've easily calculated the "start node"
    /// by the formula '2^in_length + column index'. However, our LSB is at level 0, meaning
    /// that this formula will yield the wrong start node.
    ///
    /// In theory, we could use nn.reverse_bits(), and update the formula to be
    /// '2^in_length + column index.reverse_bits()'.
    /// However, using .reverse_bits() on a uX will reverse *all* the X bits, whereas we only care
    /// about the 'in_length' last bits.
    ///
    /// Therefore:
    /// This fn accepts a digit, and a number specifying how many bits we care about. The returned value
    /// will have reversed the bits of the 'bits' last bits of the given 'digit'. (The rest is all 0's).
    fn rev_nr_bits(digit: usize, nr_bits: usize) -> u32 {
        // Not ideal to go through a String, but this should be rather cold code, so I'm not worried.
        let s = format!("{:0>w$b}", digit, w = nr_bits);
        let d: String = s.chars().rev().collect();
        u32::from_str_radix(&d, 2).expect("Binary string to usize failed")
    }
}


#[cfg(test)]
mod test {
    use std::convert::TryFrom;

    use crush::soc::{utils};

    use crate::ciphers::prince;

    use super::BaseTable;

    #[test]
    fn test_generic_generator_using_prince() {
        let bt = BaseTable::try_from(prince::ddt_raw()).unwrap();
        let actual = super::GenericShard::make_generic_shard(&bt, 4, 4);

        let path_to_expected = &["SoCs", "DDTprinceS_generic.bdd"].iter().collect();
        let sys_spec = utils::parse_system_spec_from_file(&path_to_expected);
        let mut soc = utils::build_system_from_spec(sys_spec);
        let expected = soc.drain_bdds().next().unwrap().1.into_inner();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_generic_generator_using_prince_inv() {
        let bt = BaseTable::try_from(prince::ddt_inverse_raw()).unwrap();
        let actual = super::GenericShard::make_generic_shard(&bt, 4, 4);

        let path_to_expected = &["SoCs", "DDTprinceSinv_generic.bdd"].iter().collect();
        let sys_spec = utils::parse_system_spec_from_file(&path_to_expected);
        let mut soc = utils::build_system_from_spec(sys_spec);
        let expected = soc.drain_bdds().next().unwrap().1.into_inner();

        assert_eq!(actual, expected);
    }
}

