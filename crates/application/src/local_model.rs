//! Ports for optional on-device interpretation. Models never receive a repository.
use crate::AppError;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
};

pub trait QuickEntryModel {
    fn propose(&mut self, source: &str, operation: &ModelOperation) -> Result<String, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelAvailability {
    Missing,
    Installed,
}

/// One operation's cancellation and download progress; not shared financial state.
#[derive(Clone, Default)]
pub struct ModelOperation {
    pub cancelled: Arc<AtomicBool>,
    pub downloaded_bytes: Arc<AtomicU64>,
    phase: Arc<AtomicU8>,
    pub generated_tokens: Arc<AtomicU64>,
}

impl PartialEq for ModelOperation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ModelPhase {
    Queued,
    Verifying,
    Loading,
    Reading,
    Generating,
}

impl ModelOperation {
    pub fn set_phase(&self, phase: ModelPhase) {
        self.phase.store(phase as u8, Ordering::Relaxed);
    }
    pub fn phase(&self) -> ModelPhase {
        match self.phase.load(Ordering::Relaxed) {
            1 => ModelPhase::Verifying,
            2 => ModelPhase::Loading,
            3 => ModelPhase::Reading,
            4 => ModelPhase::Generating,
            _ => ModelPhase::Queued,
        }
    }
}
