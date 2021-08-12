
//! FIXME, update and improve the comments!
//! Early stage mod for functionality that may be useful after the simple solver in 'dev' is done.
//! I'm currently using this module as a place to collect all these methods, although this will
//! probably change as I flesh out what we need, and where it belongs.
//!
//!
//!


use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::ops::Range;
use vob::Vob;

use crate::soc::bdd::Bdd;
use crate::soc::system::System;

use super::dependency_finder::DepPathFinder;

#[allow(unused_variables, dead_code)]
pub struct PostProcessing {
    soc: System,
    step: usize,
    active_area: Range<usize>,
}

impl PostProcessing{
    pub fn new(soc: System, step: usize, active_area: Range<usize>) -> Self {
        Self {
            soc,
            step,
            active_area
        }
    }

    // pub fn next_solution(&self) -> ASolution {
    // // need list of "roots": All nodes at active.start level which has the lowest non-trivial LSB
    //     // then, for each root: Itarate first through all paths from root to sink which is a path
    //     // that yields the LSB. Keep a separate DFS with paths from source to root, allowing to
    //     // choose live, but before start, whether we want to first give all input states (source to
    //     // root) per active path (root to sink), or all active paths per input states. Once a root
    //     // is "depleted": all active paths combinations with input paths have been iterated over,
    //     // once that is done, repeat the process for the next root.
    // }
}

#[allow(unused_variables, dead_code)]
pub struct ASolution {
    /// Indices correspond, and index 0 corresponds to LSB, assumed to be x0.
    lhs: Vec<Vob>,
    rhs: Vec<bool>,
}


