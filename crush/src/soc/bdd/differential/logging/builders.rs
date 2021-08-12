
//! Some quick notes:
//! This is `the Builders`, responsible for "live" recording and handling of data.
//!
//! There is a hierarchy here, closely following the computational flow of how pruning is performed.
//! All but the 'top' builder, "`PruneRecordBuilder`" (PRB), can only be build through their "parent" builder.
//! This allows for any settings in PRB to trickle down through all builders. We'll see if this is


use console::{Style, style};
use std::ops::Range;

use super::records::*;

// CONFIGing
// Simple functionality to determine what gets printed during a run-through
/// Prune Record Builder print live
const PR_PRINT_LIVE: bool = false;
/// Prune Loop Record Builder print live
const PLR_PRINT_LIVE: bool = false;
/// Depth Deletion Record print live
const DDR_PRINT_LIVE: bool = false;
/// Batch Record print live
const BR_PRINT_LIVE: bool = false;


const PR_WIDTH: usize = 150;
const PLR_INLINE: usize = 20;
const DDR_INLINE: usize = 28;
const BR_INLINE: usize = 28;

const ARROW_INLINE: usize = 4;


// =================================================================================================
// =================================================================================================
// ==================================== Struct Declarations ========================================
// =================================================================================================
// =================================================================================================

pub struct PruneRecordBuilder {
    step: usize,
    active_area: Range<usize>,
    start_complexity: usize,
    complexity_target: usize,
    loop_recs: Vec<PruneLoopRecord>,
    prune_version: usize,
}



pub struct PruneLoopRecordBuilder {
    /// The widest level, and its number of nodes
    widest: (usize, usize),
    /// The second widest level, and its number of nodes
    second_widest: (usize, usize),
    /// Widest level is part of a cohort. This cohort can be found located at these depths
    cohort_range: Range<usize>,
    /// All nodes with LSB higher than or equal to this threshold may be marked for deletion
    prune_threshold: u32,
    /// Maximum number of nodes that may be marked for deletion
    roof_marked: usize,
    /// Nr of nodes that was marked for deletion, i.e. were above the prune threshold. Note that
    /// there may be, and often will be, nodes above or at the prune threshold left in the level that
    /// was not marked for deletion. This would be the case when we've hit the roof for how many
    /// to mark.
    nodes_marked: usize,

    depth_deletion_recs: Vec<DepthDeletionRecord>,
}


/// A DepthDeletionRecord holds stats from the deletion algorithm, that is, the algorithm actually
/// deleting nodes from a level. This is the second lowest logger in the current logging hierarchy.
pub struct  DepthDeletionRecBuilder {
    at_depth: usize,
    nodes_at_level: usize,
    /// Nr of nodes marked for deletion
    marked_for_deletion: usize,
    start_complexity: usize,
    complexity_target: usize,
    /// Batch records
    inner_loops: Vec<BatchRecord>,

}



/// Records statistics about a 'Batch'.
/// The deletion algorithm iterate over 'batches' in order to minimize the number of calls to the
/// reduce algorithm (which is a rather costly op), while still maintaining a level of granularity
/// of the deletion process.
///
/// This is the lowest logger in the current logging hierarchy.
///
/// Since not all the statistics are available at the start of a batch, we use a builder to build
/// the record.
pub struct  BatchRecordBuilder {
    /// Run the 'reduction' algorithm after 'reduce_after' number of node deletions.
    /// The reduction algorithm has a throughput of 10 ~ 20 nodes per second, but by bulk
    /// processing ('a batch') deletions we may increase the overall throughput of the deletion process.
    batch_size: usize,
    /// Complexity at the start of a batch.
    start_complexity: usize,
    /// Estimated nr of nodes removed for the DAG when a node is deleted. (Including the node
    /// that was deleted).
    guesstimated_deletion_rate: f64,
}




// =================================================================================================
// =================================================================================================
// ==================================== PruneRecordBuilder =========================================
// =================================================================================================
// =================================================================================================

