#![expect(unused)]
use eframe::egui;
use std::{
    any::Any,
    fmt::Display,
    panic::AssertUnwindSafe,
    sync::mpsc::{self, SendError},
    time::{Duration, Instant},
};
use tracing::error;

pub trait BackgroundComputeProvider {
    type Output;
    type Error: Display;

    fn was_attempted(&self) -> bool;
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

pub struct RepeatedCompute<T> {
    update_period: Duration,
    worker_thread: std::thread::JoinHandle<()>,
    compute_reciever: mpsc::Receiver<T>,
    last_value: Option<T>,
}

impl<T> RepeatedCompute<T>
where
    T: Send + 'static,
{
    pub fn new(
        mut compute_with: impl FnMut() -> T + Send + 'static,
        update_period: Duration,
    ) -> Self {
        let (sender, compute_reciever) = mpsc::sync_channel(1);
        let worker_thread =
            std::thread::spawn(move || Self::worker(compute_with, update_period, sender));
        RepeatedCompute {
            update_period,
            worker_thread,
            compute_reciever,
            last_value: None,
        }
    }
    fn worker(
        mut compute_with: impl FnMut() -> T + Send + 'static,
        update_period: Duration,
        sender: mpsc::SyncSender<T>,
    ) {
        let mut last_compute_start;
        loop {
            let compute_with = AssertUnwindSafe(&mut compute_with);
            last_compute_start = Instant::now();
            let val = match std::panic::catch_unwind(compute_with) {
                Ok(val) => val,
                Err(err) => {
                    error!("Worker thread's compute function panicked: {err:?}");
                    continue;
                }
            };
            if sender.send(val).is_err() {
                // Reciever disconnected
                break;
            };
            std::thread::sleep(update_period.saturating_sub(last_compute_start.elapsed()));
        }
    }
}

impl<T, E> BackgroundComputeProvider for RepeatedCompute<Result<T, E>>
where
    T: Send + 'static,
    E: Send + 'static + Display,
{
    type Output = T;
    type Error = E;

    fn compute(&mut self) -> Option<&mut Self::Output> {
        self.get_val().and_then(|v| v.as_mut().ok())
    }
    fn get_error(&self) -> Option<&Self::Error> {
        self.last_value.as_ref().and_then(|v| v.as_ref().err())
    }
    fn was_attempted(&self) -> bool {
        self.last_value.is_some()
    }
    fn update_period(&self) -> Option<Duration> {
        Some(self.update_period)
    }
    fn is_being_computed(&self) -> bool {
        self.last_value.is_none()
    }
}
impl<T> RepeatedCompute<T>
where
    T: Send + 'static,
{
    fn get_val(&mut self) -> Option<&mut T> {
        // Check if we have a new compute result
        if let Ok(val) = self.compute_reciever.try_recv() {
            self.last_value = Some(val)
        }
        self.last_value.as_mut()
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
    fn was_attempted(&self) -> bool {
        !matches!(self, Self::Computable(_))
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
