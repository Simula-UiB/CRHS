use std::ops::Range;
use std::sync::Arc;

use crush::soc::bdd::{Bdd as Shard};
use crush::soc::bdd::differential::{Depth, DepPathFinder};
use crush::soc::bdd::differential::wd::{NcWDistribution, TransparentFactory, WDArena, WDCountV2, WDLevel, WDPresence, TargetedFactory};
use crush::soc::Id;

use crate::diff_solver::post_processing_v5::SolvedSocMeta;
use crate::diff_solver::post_processing_v5::utils::path::Path;
use vob::Vob;
use crate::diff_solver::post_processing_v5::hull_calc_v2::dfe::{SemiTargetedDFE};
use std::sync::mpsc::sync_channel;

pub (super) struct Primed {
    pub(crate) arena: Arc<WDArena<WDCountV2>>,
    // Arena
    // Example path 1
    // Example path 2
    // Alpha beta paths 1
    // Alpha beta paths 2
}

pub (super) fn make_primers(master: Arc<Shard>, master_md: &SolvedSocMeta) {
    let inner_active_area = Range{start: master_md.alpha_lvl_depth, end: master_md.beta_lvl_depth};
    let arena = master.weight_distributions_arena_for_level(
        inner_active_area.start,
        &inner_active_area,
        master_md.step.clone(),
        &TransparentFactory::new(),
    );

    let example_inner = extract_example_inner(master.clone(),&arena, master_md);

    let abc = construct_alpha_beta(master.clone, master_md, example_inner.clone());
    // let abe = extract_alpha_beta(master.clone, master_md, example_inner.clone());

    // todo logg

}

/// Extract an inner path, usable to expand into an example path.
/// The path will be "optimal"
fn extract_example_inner(master: Arc<Shard>,
                         beta_alpha_arena: &WDArena<WDPresence>,
                         master_md: &SolvedSocMeta
) -> Path {
    // First step is to find the n-node which we want our example path to go through. We want the node
    // which has the most number of paths equal to the alpha level nt-lew. As all paths between the
    // alpha node and any node on the n-level will have the same number of active S-boxes (otherwise,
    // they would yield an inconsistent path), we know that we may look for the n-level nt-lew instead.
    // Furthermore, it does not matter if we look at the n-level nt-lew built from Tau or from Beta,
    // of the same reasoning as above.
    // Letting our example (inner) path be of weight alpha level nt lew and pass through the n-node
    // achieved two purposes: Firstly, the example path is a path with the fewest number of active
    // S-boxes (which is what we want from the example path). Secondly, by passing through the "n-node"
    // the example path also becomes ideal to be used as a basis for when we construct an alpha path
    // and a beta path. (By passing through the node with the most alpha level nt-lew paths we
    // maximize the number of "best" paths which are included in the hull calc based on this alpha, beta
    // pair. However, there is an exception to this, which is when the difference between the paths
    // and weights between the "n-node" and another node on the n level rises above a certain "threshold".
    // This threshold depends on the weights of the paths and the number of paths in each node. More
    // on this elsewhere).
    // Our second step will then be to extract such an example path.

    // ==== Finding the "n-node" ====
    // We count from beta to the n-level, only counting inner paths. (All current inner paths have
    // Tau->Beta in common).

    // Note that we double the depth of alpha to get the depth of n. This will hold for ciphers with
    // complete S-box layers. However, this may of may not work on ciphers with incomplete S-box layers.
    // It depends on how the cipher is constructed and what the levels above the alpha level represents.
    // (Only the input to the non-linear S-box layer, or does it also include the input to the identity
    // element/linear part of the S-box layer?). => Invariant checks may need to be introduced.
    let inner_active_area = Range{start: master_md.alpha_lvl_depth*2, end: master_md.beta_lvl_depth};
    let beta_n_dists: WDArena<WDCountV2> = master.weight_distributions_arena_for_level(
        master_md.beta_lvl_depth,
        &inner_active_area,
        master_md.step.clone(),
        &TransparentFactory::new(),
    );

    let (beta_n_lvl_nt_lew, nodes_with_nt_lew) = match beta_n_dists.get(&master_md.beta_lvl_depth)
        .unwrap()
        .nt_lew() {
        Some(ok) => ok,
        None => panic!("Only the trivial lew was present!"),
    };

    if nodes_with_nt_lew.len() == 1 {
    // Only one node on the n-level, all paths go through that node, so extract any path with the
    // correct weight
        todo!("Extract a path from n to beta with level nt lew directly")

    } else {
        // We need to id the "n-node", and extract a path of correct weight passing through this node
        let best_id = identify_n_node(&beta_n_dists.get(&master_md.beta_lvl_depth).unwrap(),
                                      beta_n_lvl_nt_lew, nodes_with_nt_lew);


        // We've found the node we want our path to go through, time to extract it. This is a two
        // step process: First we extract the path down to the n-node. Second we extract the path
        // from the n-node down to beta. We need to be mindful of the "target weight" of the path.

        // Extracting path down to best node on the n-level:
        let mut inner_top = extract_alpha_to_n(&master, master_md, &best_id);

        // Path from alpha to n-node is extracted, time to get from n-node to beta

        let (tx, rx) = sync_channel(1);
        let beta_n_dists = Arc::new(beta_n_dists);

        let dfe = SemiTargetedDFE::new(
            master_md.beta_lvl_depth,
            beta_n_dists.clone(),
            master.clone(),
            master_md.step.clone(),
            tx.clone());


        let inner_bottom = rx.recv().unwrap();

        inner_top.append(&inner_bottom);
        return inner_top
    }
}

