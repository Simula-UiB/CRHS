use crate::soc::bdd::differential::wd::NWDistribution;
use crate::soc::Id;

pub trait DistFactory<W> {

    /// Return a new instance of W, where all weights are zero.
    fn new_zeroed(&self) -> W;

    /// Return a new instance of W, where *only* the *trivial* path is present.
    /// Argument 'id' is expected to be the id of the node at the end of the path.
    /// This enables a path to have both a start node_id and end node_id.
    fn new_trivial(&self, id: &Id) -> W;
}


/// A factory which return always returns W::new_trivial(), independently of the Id given.
/// Use this factory when making an Arena/Level covering all nodes.
pub struct TransparentFactory {}

impl TransparentFactory {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }
}

impl<W: NWDistribution> DistFactory<W> for TransparentFactory {

    #[inline]
    fn new_zeroed(&self) -> W {
        W::new_zeroed()
    }

    #[inline]
    fn new_trivial(&self, id: &Id) -> W {
        W::new_trivial(id)
    }
}




/// This factory allows for targeting specific nodes. Only nodes which Id has been given to the
/// Factory will return W::new_trivial() when Factory::new_trivial() is called.
/// This allows for building distributions *only* for the targeted nodes: all other nodes will not
/// be present in the distributions. (IMPORTANT: The factory can only go so far. Ensure that the
/// underlying distribution upholds its part of this deal).
///
/// This is thus a memory saving technique, which is recommended to use when we're anyways interested
/// only in a subset of the nodes.
pub struct TargetedFactory {
    targets: Vec<Id>,
}

impl TargetedFactory {
    #[inline]
     pub fn new(targets: Vec<Id>) -> TargetedFactory {
        Self {
            targets
        }
    }
}

impl<W: NWDistribution>  DistFactory<W> for TargetedFactory {
    #[inline]
    fn new_zeroed(&self) -> W {
        W::new_zeroed()
    }

    /// Calls the underlying `W::new_trivial(id)` **iff** the given Id is a target Id.
    /// Otherwise calls `W::new_zeroed()`.
    #[inline]
    fn new_trivial(&self, id: &Id) -> W {
        if self.targets.contains(id) {
            W::new_trivial(id)
        } else {
            W::new_zeroed()
        }
    }
}
