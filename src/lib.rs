use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// A lock-free, atomic state of a progress bar
pub struct BenchmarkProgressTracker {
    total: AtomicU64,
    counter: AtomicU64,
    threads: AtomicUsize,
    threads_done: AtomicUsize,
}

/// A single snapshot of the state of a benchmark progress bar - created via [`BenchmarkProgressTracker::load`].
pub struct BenchmarkProgressSnapshop {
    pub total: u64,
    pub counter: u64,
}

impl BenchmarkProgressTracker {
    #[must_use]
    pub fn new(total: u64, threads: usize) -> Self {
        BenchmarkProgressTracker {
            total: AtomicU64::new(total),
            counter: AtomicU64::new(0),
            threads: AtomicUsize::new(threads),
            threads_done: AtomicUsize::new(0),
        }
    }
    pub fn flag_thread_done(&self) {
        let other_threads_done = self.threads_done.fetch_add(1, Ordering::AcqRel);
        if other_threads_done >= self.threads.load(Ordering::Relaxed) {
            // Do not allow more than self.threads threads to be marked as done.
            self.threads_done.fetch_sub(1, Ordering::Relaxed);
        }
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
    pub fn reset(&self, total: u64, threads: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.threads_done.store(0, Ordering::Relaxed);
        self.threads.store(threads, Ordering::Relaxed);
        self.set_counter(0);
    }
    pub fn all_threads_done(&self) -> bool {
        self.threads_done.load(Ordering::Acquire) == self.threads.load(Ordering::Acquire)
    }
    pub fn load(&self) -> BenchmarkProgressSnapshop {
        let total = self.total.load(Ordering::Acquire);
        let counter = self.counter.load(Ordering::Relaxed);
        BenchmarkProgressSnapshop { total, counter }
    }
}

impl BenchmarkProgressSnapshop {
    pub fn is_full(&self) -> bool {
        self.counter >= self.total
    }
    pub fn as_f32(&self) -> f32 {
        self.counter as f32 / self.total as f32
    }
}

pub struct MemoryThroughputBenchConfig {
    pub total_size: usize,
    pub block_size: usize,
    pub hugepages: bool,
    pub threads: usize,
}

pub struct MemoryThroughputBench<'c, 'scope> {
    config: &'c MemoryThroughputBenchConfig,
    threads: Vec<std::thread::ScopedJoinHandle<'scope, ()>>,
}

impl MemoryThroughputBenchConfig {
    pub fn start<'c, 'a>(
        &'c self,
        thread_scope: &'a std::thread::Scope<'a, 'a>,
        progress: &'a BenchmarkProgressTracker,
    ) -> MemoryThroughputBench<'c, 'a> {
        let mut threads = Vec::new();
        threads.push(thread_scope.spawn(|| {
            progress.reset(1, 1);
        }));
        MemoryThroughputBench {
            config: self,
            threads,
        }
    }
}