impl PruneRecordBuilder {
    pub fn new (step: usize,
                start_complexity: usize,
                complexity_target: usize,
                active_area: Range<usize>,
                prune_version: usize,) -> Self {
        if PR_PRINT_LIVE {
            let me = Self {
                step,
                active_area,
                start_complexity,
                complexity_target,
                loop_recs: vec![],
                prune_version
            };
            println!("{}", me.print_live());
            me
        } else {
            Self {
                step,
                active_area,
                start_complexity,
                complexity_target,
                loop_recs: vec![],
                prune_version
            }
        }
    }

    pub fn finalize(self, end_complexity: usize) -> PruneRecord {

        PruneRecord {
            step: self.step,
            active_area: self.active_area,
            start_complexity: self.start_complexity,
            end_complexity,
            complexity_target: self.complexity_target,
            loop_recs: self.loop_recs,
        }
    }

    // =================================================================================================
    fn print_live(&self) -> String {
        let arrow_offset = 0;

        let mut s = String::with_capacity(100);
        s.push_str(&format!("\n\n{: >a$}{:=^w$}\n",
                            "", Self::style().apply_to(" Pruning: "), a = ARROW_INLINE, w = PR_WIDTH));

        s.push_str(&format!("{: >a$}{}{: ^w$}\n",
                            "", Self::style().apply_to("└>"),
                            &format!("{}: {}", "Prune version", self.prune_version),
                            a = ARROW_INLINE + arrow_offset,
                            w = PR_WIDTH - 4));
        s.push_str(&format!("{: >a$}{}{: ^w$}\n",
                            "", Self::style().apply_to("└>"),
                            &format!("{}: {}", "Step", self.step),
                            a = ARROW_INLINE + arrow_offset,
                            w = PR_WIDTH - 4));

        s.push_str(&format!("{: >a$}{}{: ^w$}\n",
                            "", Self::style().apply_to("└>"),
                            &format!("{}: {}", "Active area",
                                     &format!("[{}..{})",
                                              self.active_area.start, self.active_area.end)),
                            a = ARROW_INLINE + arrow_offset,
                            w = PR_WIDTH - 4));

        s.push_str(&format!("{: >a$}{}{: ^w$}\n",
                            "", Self::style().apply_to("└>"),
                            &format!("{}: {}", "Start complexity", self.start_complexity),
                            a = ARROW_INLINE + arrow_offset,
                            w = PR_WIDTH - 4));

        s.push_str(&format!("{: >a$}{}{: ^w$}\n",
                            "", Self::style().apply_to("└>"),
                            &format!("{}: {}", "Target complexity", self.complexity_target),
                            a = ARROW_INLINE + arrow_offset,
                            w = PR_WIDTH - 4));
        s.shrink_to_fit();
        s
    }

    #[allow(dead_code)]
    fn live_report_finalized(_rec: &PruneRecord) {
        todo!();
        // How many loops, unique levels (nr, and high and low), level most worked on?,
        // nodes deleted, avg deleted per removed node, ...
    }

    fn style() -> Style {
        Style::new().magenta()
    }

// =================================================================================================

    pub fn new_prune_loop_rec_builder(&self, widest: (usize, usize), second_widest: (usize, usize),
                                      cohort_range: Range<usize>,
                                      prune_threshold: u32, roof_marked: usize, nodes_marked: usize)
        -> PruneLoopRecordBuilder {

        PruneLoopRecordBuilder::new(widest, second_widest, cohort_range,
                                    prune_threshold, roof_marked, nodes_marked)
    }

    pub fn register_loop_rec(&mut self, rec: PruneLoopRecord) {
        self.loop_recs.push(rec);
    }

}

// =================================================================================================
// =================================================================================================
// ==================================== PruneLoopRecordBuilder =====================================
// =================================================================================================
// =================================================================================================
impl PruneLoopRecordBuilder {
    fn new (widest: (usize, usize), second_widest: (usize, usize), cohort_range: Range<usize>,
            prune_threshold: u32, roof_marked: usize, nodes_marked: usize) -> PruneLoopRecordBuilder {
        let rec =
        PruneLoopRecordBuilder {
            widest,
            second_widest,
            cohort_range,
            prune_threshold,
            roof_marked,
            nodes_marked,
            depth_deletion_recs: vec![]
        };

        if PLR_PRINT_LIVE {
            print!("{}", rec.live_header());
            print!("{}", rec.live_report());
        }
        rec
    }

