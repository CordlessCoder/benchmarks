use seq_macro::seq;
use std::{hint::black_box, mem::MaybeUninit};

pub(super) fn for_each_idx_chunked<const MAX_CHUNK: usize, U>(
    len: usize,
    mut cb: impl FnMut(usize) -> U,
) {
    let mut idx = 0;
    macro_rules! run_chunked {
        ($size:expr) => {
            if MAX_CHUNK >= $size {
                while idx + $size < len {
                    seq!(extra in 0..$size {
                        cb(idx + extra);
                    });
                    idx += $size;
                }
            }
        };
    }
    run_chunked!(64);
    run_chunked!(32);
    run_chunked!(16);
    run_chunked!(8);
    run_chunked!(4);
    run_chunked!(2);
    run_chunked!(1);
}

pub(super) fn for_each_aligned_value<const MAX_CHUNK: usize, T, U>(
    data: &mut [u8],
    mut cb: impl FnMut(&mut MaybeUninit<T>) -> U,
) {
    const { assert!(core::mem::size_of::<T>() != 0) };
    unsafe {
        let (_, data, _) = data.align_to_mut::<MaybeUninit<T>>();
        for_each_idx_chunked::<MAX_CHUNK, U>(data.len(), |idx| {
            black_box(cb(black_box(data.get_unchecked_mut(idx))))
        });
    }
}

pub(super) fn read_by<const MAX_CHUNK: usize, T>(data: &mut [u8]) {
    for_each_aligned_value::<MAX_CHUNK, T, T>(data, |v| unsafe { black_box(v.assume_init_read()) });
}

pub(super) unsafe fn write_by<const MAX_CHUNK: usize, T: Copy>(data: &mut [u8], value: T) {
    for_each_aligned_value::<MAX_CHUNK, T, ()>(data, |v| {
        v.write(value);
    });
}