impl Bdd {
    /// Extract a path going through any node at active_level.start, which contains the lowest LSB
    /// present at that level.
    pub fn extract_an_lsb_path(&self, active_area: &Range<usize>, step: usize) -> Vec<(Vob, bool)> {
        let top = active_area.start;
        let (arena, _) =
            self.ensure_level_is_in_arena(&top, &active_area, step);

        // Find lsb at start of active area,
        let (lsb, _) = arena.lowest_lsb_in_level(&top);
        // candidate "starting" nodes. (Sink will be the starting node in the end, but any path will
        // go through the chosen candidate.
        let mut candidate_nodes = arena.nodes_with_lsb_at_level(&top, lsb);

        let mut path: VecDeque<bool> = VecDeque::with_capacity(self.get_levels_size());
        let start_node = candidate_nodes.pop().unwrap().0;

        let mut deps = DepPathFinder::new(start_node,
                                          top, NonZeroUsize::new(step).unwrap(), &self);
        let mut root_lsb = lsb;
        let second_last = active_area.end - (step*2);


        for root_depth in (top..=second_last).step_by(step) {
            for (id, sub_path) in deps.iter() {
                let mut traversed_ones = false;
                // Check if path contains at least one 1-edge
                for edge in sub_path.iter() {
                    traversed_ones |= edge;
                }

                if traversed_ones {
                    let shifted_lsb = root_lsb - 1;
                    if arena.node_lsb(&(root_depth + step), id) == shifted_lsb {
                        root_lsb = shifted_lsb;
                        path.extend(sub_path);

                        deps = DepPathFinder::new(*id,
                                                  root_depth+step,
                                                  NonZeroUsize::new(step).unwrap(),
                                                  &self);
                        break;
                    }

                } else {
                    if arena.node_lsb(&(root_depth + step), id) == root_lsb {
                        path.extend(sub_path);
                        deps = DepPathFinder::new(*id,
                                                  root_depth+step,
                                                  NonZeroUsize::new(step).unwrap(),
                                                  &self);
                        break;
                    }
                }
            }
        }
        // Done with a path across all but the last cohort in the active area: The last would panic
        // since we don't have weights below it which it can compare to. Need to add from here on
        // and all the way to sink.
        // First, make sure that we pick a valid node from the Centurion of the last cohort, and
        // also that we fill in the path accordingly.

        // let mut c_id= Id::new(usize::max_value()); // I really wish I didn't have to use a bogus value here...

        // If the trivial path exists, pick that one for the last cohort.
        let mut trivial_root = None;

        for (id, sub_path) in deps.iter() {
            let mut traversed_ones = false;
            // Check if path is trivial
            for edge in sub_path.iter() {
                traversed_ones |= edge;
            }
            if !traversed_ones {
                path.extend(sub_path);
                trivial_root = Some(id);
                break;
            }
        }

        if trivial_root.is_some() {
            // c_id = *trivial_root.unwrap();

        } else {
            let (id, sub_path) = deps.iter().next().expect("Missing dependencies?");
            path.extend(sub_path);
        }

        // // Now we have a valid node from the last Centurion. Chose any path from the node on and all
        // // the way to the sink.
        //
        // let mut local = c_id;
        // let mut current_depth = second_last - 1 + step;
        // loop {
        //     if current_depth == self.get_sink_level_index() {
        //         break;
        //     }
        //     let node = self.levels.get(current_depth).unwrap()
        //         .get_node(&local).unwrap();
        //     if let Some(e0) = node.get_e0() {
        //         path.push_back(false);
        //         local = local;
        //         current_depth += 1;
        //         continue;
        //     }
        //     if let Some(e1) = node.get_e1() {
        //         path.push_back(true);
        //         local = local;
        //         current_depth += 1;
        //         continue;
        //     }
        //     panic!("Unable to find a valid node!");
        // }

        // Also need to add a path from source and down to the starting node found in active_area.start.
        #[cfg(debug_assertions)]
        let control = path.len();


        // let (source_id, _) = self.levels[0].iter_nodes().next().unwrap();

        // for candidate in DepPathFinder::new(*source_id,
        //                                     0,
        //                                     NonZeroUsize::new(active_area.start).unwrap(),
        //                                     &self)
        //     .into_iter() {
        //     if candidate.0 == start_node {
        //         let mut top_half = VecDeque::with_capacity(candidate.1.len() + path.len());
        //         top_half.append(&mut VecDeque::from(candidate.1));
        //         top_half.append(&mut path);
        //         path = top_half;
        //         break;
        //     }
        // }

        let mut i = top;
        let mut current_node = start_node;
        loop {
            if i == 0 {
                break;
            }

            for parent_node in self.levels[i-1].iter_nodes() {
                if let Some(e0) = parent_node.1.get_e0() {
                    if e0 == current_node {
                        path.push_front(false);
                        current_node = *parent_node.0;
                        break;
                    }
                }
                if let Some(e1) = parent_node.1.get_e1() {
                    if e1 == current_node {
                        path.push_front(true);
                        current_node = *parent_node.0;
                        break;
                    }
                }
            }
            i -= 1;
        }

        #[cfg(debug_assertions)]
            {
                assert_ne!(control, path.len(),
                           "We were unsuccessful in finding a path from source to start of active area!");
                assert_eq!(self.get_levels_size()-1, path.len(),
                           "The path is not of same length as we have levels!");
            }

        self.get_lhs().iter()
            .zip(path.iter())
            .map(|(vob, edge)| (vob.clone(), *edge) )
            .collect()

    }


    /// Will graph a path through the Shard and return the path together with the respective LHS
    /// values, as a String. Given that there are no linear dependencies left in the Shard, then
    /// this path is a Solution. If this is the only shard left in a System of CRHS Equations, then
    /// this solution is also a solution to the SoC.
    ///
    /// We give some guarantees about this path:
    ///     1) The given path will be a path which yields the lowest number of active S-boxes.
    ///     2) If this path is the all 0-path, then only the trivial solution exists in the SoC.
    ///
    /// Repeated calls to this fn will probably result in the same path being returned.
    pub fn extract_a_sol(&self, active_area: &Range<usize>, step: usize) -> String {
        let a_path = self.extract_an_lsb_path(active_area, step);

        let mut formatted = String::new();

        // Getting some metadata used in later formatting.
        let setup: Vec<(usize, usize)> = a_path.iter()
            .map(|(lhs, rhs)| {
                let iter = lhs.iter_set_bits(..);
                let count = iter.clone().count();
                let max_elem_size = iter.last().expect("Encountered an unexpected all zero LHS!")
                    .to_string().chars().count();
                (count, max_elem_size)
            })
            .collect();

        let elem_size = setup.iter()
            .map(|(_, max_elem_size)| max_elem_size).max().unwrap();
        let max_vars = setup.iter()
            .map(|(count, _)| count).max().unwrap();


        // Include "x" in the elem size;
        let elem_size = elem_size + 1;
        // Length needed for the lhs side = lhs with most number of variables * size of largest var
        // plus space for " + ". (Minus last " + ").
        let row_len = max_vars * (elem_size  + 3) - 3;
        // Setup done

        // Building the rows
        // var x0 is expected to be MSB in Vob.
        for (i, (lhs, rhs)) in a_path.iter().enumerate() {
            if i % step == 0 {
                formatted.push_str(&format!("{:->r$}\n", "", r = row_len + 3));
            }

            let mut lhs_buff = String::new();
            for int in lhs.iter_set_bits(..) {
                lhs_buff.push_str(&format!("{: >e$} + ", &format!("x{}", int), e = elem_size));
            }
            lhs_buff.pop();
            lhs_buff.pop();
            lhs_buff.pop();
            formatted.push_str(&format!("{: <r$}: {}\n", lhs_buff, *rhs as u8, r = row_len));
        }

        formatted
    }

