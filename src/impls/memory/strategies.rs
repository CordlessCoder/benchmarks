use std::hint::black_box;

use super::x64_strategies::*;
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OperationStrategy {
    #[default]
    Generic,
    Int32,
    Int64,
    Int128,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    SSE,
    #[cfg(target_arch = "x86_64")]
    AVX2,
    #[cfg(target_arch = "x86_64")]
    AVX512,
}

impl OperationStrategy {
    pub fn is_enabled(&self) -> bool {
        use OperationStrategy::*;
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        use std::arch::is_x86_feature_detected;
        match self {
            Generic | Int32 | Int64 => true,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SSE => is_x86_feature_detected!("sse"),
            #[cfg(target_arch = "x86_64")]
            AVX2 => is_x86_feature_detected!("avx2"),
            #[cfg(target_arch = "x86_64")]
            AVX512 => is_x86_feature_detected!("avx512f"),
        }
    }
    pub const fn read_fn(&self) -> fn(&mut [u8]) {
        use OperationStrategy::*;
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        use core::arch::x86_64 as x86;
        match self {
            Generic => read_by::<1, u8>,
            Int32 => read_by::<16, u32>,
            Int64 => read_by::<16, u64>,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SSE => |data| {
                for_each_aligned_value::<32, x86::__m128i, x86::__m128i>(data, |val| unsafe {
                    x86::_mm_stream_load_si128(val.as_ptr())
                })
            },
            #[cfg(target_arch = "x86_64")]
            AVX2 => |data| {
                for_each_aligned_value::<64, x86::__m256i, x86::__m256i>(data, |val| unsafe {
                    x86::_mm256_stream_load_si256(val.as_ptr())
                })
            },
            #[cfg(target_arch = "x86_64")]
            AVX512 => |data| {
                for_each_aligned_value::<64, x86::__m512i, x86::__m512i>(data, |val| unsafe {
                    x86::_mm512_stream_load_si512(val.as_ptr())
                })
            },
        }
    }
    pub const fn write_fn(&self) -> fn(&mut [u8]) {
        use OperationStrategy::*;
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        use core::arch::x86_64 as x86;
        match self {
            Generic => |data| unsafe {
                data.as_mut_ptr().write_bytes(0xAA, data.len());
            },
            Int32 => |data| unsafe { write_by::<16, u32>(data, 0xAAAAAAAA) },
            Int64 => |data| unsafe { write_by::<16, u64>(data, 0xAAAAAAAAAAAAAAAA) },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SSE => |data| unsafe {
                let value = x86::_mm_set1_epi8(0xAA_u8 as i8);
                for_each_aligned_value::<32, x86::__m128i, ()>(data, |slot| unsafe {
                    x86::_mm_stream_si128(slot.as_mut_ptr(), value);
                })
            },
            #[cfg(target_arch = "x86_64")]
            AVX2 => |data| unsafe {
                let value = x86::_mm256_set1_epi8(0xAA_u8 as i8);
                for_each_aligned_value::<64, x86::__m256i, ()>(data, |slot| unsafe {
                    x86::_mm256_stream_si256(slot.as_mut_ptr(), value);
                })
            },
            #[cfg(target_arch = "x86_64")]
            AVX512 => |data| unsafe {
                let value = x86::_mm512_set1_epi8(0xAA_u8 as i8);
                for_each_aligned_value::<64, x86::__m512i, ()>(data, |slot| unsafe {
                    x86::_mm512_stream_si512(slot.as_mut_ptr(), value);
                })
            },
        }
    }
    pub fn copy_nonoverlapping_fn(&self) -> unsafe fn(*const u8, *mut u8, usize) {
        use OperationStrategy::*;
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        use core::arch::x86_64 as x86;
        use core::mem::size_of;
        match self {
            Generic => |from, to, len| unsafe {
                to.copy_from_nonoverlapping(from, len);
            },
            Int32 => |from, to, len| unsafe {
                type Register = u32;
                for_each_idx_chunked::<16, ()>(len / size_of::<Register>(), |idx| {
                    let from = from.cast::<Register>().add(idx);
                    let to = to.cast::<Register>().add(idx);
                    let val = core::ptr::read(from);
                    core::ptr::write(to, val);
                    black_box(val);
                });
            },
            Int64 => |from, to, len| unsafe {
                type Register = u64;
                for_each_idx_chunked::<16, ()>(len / size_of::<Register>(), |idx| {
                    let from = from.cast::<Register>().add(idx);
                    let to = to.cast::<Register>().add(idx);
                    let val = core::ptr::read(from);
                    core::ptr::write(to, val);
                    black_box(val);
                });
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SSE => |from, to, len| unsafe {
                type Register = x86::__m128i;
                for_each_idx_chunked::<32, ()>(len / size_of::<Register>(), |idx| {
                    let from = from.cast::<Register>().add(idx);
                    let to = to.cast::<Register>().add(idx);
                    let val = x86::_mm_stream_load_si128(from);
                    x86::_mm_stream_si128(to, val);
                });
            },
            #[cfg(target_arch = "x86_64")]
            AVX2 => |from, to, len| unsafe {
                type Register = x86::__m256i;
                for_each_idx_chunked::<64, ()>(len / size_of::<Register>(), |idx| {
                    let from = from.cast::<Register>().add(idx);
                    let to = to.cast::<Register>().add(idx);
                    let val = x86::_mm256_stream_load_si256(from);
                    x86::_mm256_stream_si256(to, val);
                });
            },
            #[cfg(target_arch = "x86_64")]
            AVX512 => |from, to, len| unsafe {
                type Register = x86::__m512i;
                for_each_idx_chunked::<64, ()>(len / size_of::<Register>(), |idx| {
                    let from = from.cast::<Register>().add(idx);
                    let to = to.cast::<Register>().add(idx);
                    let val = x86::_mm512_stream_load_si512(from);
                    x86::_mm512_stream_si512(to, val);
                });
            },
        }
    }
}
