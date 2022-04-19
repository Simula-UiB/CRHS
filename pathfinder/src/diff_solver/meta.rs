use std::cell::Cell;
use std::fmt::{self, Display, Formatter,};

use crush::soc::bdd::differential::PruneRecord;
use crush::soc::bdd::differential::StyledProgressBar;
use crush::soc::Id;

use super::simple_solver::Depth;

const LIVE_PRINT: bool = false; // OBS, should be left at false as long as live reporting is as it is.
// when a more thought through system for how alternative live printing schemes may be supported is
// implemented, then this flag, or some variant of it, should be free to be set as desired.


pub trait SPFactory {
    type ProgressBar: StyledProgressBar;

    fn new_solve_progress(&self, len: u64) -> Self::ProgressBar;
}


pub struct Librarian<F>
    where
        F: SPFactory,
{
    progress: ProgressHelper<F>,
    history: Vec<Ops>,
}

struct ProgressHelper<F>
    where
        F: SPFactory,
{
    factory: F,
    absorb: Option<ProgressHelperAbsorb<F::ProgressBar>>,
}


#[derive(Debug)]
struct ProgressHelperAbsorb<S: StyledProgressBar> {
    /// Total nr of Absorptions to do
    total: usize,
    /// How many have been executed
    executed: Cell<usize>,
    /// Progress bar to update
    pb: S,
}


impl<F> Librarian<F>
    where
        F: SPFactory,
{
    pub fn new(complexity: usize, factory: F ) -> Self {
        let history = vec![Ops::New(complexity)];
        Self {
            progress: ProgressHelper{factory, absorb: None},
            history,
        }
    }

    pub fn record(&mut self, op: Ops) {
        match &op {
            Ops::Join(rec) => {
                let pb = self.progress.factory.new_solve_progress(rec.unresolved_deps  as u64);
                self.progress.absorb = Some(
                    ProgressHelperAbsorb{
                        total: rec.unresolved_deps,
                        executed: Cell::new(0),
                        pb,
                    });
                self.progress.absorb.as_ref().unwrap().pb.set_message("Absorbing");
                self.progress.absorb.as_ref().unwrap().pb.inc(1, );
            },

            Ops::Absorb(_) => {
                let a = self.progress.absorb.as_ref().unwrap();
                a.executed.set(a.executed.get() + 1);
                if a.executed.get() == a.total {
                    a.pb.finish_and_clear();
                    self.progress.absorb = None;
                } else {
                    a.pb.inc(1);
                }
            },
            _ => {},
        }

        if LIVE_PRINT {
            match &op {
                Ops::Prune(_) => {},
                op => {
                    println!("{}", &op);
                },
            }
        }

        self.history.push(op);
    }

    #[allow(dead_code)]
    pub fn print (&self) {
        for ops in self.history.iter() {
            let ops = format!("{}", ops);
            println!("{: <25}", ops,);
        }
    }
    
    pub fn record_prune_helper() -> PruneRecorder {
        PruneRecorder {
            rec: None,
        }
    }
}


#[derive(Debug)]
pub enum Ops {

    /// Recording of the complexity. Useful to start get a record at the start of any operation.
    New(usize),
    /// Recording of a swap operation. First value in tuple is the Id of `top` shard, the one whose
    /// sink is replaced with the root of the `bottom` shard. Second value is the Id of `bottom`.
    Join(JoinRec),

    /// Recording of a swap operation. First value in tuple is the `from` depth, the second is the
    /// `to` depth.
    Absorb(AbsorbRec),

    Prune(PruneRecord),

    Text(String),
}


#[derive(Debug)]
pub enum CoreOps {
    Swap(Depth, Depth),

    Add(Depth, Depth), // above, below

    Extract(Depth),
}


#[derive(Debug)]
pub struct JoinRec {
    top: Id,
    bottom: Id,
    complexity: usize,
    /// Number of unresolved linear dependencies present after this join, in the newly formed shard.
    unresolved_deps: usize,
}

impl JoinRec {
    pub fn new(top: Id, bottom: Id, complexity: usize, unresolved_deps: usize) -> Self {
        Self {
            top,
            bottom,
            complexity,
            unresolved_deps,
        }
    }
}

#[derive(Debug)]
pub struct AbsorbRec {
    pre_absorb: Option<PreAbsorbRec>,
    ops: Vec<CoreOps>,
    complexity: Option<usize>,
}

impl AbsorbRec {
    pub fn new() -> Self {
        Self {
            pre_absorb: None,
            ops: Default::default(),
            complexity: None,
        }
    }