    /// Returns a path as hex values, where the LSB is associated with the source level and MSB
    /// is associated with the sink - 1 level. The last byte will be padded with 0 if needed.
    pub fn stringify_sol_as_hex(&self, active_area: &Range<usize>, step: usize) -> String {
        let a_path = self.extract_an_lsb_path(active_area, step);

        // split into two vec's, one for lhs and one for rhs, synced by index.
        // Also maps lhs from vob to int.
        let mut lhss = Vec::new();
        let mut rhss = Vec::new();
        for (lhs, rhs) in a_path.iter() {
            let lhs_int = lhs.iter_set_bits(..).next().unwrap();
            lhss.push(lhs_int);
            rhss.push(*rhs);
        }

        // Convert rhs into vec of u8's
        let rhs_bytes = Self::bools_to_u8(&rhss);
        let s = Bdd::u8s_to_hex_separated(&rhs_bytes);
        // s = s.trim().to_owned();
        s
    }

    /// Will convert a vec of bytes into a String.
    /// Index 0 of the Vec is considered LSB, and the last index is considered MSB.
    /// Every 4 bytes (32 bits) will be separated by a space.
    ///
    /// LSB will be leftmost in the finished String, and the length of the string (excluding
    /// whitespaces) will be an even number, padding with 0's if needed.
    fn u8s_to_hex_separated(bytes: &[u8] ) -> String {
        // Reverse the byte vec, to get LSB in leftmost pos when writing
        let bytes_rev: Vec<u8> = bytes.into_iter().rev().cloned().collect();

        let mut s = String::new();
        for four_bytes in bytes_rev.chunks(4) {
            println!("Four bytes: {:?}", four_bytes);
            for byte in four_bytes.iter(){
                s.push_str(&format!("{:0>2x}", byte));
            }
            s.push_str(" ");
        }
        // remove trailing " ".
        s.pop();
        s
    }

    /// Convert a vec of bool into a vec of u8.
    /// Index 2^n of the vec is assumed as LSB of the Byte, and the last byte will be padded with
    /// 0's if needed. (If the number of bits is not a multiple of 2. Padding happens towards MSB).
    ///
    /// ```
    /// # use crush::soc::bdd::Bdd;
    ///
    /// let bools = vec![true, false, true, false];
    /// let expected_vec = vec![5, 0];
    /// // assert_eq!(expected_vec, Bdd::bools_to_u8); // Private fn
    /// ```
    fn bools_to_u8(bits: &Vec<bool>) -> Vec<u8>{
        let b = bits.chunks(8)
            .map(|v| {
                v.iter().enumerate()
                    .fold(0u8, |acc, (idx, x)| { acc | ((*x as u8) << idx)} )
            })
            .collect();
        println!("As vec of u8: {:?}", &b);
        b
    }

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bool_to_hex() {
        // index 0 is LSB, last index is MSB
        let bools = vec![
                         false, true, true, false,
                         false, false, false, false,

                         false, false, false, true,
                         false, false, false, false,

                         false, false, false, false,
                         false, true, false, false,

                         false, false, false];
        let expected_u8s = vec![6, 8, 32, 0];
        assert_eq!(expected_u8s, Bdd::bools_to_u8(&bools));
        println!("Passed first assert.");
        let expected_hex = "00200806".to_owned();
        assert_eq!(expected_hex, Bdd::u8s_to_hex_separated(&expected_u8s));
    }
}