//! ProgressBar and other functionality to report the progress of the program

use std::sync::Arc;

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

use crush::soc::bdd::differential::{PPFactory, StyledProgressBar};
// use pathfinder::diff_solver::post_processing_v3::PostPFactory;
use pathfinder::diff_solver::SPFactory;

#[derive(Debug, Clone)]
pub struct Progress {
    mp: Arc<MultiProgress>,
}

#[derive(Debug, Clone)]
pub struct MyStyledSpinner {
    spinner: ProgressBar,
}

#[derive(Debug, Clone)]
pub struct MyStyledProgressBar {
    pb: ProgressBar,
}

// ===============================================================================================

impl Progress {

    pub fn new() -> Self {
        Self {
            mp: Arc::new(
                MultiProgress::with_draw_target(ProgressDrawTarget::stdout())
                // quickfix for debugging
                // MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
            ),
        }
    }

    pub fn new_main_spinner(&self) -> MyStyledSpinner {
        let spinner = self.mp.add(ProgressBar::new_spinner());
        spinner.set_style(Styles::MainSpinner.style());

        MyStyledSpinner::new(spinner)
    }

    pub fn new_spinner(&self) -> MyStyledSpinner {
        let spinner = self.mp.add(ProgressBar::new_spinner());
        spinner.set_style(Styles::Spinner.style());

        MyStyledSpinner::new(spinner)
    }

    pub fn new_progress_bar(&self, len: u64) -> MyStyledProgressBar {
        let pb = self.mp.add(ProgressBar::new(len));
        pb.set_style(Styles::Pb.style());

        MyStyledProgressBar::new(pb)
    }

    pub fn join(&self) {
        self.mp.join().unwrap();
    }

    pub fn join_and_clear(&self) {
        self.mp.join_and_clear().unwrap();
    }
}


impl SPFactory for Progress {
    type ProgressBar = MyStyledProgressBar;

    fn new_solve_progress(&self, len: u64) -> Self::ProgressBar {
        self.new_progress_bar(len)
    }

}

impl PPFactory for Progress {
    type ProgressBar = MyStyledProgressBar;

    fn new_progress_bar(&self, len: u64) -> Self::ProgressBar {
        self.new_progress_bar(len)
    }
}

// impl PostPFactory for Progress {
//     type ProgressBar = MyStyledProgressBar;
//
//     fn new_processing_progress(&self, len: u64) -> Self::ProgressBar {
//         self.new_progress_bar(len)
//     }
// }
// ===============================================================================================


//TODO use 'ambassador' to implement relevant fn's from ProgressBar?
impl MyStyledSpinner {
    fn new(spinner: ProgressBar) -> MyStyledSpinner {
        let spinner = Self {
            spinner,
        };

        spinner
    }

    pub fn enable_steady_tick(&self, ms: u64) {
        self.spinner.enable_steady_tick(ms);
    }

    pub fn is_finished(&self) -> bool {
        self.spinner.is_finished()
    }
}


impl StyledProgressBar for MyStyledSpinner {
    fn inc(&self, delta: u64) {self.spinner.inc(delta);}

    fn set_message(&self, msg: &str) {
        self.spinner.set_message(msg);
    }

    fn finish_with_message(&self, msg: &str) {
        self.spinner.finish_with_message(msg);
    }

    fn finish_and_clear(&self) {
        self.spinner.finish_and_clear();
    }

    fn println(&self, msg: &str) {
        self.spinner.println(msg);
    }

}


// ===============================================================================================
//TODO use 'ambassador' to implement relevant fn's from ProgressBar?
impl MyStyledProgressBar {
    fn new(pb: ProgressBar) -> MyStyledProgressBar {
        Self {
            pb,
        }
    }

    pub fn enable_steady_tick(&self, ms: u64) {
        self.pb.enable_steady_tick(ms);
    }

    pub fn is_finished(&self) -> bool {
        self.pb.is_finished()
    }

}


impl StyledProgressBar for MyStyledProgressBar {
    /// Advances the position of a progress bar by delta.
    fn inc(&self, delta: u64) {self.pb.inc(delta);}

    /// Sets the current message of the progress bar.
    fn set_message(&self, msg: &str) {
        self.pb.set_message(msg);
    }

    /// Finishes the progress bar and sets a message.
    fn finish_with_message(&self, msg: &str) {
        self.pb.finish_with_message(msg);
    }

    /// Finishes the progress bar and completely clears it.
    fn finish_and_clear(&self) {
        self.pb.finish_and_clear();
    }

    /// Will print a log line above all progress bars.
    /// Note that if the progress bar is hidden (which by default happens if the progress bar is
    /// redirected into a file) println will not do anything either.
    fn println(&self, msg: &str) {
        self.pb.println(msg);
    }
}

// ===============================================================================================

// ===============================================================================================

enum Styles {
    Pb,
    Spinner,
    MainSpinner,
}

impl Styles {
    fn style(&self) -> ProgressStyle {
        use Styles::{Pb, Spinner, MainSpinner};

        match self {
            Pb => {
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:100.cyan/yellow} {pos:>7}/{len:7} {msg}")
                    .progress_chars("#>-")
            },
            Spinner => {
                ProgressStyle::default_spinner()
                    .template("{spinner} {msg}")
            },
            MainSpinner => {
                ProgressStyle::default_spinner()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                    .template("[{elapsed_precise}] {msg} {spinner.green}")

            }
        }
    }
}



