#![allow(unused)]
use nix::unistd::SysconfVar;
use rand::{Rng, RngCore, rng};
use std::{
    fmt::Display,
    hint::black_box,
    mem::MaybeUninit,
    ptr::NonNull,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use crate::ProgressTracker;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryOperation {
    Read,
    Write,
    Copy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryInitializationType {
    Zeros,
    Random,
    Ones,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Allocating,
    Initializing,
    Executing(usize, usize),
    Done,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use State::*;
        let text = match self {
            Allocating => "Allocating Buffers",
            Initializing => "Initializing Buffers",
            Executing(pass, total) => return write!(f, "Pass {pass} of {total}"),
            Done => "Done",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub size_per_thread: usize,
    pub passes: usize,
    pub threads: usize,
    pub operation: MemoryOperation,
    pub init_type: MemoryInitializationType,
}

#[derive(Debug)]
pub struct MemoryThroughputBench {
    config: Config,
    threads: Vec<std::thread::JoinHandle<Option<TestResult>>>,
    progress: Arc<ProgressTracker<State>>,
}

impl Config {
    pub fn start(self) -> MemoryThroughputBench {
        let progress = Arc::new(ProgressTracker::new(
            (self.thread_memory_layout().size() * self.threads) as u64,
            self.threads,
            State::Allocating,
        ));
        let mut bench = MemoryThroughputBench {
            config: self,
            threads: Vec::new(),
            progress,
        };
        for _ in 0..bench.config.threads {
            bench.spawn_worker();
        }
        bench
    }
    fn thread_memory_layout(&self) -> core::alloc::Layout {
        core::alloc::Layout::array::<u8>(self.size_per_thread)
            .unwrap()
            .align_to(*PAGE_SIZE)
            .unwrap()
            .pad_to_align()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TestResult {
    pub memory_processed: usize,
    pub runtime: Duration,
}
impl TestResult {
    pub fn throughput(&self) -> f64 {
        self.memory_processed as f64 / self.runtime.as_secs_f64()
    }
}
pub static PAGE_SIZE: LazyLock<usize> = LazyLock::new(|| {
    nix::unistd::sysconf(SysconfVar::PAGE_SIZE)
        .ok()
        .flatten()
        .unwrap_or(4096) as usize
});

impl MemoryThroughputBench {
    pub fn progress(&self) -> Arc<ProgressTracker<State>> {
        Arc::clone(&self.progress)
    }
    pub fn is_done(&self) -> bool {
        self.progress.stop_requested() || *self.progress.state.lock().unwrap() == State::Done
    }
    pub fn wait_for_results(self) -> Vec<TestResult> {
        let Self {
            config,
            threads,
            progress,
        } = self;
        let mut results = Vec::new();
        for thread in threads {
            let Some(sample) = thread.join().unwrap() else {
                continue;
            };
            results.push(sample);
        }
        results
    }
    fn spawn_worker(&mut self) {
        let config = self.config.clone();
        let progress = Arc::clone(&self.progress);
        self.threads
            .push(std::thread::spawn(move || Self::run(config, progress)));
    }
    fn run(config: Config, progress: Arc<ProgressTracker<State>>) -> Option<TestResult> {
        let chunk_size = *PAGE_SIZE * 4;
        let mem = config.thread_memory_layout();
        let memory: &mut [u8] = unsafe {
            let ptr = std::alloc::alloc_zeroed(mem);
            let Some(ptr) = NonNull::new(ptr) else {
                std::alloc::handle_alloc_error(mem);
            };
            core::slice::from_raw_parts_mut(ptr.as_ptr(), mem.size())
        };
        progress.add(1);
        progress.transition_state(State::Initializing, (mem.size() * config.threads) as u64);
        if progress.stop_requested() {
            return None;
        }
        unsafe {
            match config.init_type {
                // The data was zeroed on initialization
                MemoryInitializationType::Zeros => (),
                MemoryInitializationType::Ones => {
                    for chunk in memory.chunks_exact_mut(chunk_size) {
                        chunk.iter_mut().for_each(|b| *b = u8::MAX);
                        progress.add(chunk.len() as u64);
                        if progress.stop_requested() {
                            return None;
                        }
                    }
                }
                MemoryInitializationType::Random => {
                    let mut rng = rng();
                    for chunk in memory.chunks_exact_mut(chunk_size) {
                        rng.fill_bytes(chunk);
                        progress.add(chunk.len() as u64);
                        if progress.stop_requested() {
                            return None;
                        }
                    }
                }
            }
        };
        let mut total_runtime = Duration::ZERO;
        for pass in 0..config.passes {
            progress.transition_state(
                State::Executing(pass + 1, config.passes),
                (mem.size() * config.threads) as u64,
            );
            if progress.stop_requested() {
                return None;
            }

            let start = Instant::now();
            match config.operation {
                MemoryOperation::Read => {
                    for chunk in memory.chunks_exact_mut(chunk_size) {
                        black_box(chunk.as_mut_ptr());
                        for byte in &mut *chunk {
                            _ = black_box(*byte);
                        }
                        black_box(chunk.as_mut_ptr());
                        progress.add(chunk.len() as u64);
                        if progress.stop_requested() {
                            return None;
                        }
                    }
                }
                MemoryOperation::Write => {
                    for chunk in memory.chunks_exact_mut(chunk_size) {
                        black_box(chunk.as_mut_ptr());
                        unsafe { chunk.as_mut_ptr().write_bytes(0b10101010, chunk.len()) };
                        black_box(chunk.as_mut_ptr());
                        progress.add(chunk.len() as u64);
                        if progress.stop_requested() {
                            return None;
                        }
                    }
                }
                MemoryOperation::Copy => {
                    let (from, to) = memory.split_at_mut(memory.len() / 2);
                    for (from, to) in from
                        .chunks_exact_mut(chunk_size)
                        .zip(to.chunks_exact_mut(chunk_size))
                    {
                        black_box((from.as_mut_ptr(), to.as_mut_ptr()));
                        unsafe {
                            to.as_mut_ptr()
                                .copy_from_nonoverlapping(from.as_mut_ptr(), to.len());
                        }
                        black_box((from.as_mut_ptr(), to.as_mut_ptr()));
                        progress.add((from.len() + to.len()) as u64);
                        if progress.stop_requested() {
                            return None;
                        }
                    }
                }
            }
            let pass_runtime = start.elapsed();
            total_runtime += pass_runtime;
        }
        progress.transition_state(State::Done, config.threads as u64);
        if progress.stop_requested() {
            return None;
        }
        progress.add(1);
        Some(TestResult {
            memory_processed: memory.len() * config.passes,
            runtime: total_runtime,
        })
    }
}
