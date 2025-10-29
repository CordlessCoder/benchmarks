use std::{
    fmt::Display,
    hash::Hash,
    sync::{
        Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
};

use egui::ComboBox;
/// A lock-free, atomic progress bar
/// Also used for synchronizing multiple workers to the same stages.
#[derive(Debug)]
pub struct ProgressTracker<State: Clone> {
    stop_requested: AtomicBool,
    total: AtomicU64,
    counter: AtomicU64,
    threads: AtomicUsize,
    threads_waiting_to_transition: AtomicUsize,
    state: Mutex<State>,
    state_transition: Condvar,
}

/// A single snapshot of the state of a benchmark progress bar - created via [`BenchmarkProgressTracker::load`].
pub struct BenchmarkProgressSnapshop<State> {
    pub total: u64,
    pub counter: u64,
    pub state: State,
    pub threads_waiting_to_transition: usize,
    pub was_cancelled: bool,
}

impl<State: PartialEq + Display + Clone> ProgressTracker<State> {
    #[must_use]
    pub fn new(total: u64, threads: usize, state: State) -> Self {
        ProgressTracker {
            total: AtomicU64::new(total),
            counter: AtomicU64::new(0),
            threads: AtomicUsize::new(threads),
            threads_waiting_to_transition: AtomicUsize::new(0),
            state_transition: Condvar::new(),
            state: Mutex::new(state),
            stop_requested: AtomicBool::new(false),
        }
    }
    pub fn add_thread(&self) {
        self.threads.fetch_add(1, Ordering::Relaxed);
    }
    pub fn stop_requested(&self) -> bool {
        self.stop_requested.load(Ordering::Relaxed)
    }
    pub fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::Relaxed);
        self.state_transition.notify_all();
    }
    pub fn transition_state(&self, new_state: State, new_total: u64) {
        // The thread that will actually perform the state transition will be the thread with
        // the transition_number that indicates it brought the number of threads waiting to
        // self.threads
        let transition_number = self
            .threads_waiting_to_transition
            .fetch_add(1, Ordering::AcqRel);
        let mut state = self.state.lock().unwrap();
        if transition_number + 1 == self.threads.load(Ordering::Relaxed) {
            // We have successfully transitioned state.
            *state = new_state;
            // Reset counters
            self.threads_waiting_to_transition
                .store(0, Ordering::Release);
            self.set_total(new_total);
            self.set_counter(0);
            self.state_transition.notify_all();
        } else {
            // Wait for the transition to happen, or stop to be requested
            let _state = self
                .state_transition
                .wait_while(state, |state| *state != new_state && !self.stop_requested())
                .unwrap();
        };
    }
    pub fn add(&self, amount: u64) {
        self.counter.fetch_add(amount, Ordering::Relaxed);
    }
    pub fn set_total(&self, new_total: u64) {
        self.total.store(new_total, Ordering::Release);
    }
    pub fn set_counter(&self, new_counter: u64) {
        self.counter.store(new_counter, Ordering::Release);
    }
    pub fn reset(&self, total: u64, threads: usize, state: State) {
        self.total.store(total, Ordering::Relaxed);
        self.threads_waiting_to_transition
            .store(0, Ordering::Relaxed);
        self.threads.store(threads, Ordering::Relaxed);
        self.set_counter(0);
        *self.state.lock().unwrap() = state;
    }
    pub fn load_state(&self) -> State {
        self.state.lock().unwrap().clone()
    }
    pub fn load(&self) -> BenchmarkProgressSnapshop<State> {
        let total = self.total.load(Ordering::Acquire);
        let counter = self.counter.load(Ordering::Relaxed);
        let state = self.load_state();
        let threads_waiting_to_transition =
            self.threads_waiting_to_transition.load(Ordering::Relaxed);
        let was_cancelled = self.stop_requested();
        BenchmarkProgressSnapshop {
            total,
            counter,
            state,
            threads_waiting_to_transition,
            was_cancelled,
        }
    }
}

impl<State: Clone + Display + PartialEq> BenchmarkProgressSnapshop<State> {
    pub fn was_cancelled(&self) -> bool {
        self.was_cancelled
    }
    pub fn current_state(&self) -> State {
        self.state.clone()
    }
    pub fn as_f32(&self) -> f32 {
        self.counter as f32 / self.total as f32
    }
}

pub trait SelectableEnum: Sized + Clone + 'static + PartialEq {
    fn all_values() -> &'static [Self];
    fn is_enabled(&self) -> bool {
        true
    }
    fn as_str(&self) -> &'static str;
}

pub fn selectable_enum<E: SelectableEnum>(
    ui: &mut egui::Ui,
    id: impl Hash,
    selected: &mut E,
    set_options: impl FnOnce(ComboBox) -> ComboBox,
) {
    set_options(egui::ComboBox::from_id_salt(id))
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for value in E::all_values() {
                if !value.is_enabled() {
                    continue;
                }
                ui.selectable_value(selected, value.clone(), value.as_str());
            }
        });
}