    pub fn record(&mut self, ops: CoreOps) {
        self.ops.push(ops);
    }

    pub fn set_complexity(&mut self, complexity: usize) {
        self.complexity = Some(complexity)
    }

    pub fn set_pre_abs_rec(&mut self, pre_abs: PreAbsorbRec) {
        self.pre_absorb = Some(pre_abs);
    }
}


#[derive(Debug)]
pub struct PreAbsorbRec {
    base: Depth,
    rest: Vec<Depth>,
    swaps: Option<Vec<CoreOps>>,
}

impl PreAbsorbRec {
    pub fn new(base: Depth, rest: Vec<Depth>) -> Self {
        Self {
            base,
            rest,
            swaps: None,
        }
    }

    pub fn record_swaps(&mut self, swaps: Vec<CoreOps>) {
        self.swaps = Some(swaps);
    }
}



// ==============================================================================================
pub struct PruneRecorder {
    rec: Option<PruneRecord>,

}

impl PruneRecorder {
    pub fn get_rec(self) -> Option<PruneRecord> {
        self.rec
    }
}

impl crush::soc::bdd::differential::PruneLogger for PruneRecorder{
    fn record(&mut self, rec: PruneRecord) {
        self.rec = Some(rec);
    }
}


// ===============================================================================================
// ====================================== DISPLAY IMPLS ==========================================
// ===============================================================================================

impl Display for Ops {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use console::style;

        use Ops::*;
        let width = 20;
        match self {
            New(c) => {write!(f, "Initial complexity {}", c)},
            Join(jr) => {
                let s =
                    format!("Join:\n{: >w$} {}\n{: >w$} {}\n{: >w$} {}\n{: >w$} {}",
                            "Top:", jr.top,
                            "Bottom:", style(jr.bottom).green(),
                            "Complexity:", jr.complexity,
                            "Unresolved deps:", jr.unresolved_deps,
                            w=width);
                write!(f, "{}", s)
            },
            Absorb(rec) => {
                write!(f, "{}", rec)
            }
            Text(s) => {write!(f, "{}", s) }
            Prune(rec ) => {
                write!(f, "{}", rec)
            }
        }
    }
}

impl Display for PreAbsorbRec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let width = 6;
        let width2 = 20;
        let s;
        let mut deps = format!("{}", self.rest.first().unwrap());
        for dep in self.rest.iter().skip(1) {
            deps = format!("{}, {}", deps, dep);
        }
        if self.swaps.is_none() {

            s = format!("{: >w$}\n{: >w2$} {}\n{: >w2$} {}",
                            "Pre Absorb:",
                            "Base:", self.base,
                            "Remaining:", deps,
                            w = width,
                            w2 = width2,
            );

        } else {
            let mut swap = format!("{}", self.swaps.as_ref().unwrap().first().unwrap()) ;
            for sw in self.swaps.as_ref().unwrap().iter().skip(1) {
                swap = format!("{}\n{}", swap, sw);
            }

            s = format!("{: >w$}\n{: >w2$}{}\n{: >w2$}{}\n{: >w2$}\n{}",
                        "Pre Absorb:",
                        "Base:", self.base,
                        "Remaining:", deps,
                        "Swaps performed:", swap,
                        w = width,
                        w2 = width2,
            );
        }
        write!(f, "{}", s)
    }
}

impl Display for CoreOps {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CoreOps::Add(top, bottom) => {
                write!(f, "{}\n{: >w$}{} onto {}",
                       "Add:",
                       "",
                       top,
                       bottom,
                       w = 20,
                )
            },
            CoreOps::Swap(from, to) => {
                write!(f, "{}\n{: >w$} {}\n{: >w$} {}",
                       "Swap:",
                       "From:", from,
                       "To:", to,
                       w = 20,
                )
            },
            CoreOps::Extract(depth) => {
                write!(f, "{}\n{: >w$}{}", "Extract:",
                       "Depth:", depth,
                       w = 20)
            }
        }

    }
}

impl Display for AbsorbRec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let width = 20;
        if self.pre_absorb.is_some() {
            // write!(f, "{}\n", self.pre_absorb.as_ref().unwrap())?;
        }

        write!(f, "{}\n",
               "Absorb:",
        )?;
        if self.complexity.is_some() {
            write!(f, "{: >w$} {}\n", "Complexity:", self.complexity.as_ref().unwrap(), w=width)?;
        }
        write!(f, "{: >w$}", "Core ops: {Omitted}", w=width)?;
        // for ops in self.ops.iter() {
        //     write!(f, "\n{: >10}{}", "", ops)?;
        // }
        write!(f, "\n",)

    }
}