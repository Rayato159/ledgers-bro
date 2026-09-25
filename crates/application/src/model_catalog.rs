//! Curated ChatML Qwen3 models; financial validation never depends on model choice.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelId {
    Small,
    Compact,
    Balanced,
    Large,
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalModelInfo {
    pub id: LocalModelId,
    pub name: &'static str,
    pub filename: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    pub url: &'static str,
    pub source: &'static str,
    /// Conservative estimate for weights, 2048-token KV cache and runtime overhead.
    /// This is an app estimate, not a benchmark or a guarantee against OS eviction.
    pub working_memory_bytes: u64,
}

impl LocalModelId {
    pub const ALL: [Self; 5] = [
        Self::Small,
        Self::Compact,
        Self::Balanced,
        Self::Large,
        Self::Advanced,
    ];
    pub fn code(self) -> &'static str {
        match self {
            Self::Small => "qwen3-0.6b",
            Self::Compact => "qwen3-1.7b",
            Self::Balanced => "qwen3-4b",
            Self::Large => "qwen3-8b",
            Self::Advanced => "qwen3-14b",
        }
    }
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.code() == code)
    }
    pub fn info(self) -> LocalModelInfo {
        match self {
            Self::Small => LocalModelInfo {
                id: self,
                name: "Qwen3 0.6B",
                filename: "Qwen3-0.6B-Q4_K_M.gguf",
                bytes: 396705472,
                sha256: "ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a",
                url: "https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/Qwen3-0.6B-Q4_K_M.gguf",
                source: "https://huggingface.co/unsloth/Qwen3-0.6B-GGUF",
                working_memory_bytes: 1536 * 1024 * 1024,
            },
            Self::Compact => LocalModelInfo {
                id: self,
                name: "Qwen3 1.7B",
                filename: "Qwen3-1.7B-Q4_K_M.gguf",
                bytes: 1107409472,
                sha256: "b139949c5bd74937ad8ed8c8cf3d9ffb1e99c866c823204dc42c0d91fa181897",
                url: "https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/resolve/d7f544eead698dbd1f15126ef60b45a1e1933222/Qwen3-1.7B-Q4_K_M.gguf",
                source: "https://huggingface.co/unsloth/Qwen3-1.7B-GGUF",
                working_memory_bytes: 2560 * 1024 * 1024,
            },
            Self::Balanced => LocalModelInfo {
                id: self,
                name: "Qwen3 4B",
                filename: "Qwen3-4B-Q4_K_M.gguf",
                bytes: 2497280256,
                sha256: "7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5",
                url: "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/bc640142c66e1fdd12af0bd68f40445458f3869b/Qwen3-4B-Q4_K_M.gguf",
                source: "https://huggingface.co/Qwen/Qwen3-4B-GGUF",
                working_memory_bytes: 4608 * 1024 * 1024,
            },
            Self::Large => LocalModelInfo {
                id: self,
                name: "Qwen3 8B",
                filename: "Qwen3-8B-Q4_K_M.gguf",
                bytes: 5027783488,
                sha256: "d98cdcbd03e17ce47681435b5150e34c1417f50b5c0019dd560e4882c5745785",
                url: "https://huggingface.co/Qwen/Qwen3-8B-GGUF/resolve/7c41481f57cb95916b40956ab2f0b139b296d974/Qwen3-8B-Q4_K_M.gguf",
                source: "https://huggingface.co/Qwen/Qwen3-8B-GGUF",
                working_memory_bytes: 7680 * 1024 * 1024,
            },
            Self::Advanced => LocalModelInfo {
                id: self,
                name: "Qwen3 14B",
                filename: "Qwen3-14B-Q4_K_M.gguf",
                bytes: 9001752960,
                sha256: "500a8806e85ee9c83f3ae08420295592451379b4f8cf2d0f41c15dffeb6b81f0",
                url: "https://huggingface.co/Qwen/Qwen3-14B-GGUF/resolve/530227a7d994db8eca5ab5ced2fb692b614357fd/Qwen3-14B-Q4_K_M.gguf",
                source: "https://huggingface.co/Qwen/Qwen3-14B-GGUF",
                working_memory_bytes: 12288 * 1024 * 1024,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelDevice {
    pub total_memory: Option<u64>,
    pub available_memory: Option<u64>,
    pub free_disk: Option<u64>,
    pub cpu_threads: usize,
    pub mobile: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSettingsSnapshot {
    pub selected: LocalModelId,
    /// Correct file size only; cryptographic validation still runs before activation/loading.
    pub downloaded: Vec<LocalModelId>,
    pub device: ModelDevice,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFit {
    FitsEstimate,
    LowMemory,
    InsufficientMemory,
    InsufficientDisk,
    MobileCaution,
    Unknown,
}
impl ModelFit {
    pub fn blocked(self) -> bool {
        matches!(self, Self::InsufficientMemory | Self::InsufficientDisk)
    }
}
pub fn model_fit(model: &LocalModelInfo, device: &ModelDevice, needs_download: bool) -> ModelFit {
    if needs_download
        && device
            .free_disk
            .is_some_and(|free| free < model.bytes + 256 * 1024 * 1024)
    {
        return ModelFit::InsufficientDisk;
    }
    if device
        .total_memory
        .is_some_and(|ram| ram < model.working_memory_bytes + 512 * 1024 * 1024)
    {
        return ModelFit::InsufficientMemory;
    }
    if device
        .available_memory
        .is_some_and(|ram| ram < model.working_memory_bytes)
    {
        return ModelFit::LowMemory;
    }
    if device.total_memory.is_none()
        || device.available_memory.is_none()
        || (needs_download && device.free_disk.is_none())
    {
        return ModelFit::Unknown;
    }
    if device.mobile {
        return ModelFit::MobileCaution;
    }
    ModelFit::FitsEstimate
}