/// Extracts a path from the alpha node to the given Id, expected to be on the n-level. (Will panic
/// if not). Returns the path.
fn extract_alpha_to_n(master: &Arc<Shard>, master_md: &SolvedSocMeta, best_id: &Id) -> Path {
    // Note that we double the depth of alpha to get the depth of n. This will hold for ciphers with
    // complete S-box layers. However, this *may of may not* work on ciphers with *incomplete* S-box layers.
    // It depends on how the cipher is constructed and what the levels above the alpha level represents.
    // (Only the input to the non-linear S-box layer, or does it also include the input to the identity
    // element/linear part of the S-box layer?). => Invariant checks may need to be introduced.
    let inner_top_area = Range { start: master_md.alpha_lvl_depth, end: master_md.alpha_lvl_depth * 2 };

    // We only need to know which Centurions connects to the n-level, as all paths are of the same
    // weight anyways, down to the n-level.
    let n_alpha_arena: WDArena<WDPresence> = master.weight_distributions_arena_for_level (
        master_md.alpha_lvl_depth,
        &inner_top_area,
        master_md.step.clone(),
        &TargetedFactory::new(vec![best_id.clone()]),
    );

    let start_nodes = master
        .level(master_md.alpha_lvl_depth)
        .expect("Start level is missing!")
        .get_nodes();
    assert_eq!(start_nodes.len(), 1);
    let mut current_id = start_nodes.keys().next().expect("Start level is empty!").clone();

    // We start at the alpha level, and build down towards the n-level
    let mut current_depth: Depth = master_md.alpha_lvl_depth ;
    let mut inner_top = Vob::with_capacity(master_md.alpha_lvl_depth);

    loop {
        // Base case
        if current_depth == inner_top_area.end {
            return inner_top.into()
        }

        let deps = DepPathFinder::new(
            current_id, current_depth,
            master_md.step.clone(),
            &master,
        );

        let child_depth = current_depth + master_md.step.get();
        let child_level = n_alpha_arena.get(&child_depth)
            .expect(&format!("Did we pass the n-level? Child depth: {}. n-level depth: {}",
                             child_depth, inner_top_area.end));

        for (id, sub_path) in deps.into_iter() {

            // This will panic if we pass the n-level, so we're indirectly covered in so regard.
            if child_level
                .get(&id)
                .expect(&format!("Unable to get node: {}", id))
                .end_connections()
                .contains(&best_id)
            {
                inner_top.extend_from_slice(&sub_path);
                current_depth = child_depth;
                current_id = id;
                break;
            }
        }

    }
}

/// Find the node on the n-level which we want our example path to go through.
/// Return the Id of the n-node.
fn identify_n_node(beta_n_dists: &WDLevel<WDCountV2>, beta_n_lvl_nt_lew: u32, nodes_with_nt_lew: Vec<Id>
) -> Id {
    // This fn can later be modified to take into account that the node with the most paths of weight
    // == lvl_nt_lew is not always the best node. However, the example path is always expected to
    // have weight == lvl nt lew, which means that the n-node still should have at least one path
    // of weight level nt-lew... Anyways, this is not expected to be a big thing, although that is
    // based on anecdotal "evidence" and not data...

    let mut nodes_iter = nodes_with_nt_lew.iter();
    // Setting up compare
    let mut best_id = nodes_iter.next().unwrap().clone();
    let mut best_count = beta_n_dists.get(&best_id).unwrap()
        .paths_for_weight(beta_n_lvl_nt_lew).unwrap();

    // Find best node, based on number of paths with n-level nt lew going through it
    for id in nodes_iter {
        if let Some(count) = beta_n_dists.get(id).unwrap()
            .paths_for_weight(beta_n_lvl_nt_lew)
        {
            if count > best_count {
                best_count = count;
                best_id = id.clone();
            }
        }
    };
    best_id
}
