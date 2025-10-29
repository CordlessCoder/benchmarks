#![expect(unused)]
use std::{any::Any, fmt::Display};

use eframe::egui;

pub enum BackgroundCompute<T, E, C = fn() -> Result<T, E>> {
    Computable(C),
    Computing(std::thread::JoinHandle<Result<T, E>>),
    Computed(T),
    Failed(E),
    Poisoned(Box<dyn Any + Send + 'static>),
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
    pub fn compute(&mut self) -> Option<&mut T> {
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
    pub fn as_computed(&self) -> Option<&T> {
        match self {
            Self::Computed(val) => Some(val),
            _ => None,
        }
    }
    pub fn is_being_computed(&self) -> bool {
        matches!(self, Self::Computing(_))
    }
    pub fn failed(&self) -> bool {
        matches!(self, Self::Poisoned(_) | Self::Failed(_))
    }
    pub fn get_error(&self) -> Option<&E> {
        match self {
            Self::Failed(err) => Some(err),
            _ => None,
        }
    }
    pub fn as_poison(&self) -> Option<&Box<dyn Any + Send + 'static>> {
        match self {
            Self::Poisoned(poison) => Some(poison),
            _ => None,
        }
    }
}

impl<T, E, C> BackgroundCompute<T, E, C>
where
    T: Send + 'static,
    E: Send + 'static + Display,
    C: Send + 'static + FnOnce() -> Result<T, E>,
{
    pub fn display(&mut self, ui: &mut egui::Ui, on_success: impl FnOnce(&mut egui::Ui, &mut T)) {
        if let Some(value) = self.compute() {
            on_success(ui, value);
        } else if self.is_being_computed() {
            ui.spinner();
        } else if let Some(err) = self.get_error() {
            ui.label(format!("Error: {err}"));
        }
    }
}