    pub fn finalize(self, end_complexity: usize ) -> PruneLoopRecord {

        PruneLoopRecord {
            end_complexity,
            widest: self.widest,
            second_widest: self.second_widest,
            cohort_range: self.cohort_range,
            prune_threshold: self.prune_threshold,
            roof_marked: self.roof_marked,
            nodes_marked: self.nodes_marked,
            depth_deletion_recs: self.depth_deletion_recs,
        }
    }

// =================================================================================================
    fn live_header(&self) -> String {

        let arrow_offset = 2;
        let mut s = format!("\n\n{: >a$}{}\n", "", style("Setting up next loop:").yellow(), a = ARROW_INLINE);
        s.push_str(&format!("{: >a$}{}{: >w$}     Widest Level    |     Second Widest     |   Prune Threshold  |  Roof Marked  |  Nodes Marked \n",
                            "", style("└>").yellow(), "", a = ARROW_INLINE + arrow_offset,
                            w = PLR_INLINE - ARROW_INLINE - arrow_offset));
        s
    }

    fn live_report(&self) -> String {
        let arrow_offset = 2;
        let mut s = String::with_capacity(90);
        s.push_str(&format!("{: >a$}{}{: >w$}{: ^21}|{: ^23}|{: ^20}|{: ^15}|{: ^15}\n",
                            "", style("└>").yellow(), "",
                            &format!("Depth: {}", self.widest.0),
                            &format!("Depth: {}", self.second_widest.0),
                            self.prune_threshold, self.roof_marked, self.nodes_marked,
                            a = ARROW_INLINE + arrow_offset,
                            w = PLR_INLINE - ARROW_INLINE - arrow_offset,
        ));
        s.push_str(&format!("{: >a$}{}{: >w$}{: ^21}|{: ^23}|{: ^50}\n",
                            "", style("└>").yellow(), "",
                            &format!("Size: {}", self.widest.1),
                            &format!("Size: {}", self.second_widest.1),
                            &format!("Level is part of Cohort located at [{}..{}).",
                                     self.cohort_range.start, self.cohort_range.end),
                            a = ARROW_INLINE + arrow_offset,
                            w = PLR_INLINE - ARROW_INLINE - arrow_offset,
        ));

        s.shrink_to_fit();
        s
    }

    // fn live_report_finalized() -> String {
    //     todo!()
    // }
// =================================================================================================

    pub fn new_depth_deletion_builder(&self,
                                      at_depth: usize,
                                      nodes_at_level: usize,
                                      marked_for_deletion: usize,
                                      start_complexity: usize,
                                      complexity_target: usize)

                                      -> DepthDeletionRecBuilder {
        DepthDeletionRecBuilder::new(at_depth, nodes_at_level, marked_for_deletion, start_complexity, complexity_target)
    }

    pub fn register_depth_deletion_rec(&mut self, rec: DepthDeletionRecord) {
        self.depth_deletion_recs.push(rec);
    }
}

// =================================================================================================
// =================================================================================================
// ================================== DepthDeletionRecBuilder ======================================
// =================================================================================================
// =================================================================================================
impl DepthDeletionRecBuilder {

    /// Returns a new DepthDeletionRecBuilder.
    /// If the live printing option is set for DepthDeletionRec's, then the live_header is printed
    fn new(at_depth: usize,
           nodes_at_level: usize,
           marked_for_deletion: usize,
           start_complexity: usize,
           complexity_target: usize,
    ) -> Self {
        if DDR_PRINT_LIVE {
            let me = Self {
                at_depth,
                nodes_at_level,
                marked_for_deletion,
                start_complexity,
                complexity_target,
                inner_loops: vec![],
            };
            print!("{}", me.live_report());
            me
        } else {
            Self {
                at_depth,
                nodes_at_level,
                marked_for_deletion,
                start_complexity,
                complexity_target,
                inner_loops: vec![],
            }
        }

    }

