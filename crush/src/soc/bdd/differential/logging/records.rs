use std::fmt::{self, Formatter};
use std::ops::Range;

#[derive(Debug)]
pub struct PruneRecord {
    pub(super) step: usize,
    pub(super) active_area: Range<usize>,
    pub(super) start_complexity: usize,
    pub(super) end_complexity: usize,
    pub(super) complexity_target: usize,
    pub(super) loop_recs: Vec<PruneLoopRecord>,
}

impl PruneRecord {
    #[inline]
    pub fn register_loop(&mut self, rec: PruneLoopRecord) {
        // rec.print_verbose(5);
        self.loop_recs.push(rec);
    }

    pub fn print_verbose(&self) {
        let inline = 15;
        println!("Pruning:");
        println!("{: >w$}{}", "Step: ", self.step, w=inline);
        println!("{: >w$}{}", "Complexity target: ", self.complexity_target, w=inline);
        println!("{: >w$} start: {} end: {}", "Active area", self.active_area.start, self.active_area.end, w=inline);
        println!("{: >w$}{: >5}", "Start complexity: ", self.start_complexity, w=inline);

        println!("\n{: >w$}{}", "Nr of loops: ", self.loop_recs.len(), w=inline);
        for loop_rec in self.loop_recs.iter() {
            // loop_rec.print_verbose(25);
            println!();
        }

        println!("{: >w$}{: >5}", "End complexity: ", self.end_complexity, w = inline);

    }

    // pub fn print_live(&self) {
    //     let inline = 15;
    //     println!("Pruning:");
    //     println!("{: >w$}{}", "Step: ", self.step, w=inline);
    //     println!("{: >w$}{}", "Complexity target: ", self.complexity_target, w=inline);
    //     println!("{: >w$} start:{} end:{}", "Active area", self.active_area.start, self.active_area.end, w=inline);
    //     println!("{: >w$}{: >5}", "Start complexity: ", self.start_complexity, w=inline);
    // }
}

impl fmt::Display for PruneRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // quickfix
        self.print_verbose();
        writeln!(f, "{}", "")
    }
}

// =============================================================================================
// ================================== PruneLoopRecord ==========================================
// =============================================================================================

#[derive(Debug)]
pub struct PruneLoopRecord {
    pub(super) end_complexity: usize,
    /// The widest level, and its number of nodes
    pub(super) widest: (usize, usize),
    /// The second widest level, and its number of nodes
    pub(super) second_widest: (usize, usize),
    /// Widest level is part of a cohort. This cohort can be found located at these depths
    pub(super) cohort_range: Range<usize>,
    /// All nodes with LSB higher than or equal to this threshold may be marked for deletion
    pub(super) prune_threshold: u32,
    /// Maximum number of nodes that may be marked for deletion
    pub(super) roof_marked: usize,
    /// Nr of nodes that was marked for deletion, i.e. were above the prune threshold. Note that
    /// there may be, and often will be, nodes above or at the prune threshold left in the level that
    /// was not marked for deletion. This would be the case when we've hit the roof for how many
    /// to mark.
    pub(super) nodes_marked: usize,
    /// Any DepthDeletionRecord's are kept here. Should be at least one.
    pub(super) depth_deletion_recs: Vec<DepthDeletionRecord>,
}

// =============================================================================================
// ================================== Depth Deletion Record ====================================
// =============================================================================================

///
#[derive(Debug)]
pub struct DepthDeletionRecord {
    pub(super) at_depth: usize,
    pub(super) nodes_at_level: usize,
    /// Nr of nodes marked for deletion
    pub(super) marked_for_deletion: usize,
    pub(super) start_complexity: usize,
    pub(super) end_complexity: usize,
    /// Batch records
    pub(super) inner_loops: Vec<BatchRecord>,
}

impl DepthDeletionRecord {

    /// Returns the number of marked nodes that actually were deleted.
    /// This may be different from the given number of marked nodes for deletion, as marked nodes
    /// may have been deleted as part of a reduce op a stage prior to this DepthDeletion op.
    pub fn marked_deleted(&self) -> usize {
        self.inner_loops.iter()
            .map(|rec| rec.marked_removed)
            .sum()
    }
}

/// Recordings of values used to process a batch.
///
/// Running the reduction algorithm after each deletion of the nodes is very time consuming.
/// I estimate that the throughput of such a way of operating allows for the deletion of about
/// 10 to 100 nodes per second. Which is way too slow. We therefore bulk delete a 'batch' of nodes
/// before running the reduction algorithm. How many nodes are deleted as part of a batch is
/// determined in a dynamical fashion, and here we record the batch size, along with other
/// parameters which were used to calculate the batch size.
///
/// All values are calculated at the beginning of a batch.
#[derive(Debug)]
pub struct BatchRecord {
    /// Run the 'reduction' algorithm after 'reduce_after' number of node deletions.
    /// The reduction algorithm has a throughput of 10 ~ 20 nodes per second, but by bulk
    /// processing ('a batch') deletions we may increase the overall throughput of the deletion process.
    pub(super) batch_size: usize,
    /// Complexity at the start of a batch.
    pub(super) start_complexity: usize,
    /// Estimated nr of nodes removed for the DAG when a node is deleted. (Including the node
    /// that was deleted).
    pub(super) guesstimated_deletion_rate: f64,
    /// Actual average of nodes removed from the CHRS equation per deleted marked node.
    pub(super) deletion_rate: f64,
    /// Number of marked nodes that was actually removed (Some marked may have been removed as part
    /// of a call to the reduce op, if other levels had marked nodes deleted prior to this).
    pub(super) marked_removed: usize,
    /// Number of marked nodes attempted to delete in this batch, but that had already been
    /// removed as part of a reduce op prior.
    pub(super) missed_removed: usize,
}
