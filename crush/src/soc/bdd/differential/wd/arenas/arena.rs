use std::collections::BTreeMap;
use std::fmt;

use crate::soc::bdd::differential::Depth;
use crate::soc::bdd::differential::wd::{NWDistribution, WDLevel};
use crate::soc::Id;

#[derive(Clone)]
pub struct WDArena<W> {
    /// Arena to introduce indirection, mapping is :> Depth : (Id : weight)
    arena: BTreeMap<Depth, WDLevel<W>>,
}

impl<W> WDArena<W> {

    #[inline]
    pub fn new() -> Self {
        Self {
            arena: Default::default(),
        }
    }

    /// Insert a level of nodes and weights into the arena.
    /// Anything already present at 'depth' will be overwritten.
    ///
    /// Does not check that the depth given and any depth set in level matches.
    #[inline]
    pub fn insert_level(&mut self, depth: usize, level: WDLevel<W>) {
        // Note, missing param checks
        self.arena.insert(depth, level);
    }

    #[inline]
    pub fn get(&self, depth: &Depth) -> Option<&WDLevel<W>> {
        self.arena.get(depth)
    }

    #[inline]
    pub fn top(&self) -> Option<Depth> {
        Some(self.arena.iter().next()?.0.clone())
    }

    /// Returns the depth of the deepest level present in self.
    /// Where calls to 'end' of a range usually is exclusive, this 'end' is *inclusive*.
    /// Note that self.start == self.deepest is allowed.
    /// Returns None if the arena is empty.
    #[inline]
    pub fn deepest(&self) -> Option<Depth> {
        Some(self.arena.iter().last()?.0.clone())
    }

    #[inline]
    pub fn contains(&self, node_id: &Id, at_depth: Depth) -> bool {
        if let Some(lvl) = self.get(&at_depth) {
            return lvl.get(node_id).is_some()
        }
        false
    }

    #[inline]
    pub fn complexity(&self) -> usize {
        self.arena.values()
            .map(|lvl| lvl.width())
            .sum()
    }

}

impl<W: NWDistribution> fmt::Debug for WDArena<W> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.arena)?;
        Ok(())
    }
}