    /// Consumes the Builder and returns a new DepthDeletionRecord.
    pub fn finalize(self, end_complexity: usize) -> DepthDeletionRecord {
        if DDR_PRINT_LIVE {
            print!("{}", self.live_report_finalized(end_complexity));
        }
        if BR_PRINT_LIVE {
            let mut s = String::new();
            s.push_str(&format!("\n{: ^w$} Note that all marked nodes have been removed, either directly (\"Removed\") or through the reduction process (\"Missed\").\n",
                                "", w = BR_INLINE + ARROW_INLINE));
            s.push_str(&format!("{: ^w$} Note that if the last entry shows 0 actual deletions, than that entry is a superfluous entry.\n",
                                "", w = BR_INLINE + ARROW_INLINE));
            print!("{}", s);
        }

        DepthDeletionRecord {
            at_depth: self.at_depth,
            nodes_at_level: self.nodes_at_level,
            marked_for_deletion: self.marked_for_deletion,
            start_complexity: self.start_complexity,
            end_complexity,
            inner_loops: self.inner_loops
        }
    }

// =================================================================================================

    pub fn new_batch_builder(&self,
                             batch_size: usize,
                             start_complexity: usize,
                             guesstimated_deletion_rate: f64) -> BatchRecordBuilder {
        if BR_PRINT_LIVE {
            if self.inner_loops.is_empty() {
                // Print with header
                let batch = BatchRecordBuilder::new(batch_size,
                                                    start_complexity,
                                                    guesstimated_deletion_rate,
                                                    true);
                print!("{}", batch.live_report());
                batch
            } else {
                // Do not print the header
                let batch = BatchRecordBuilder::new(batch_size,
                                                    start_complexity,
                                                    guesstimated_deletion_rate,
                                                    false);
                print!("{}", batch.live_report());
                batch
            }
        } else {
            BatchRecordBuilder::new(batch_size, start_complexity, guesstimated_deletion_rate, false)
        }

    }

    pub fn register_batch(&mut self, batch: BatchRecord) {
        self.inner_loops.push(batch);
    }

// =================================================================================================

    pub fn live_report(&self) -> String {
        let arrow_offset = 4;

        let mut s = format!("\n\n{: >a$}{}\n", "",
                            Self::style().apply_to("Deleting nodes:"), a = ARROW_INLINE);
        s.push_str( &format!("{: >a$}{}{: >w$}{: ^88}\n", "",
                             Self::style().apply_to("└>"), "",
                            &format!("Deleting nodes from level at depth: {}",
                                     Self::style().apply_to(self.at_depth)),
                            a = ARROW_INLINE + arrow_offset,
                            w = DDR_INLINE - ARROW_INLINE - arrow_offset));
        s.push_str(&format!(
            "{: >a$}{}{: >w$} Nodes at level |  Marked  | % marked | Complexity Target | Current Complexity \n",
            "", Self::style().apply_to("└>"), "",
            a = ARROW_INLINE + arrow_offset,
            w = DDR_INLINE - ARROW_INLINE - arrow_offset));
        s.push_str(&format!("{: >a$}{}{: >w$}{: ^16}|{: ^10}|{: ^10.p$}|{: ^19}|{: ^20} \n",
                            "", Self::style().apply_to("└>"), "",
                            self.nodes_at_level,
                            self.marked_for_deletion,
                            (self.marked_for_deletion as f64 / self.nodes_at_level as f64)  * 100_f64,
                            self.complexity_target,
                            self.start_complexity,
                            p = 2,
                            a = ARROW_INLINE + arrow_offset,
                            w = DDR_INLINE - ARROW_INLINE - arrow_offset));
        s.shrink_to_fit();
        s
    }

