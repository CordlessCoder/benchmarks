#![expect(unused)]
use eframe::egui;
use std::{
    any::Any,
    fmt::Display,
    time::{Duration, Instant},
};

pub trait BackgroundComputeProvider {
    type Output;
    type Error: Display;

    fn compute(&mut self) -> Option<&mut Self::Output>;
    fn is_being_computed(&self) -> bool;
    fn get_error(&self) -> Option<&Self::Error>;
    fn update_period(&self) -> Option<Duration> {
        None
    }

    fn display(
        &mut self,
        ui: &mut egui::Ui,
        on_success: impl FnOnce(&mut egui::Ui, &mut Self::Output),
    ) {
        if let Some(period) = self.update_period() {
            ui.ctx().request_repaint_after(period);
        }
        if let Some(value) = self.compute() {
            on_success(ui, value);
        } else if self.is_being_computed() {
            ui.spinner();
        } else if let Some(err) = self.get_error() {
            ui.label(format!("Error: {err}"));
        }
    }
}

pub struct RepeatedCompute<T, E, C = fn() -> Result<T, E>> {
    update_period: Duration,
    compute_with: C,
    last_compute_start: Instant,
    being_computed: Option<BackgroundCompute<T, E, C>>,
    previous_compute: BackgroundCompute<T, E, C>,
}

impl<T, E, C> RepeatedCompute<T, E, C>
where
    T: Send + 'static,
    E: Send + 'static,
    C: Send + 'static + FnOnce() -> Result<T, E> + Clone,
{
    pub fn new(compute_with: C, update_period: Duration) -> Self {
        RepeatedCompute {
            previous_compute: BackgroundCompute::new(compute_with.clone()),
            being_computed: None,
            update_period,
            compute_with,
            last_compute_start: Instant::now(),
        }
    }
}

impl<T, E, C> BackgroundComputeProvider for RepeatedCompute<T, E, C>
where
    T: Send + 'static,
    E: Send + 'static + Display,
    C: Send + 'static + FnOnce() -> Result<T, E> + Clone,
{
    type Output = T;
    type Error = E;

    fn compute(&mut self) -> Option<&mut T> {
        // Check if we have a new compute result
        if let Some(mut being_computed) = self.being_computed.take() {
            being_computed.compute();
            if being_computed.is_done() {
                self.previous_compute = being_computed;
            } else {
                self.being_computed = Some(being_computed);
            }
        }
        // Check if we need to start a new compute
        if self.being_computed.is_none() && self.last_compute_start.elapsed() >= self.update_period
        {
            let mut new_compute = BackgroundCompute::new(self.compute_with.clone());
            // Start worker thread
            new_compute.compute();
            self.being_computed = Some(new_compute);
        }
        self.previous_compute.compute()
    }
    fn update_period(&self) -> Option<Duration> {
        Some(self.update_period)
    }
    fn get_error(&self) -> Option<&Self::Error> {
        self.previous_compute.get_error()
    }
    fn is_being_computed(&self) -> bool {
        self.previous_compute.is_being_computed()
    }
}

pub enum BackgroundCompute<T, E, C = fn() -> Result<T, E>> {
    Computable(C),
    Computing(std::thread::JoinHandle<Result<T, E>>),
    Computed(T),
    Failed(E),
    Poisoned(Box<dyn Any + Send + 'static>),
}

impl<T, E, C> BackgroundComputeProvider for BackgroundCompute<T, E, C>
where
    T: Send + 'static,
    E: Send + 'static + Display,
    C: Send + 'static + FnOnce() -> Result<T, E>,
{
    type Output = T;
    type Error = E;

    fn get_error(&self) -> Option<&E> {
        match self {
            Self::Failed(err) => Some(err),
            _ => None,
        }
    }
    fn is_being_computed(&self) -> bool {
        matches!(self, Self::Computing(_))
    }
    fn compute(&mut self) -> Option<&mut T> {
        use BackgroundCompute::*;
        loop {
            match self {
                Computable(_) => {
                    let BackgroundCompute::Computable(compute_with) =
                        core::mem::replace(self, BackgroundCompute::Poisoned(Box::new(())))
                    else {
                        unreachable!()
                    };
                    let worker_thread = std::thread::spawn(compute_with);
                    *self = BackgroundCompute::Computing(worker_thread);
                    break None;
                }
                Computing(worker_thread) if worker_thread.is_finished() => {
                    let BackgroundCompute::Computing(handle) =
                        core::mem::replace(self, BackgroundCompute::Poisoned(Box::new(())))
                    else {
                        unreachable!()
                    };
                    *self = match handle.join() {
                        Err(poison) => BackgroundCompute::Poisoned(poison),
                        Ok(Ok(value)) => BackgroundCompute::Computed(value),
                        Ok(Err(err)) => BackgroundCompute::Failed(err),
                    };
                }
                Computed(val) => break Some(val),
                _ => break None,
            }
        }
    }
}

impl<T, E, C> BackgroundCompute<T, E, C>
where
    T: Send + 'static,
    E: Send + 'static,
    C: Send + 'static + FnOnce() -> Result<T, E>,
{
    pub fn new(compute_with: C) -> Self {
        Self::Computable(compute_with)
    }
    pub fn into_computed(self) -> Result<T, Self> {
        match self {
            Self::Computed(val) => Ok(val),
            other => Err(other),
        }
    }
    pub fn as_computed(&self) -> Option<&T> {
        match self {
            Self::Computed(val) => Some(val),
            _ => None,
        }
    }
    pub fn is_done(&self) -> bool {
        matches!(
            self,
            Self::Computed(_) | Self::Poisoned(_) | Self::Failed(_)
        )
    }
    pub fn failed(&self) -> bool {
        matches!(self, Self::Poisoned(_) | Self::Failed(_))
    }
    pub fn as_poison(&self) -> Option<&Box<dyn Any + Send + 'static>> {
        match self {
            Self::Poisoned(poison) => Some(poison),
            _ => None,
        }
    }
}
