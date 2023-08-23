// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::FilterChain;
use core::str;
use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct Replaceable {
    // guaranteed to be valid UTF-8
    content: Vec<u8>,
    // only content[freeze_pre_len..content.len()-freeze_post_len] is mutable
    freeze_pre_len: usize,
    freeze_post_len: usize,
}

// note: would be great to have something like Replaceable::replace_range(&mut self, range) -> &mut Insertable 
// where Insertable supports things like pushing chars, strs, etc, and they would directly go to the corresponding range
// of the backing Vec<u8>. pushing more things than range.len would reallocate the backing vec to make space on the fly,
// pushing fewer things than range.len would move around the tail of the Vec on Insertable::drop to fill the "empty space".
// a "rope" or "cord" to replace the Vec<u8> might also be nice

impl Replaceable {
    pub(crate) fn new(input: String) -> Self {
        Self {
            content: input.into_bytes(),
            freeze_pre_len: 0,
            freeze_post_len: 0,
        }
    }

    // pub(crate) fn splice(&mut self) {
    //     self.content.splice()
    // }

    pub(crate) fn as_str(&self) -> &str {
        debug_assert!(str::from_utf8(&self.content[..]).is_ok());

        unsafe { str::from_utf8_unchecked(&self.content[..]) }
    }

    pub(crate) fn freeze_at(&mut self, pos: usize, len: usize) {
        debug_assert!(pos < self.content.len() && len <= self.content.len() - pos);

        self.freeze_pre_len = pos;
        self.freeze_post_len = self.content.len() - pos - len;
    }

    pub(crate) fn get(&self, pos: usize) -> Option<u8> {
        self.content.get(pos).copied()
    }

    /// Returns the next run (run_start_index, run_length) that occurs after `start`, if one exists.
    pub(crate) fn next_filtered_run(
        &self,
        start: usize,
        filter: &FilterChain,
    ) -> Option<(usize, usize)> {
        debug_assert!(start < self.content.len());

        let run_start = self.find_first_in(start, filter)?;
        let run_end = self.find_first_out(run_start, filter)?;
        let run_length = run_end - run_start;

        Some((run_start, run_length))
    }

    fn find_first_in(&self, start: usize, filter: &FilterChain) -> Option<usize> {
        let tail = &self.as_str()[start..];
        let (idx, _) = tail.char_indices().find(|&(_, c)| filter.contains(c))?;
        Some(start + idx)
    }

    fn find_first_out(&self, start: usize, filter: &FilterChain) -> Option<usize> {
        let tail = &self.as_str()[start..];
        let (idx, _) = tail.char_indices().find(|&(_, c)| !filter.contains(c))?;
        Some(start + idx)
    }
}