    pub fn live_report_finalized(&self, end_complexity: usize) -> String {
        let arrow_offset = 4;

        let removed: usize = self.inner_loops.iter()
            .map(|batch| batch.marked_removed)
            .sum();

        let missed: usize = self.inner_loops.iter()
            .map(|batch| batch.missed_removed)
            .sum();
        let nodes_removed = self.start_complexity - end_complexity;
        let avg_dels = nodes_removed as f64 / removed as f64;
        // let avg_del: f64 = self.inner_loops.iter()
        //     .map(|batch| batch.deletion_rate)
        //     .sum();
        // let avg_del = avg_del / (self.inner_loops.len() as f64);


        let mut s = String::new();
        s.push_str(&format!("{: >a$}{}{: ^w$}{: ^14}|{: ^18}|{: ^23}|{: ^20}|{: ^25}\n",
                            "", Self::style().apply_to("Done:"), "",
                            "Nr Of Batches",
                            "End complexity",
                            "Total nodes removed",
                            "Avg deletions", // ≈
                            "Tot Marked Removed/Missed",
                            a = ARROW_INLINE + arrow_offset - 2,
                            w = BR_INLINE - ARROW_INLINE - arrow_offset + 4 - 1));
        s.push_str(&format!("{: >a$}{}{: ^w$}{: ^14}|{: ^18}|{: ^23}|{: ^20.2}|{: ^25}\n",
                            "", Self::style().apply_to("└>"), "",
                            self.inner_loops.len(),
                            end_complexity,
                            nodes_removed,
                            avg_dels,
                            // &format!("ds {: ^2.2} / d {: ^2.2}", avg_dels, avg_del),
                            &format!("{}/{}", removed, missed),
                            a = ARROW_INLINE + arrow_offset,
                            w = BR_INLINE - ARROW_INLINE - arrow_offset + 4) );
        s.shrink_to_fit();
        s
    }

    fn style() -> Style {
        Style::new().cyan()
    }
}



// =================================================================================================
// =================================================================================================
// ==================================== BatchRecordBuilder =========================================
// =================================================================================================
// =================================================================================================

impl BatchRecordBuilder {
    fn new(batch_size: usize,
           start_complexity: usize,
           guesstimated_deletion_rate: f64,
           print_header: bool,
    ) -> BatchRecordBuilder {
        if print_header && BR_PRINT_LIVE {
            let me = BatchRecordBuilder {
                batch_size,
                start_complexity,
                guesstimated_deletion_rate,
            };
            print!("{}", me.live_header());
            me
        } else {
            BatchRecordBuilder {
                batch_size,
                start_complexity,
                guesstimated_deletion_rate,
            }
        }
    }

    pub fn finalize(self, actual_deletion_rate: f64, marked_removed: usize, missed_removed: usize) -> BatchRecord {
        if BR_PRINT_LIVE {
            print!("{}", self.live_report_finalized(actual_deletion_rate, marked_removed, missed_removed));
        }
        BatchRecord {
            batch_size: self.batch_size,
            start_complexity: self.start_complexity,
            guesstimated_deletion_rate: self.guesstimated_deletion_rate,
            deletion_rate: actual_deletion_rate,
            marked_removed,
            missed_removed,
        }

    }

    fn live_header(&self) -> String {
        let arrow_offset = 6;
        let mut s = format!("\n\n{: >a$}{}\n",
                            "", Self::style().apply_to("Batch deletions:"), a = ARROW_INLINE);
        s.push_str(&format!("{: >a$}{}{: >w$}Batch size    |    Complexity    |   Expected deletions  |  Actual deletions  |  Marked Removed/Missed  \n",
                        "", Self::style().apply_to("└>"), "",
                        a = ARROW_INLINE + arrow_offset,
                        w = BR_INLINE - arrow_offset));

        s
    }

    fn live_report(&self) -> String {
        let precision = 2;
        let arrow_offset = 6;

        let mut s = String::with_capacity(63);
        s.push_str(&format!("{: >a$}{}{: >w$}{: ^b$}|",
                            "", Self::style().apply_to("└>"), "",
                            self.batch_size,
                            a = ARROW_INLINE + arrow_offset,
                            w = BR_INLINE - arrow_offset, b = 14));
        s.push_str(&format!("{: ^b$}|", self.start_complexity, b = 18));
        s.push_str(&format!("{: ^b$.p$}|", self.guesstimated_deletion_rate, p = precision, b = 23));

        s.shrink_to_fit();
        s
    }

    fn live_report_finalized(&self, actual_deletion_rate: f64, marked_removed: usize,
                             missed_removed: usize) -> String {
        let precision = 2;

        let mut s = String::with_capacity(45);
        s.push_str(&format!("{: ^20.p$}|{: ^25}\n", actual_deletion_rate,
                            &format!("{}/{}", marked_removed, missed_removed),
                            p = precision));
        s.shrink_to_fit();
        s
    }


    fn style() -> Style {
        Style::new().green()
    }

}