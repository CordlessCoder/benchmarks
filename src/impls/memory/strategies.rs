use super::x64_strategies::*;
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OperationStrategy {
    #[default]
    Bytewise,
    Int32,
    Int64,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    SSE,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    AVX2,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    AVX512,
}

impl OperationStrategy {
    pub fn is_enabled(&self) -> bool {
        use OperationStrategy::*;
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        use std::arch::is_x86_feature_detected;
        match self {
            Bytewise | Int32 | Int64 => true,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SSE => is_x86_feature_detected!("sse"),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            AVX2 => is_x86_feature_detected!("avx2"),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            AVX512 => is_x86_feature_detected!("avx512f"),
        }
    }
    pub fn read_fn(&self) -> fn(&[u8]) {
        use OperationStrategy::*;
        match self {
            Bytewise => read_by::<u8>,
        }
    }
    // pub fn write(&self, data: &mut [u8]) {}
    // pub fn copy(&self, from: &mut [u8], to: &mut [u8]) {}
}
