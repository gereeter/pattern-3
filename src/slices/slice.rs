use pattern::*;
use haystack::SharedSpan;
use std::cmp::{Ordering, max, min};
use std::usize;
use std::ops::Range;

//------------------------------------------------------------------------------
// Two way searcher helpers
//------------------------------------------------------------------------------

type FastSkipByteset = u64;

trait FastSkipOptimization {
    fn byteset_mask(&self) -> FastSkipByteset;
}

impl<T: ?Sized> FastSkipOptimization for T {
    #[inline]
    default fn byteset_mask(&self) -> FastSkipByteset { !0 }
}

impl FastSkipOptimization for u8 {
    #[inline]
    fn byteset_mask(&self) -> FastSkipByteset { 1 << (self & 63) }
}

trait MaximalSuffix: Sized {
    // Compute the maximal suffix of `&[T]`.
    //
    // The maximal suffix is a possible critical factorization (u, v) of `arr`.
    //
    // Returns (`i`, `p`) where `i` is the starting index of v and `p` is the
    // period of v.
    //
    // `order` determines if lexical order is `<` or `>`. Both
    // orders must be computed -- the ordering with the largest `i` gives
    // a critical factorization.
    //
    // For long period cases, the resulting period is not exact (it is too short).
    fn maximal_suffix(arr: &[Self], order: Ordering) -> (usize, usize);

    // Compute the maximal suffix of the reverse of `arr`.
    //
    // The maximal suffix is a possible critical factorization (u', v') of `arr`.
    //
    // Returns `i` where `i` is the starting index of v', from the back;
    // returns immediately when a period of `known_period` is reached.
    //
    // `order_greater` determines if lexical order is `<` or `>`. Both
    // orders must be computed -- the ordering with the largest `i` gives
    // a critical factorization.
    //
    // For long period cases, the resulting period is not exact (it is too short).
    fn reverse_maximal_suffix(arr: &[Self], known_period: usize, order: Ordering) -> usize;
}

// fallback to naive search for non-Ord slices.
impl<T: PartialEq> MaximalSuffix for T {
    default fn maximal_suffix(_: &[Self], _: Ordering) -> (usize, usize) {
        (0, 1)
    }

    default fn reverse_maximal_suffix(_: &[Self], _: usize, _: Ordering) -> usize {
        0
    }
}

impl<T: Ord> MaximalSuffix for T {
    fn maximal_suffix(arr: &[Self], order: Ordering) -> (usize, usize) {
        let mut left = 0; // Corresponds to i in the paper
        let mut right = 1; // Corresponds to j in the paper
        let mut offset = 0; // Corresponds to k in the paper, but starting at 0
                            // to match 0-based indexing.
        let mut period = 1; // Corresponds to p in the paper

        while let Some(a) = arr.get(right + offset) {
            // `left` will be inbounds when `right` is.
            let b = &arr[left + offset];
            match a.cmp(b) {
                Ordering::Equal => {
                    // Advance through repetition of the current period.
                    if offset + 1 == period {
                        right += offset + 1;
                        offset = 0;
                    } else {
                        offset += 1;
                    }
                }
                o if o == order => {
                    // Suffix is smaller, period is entire prefix so far.
                    right += offset + 1;
                    offset = 0;
                    period = right - left;
                }
                _ => {
                    // Suffix is larger, start over from current location.
                    left = right;
                    right += 1;
                    offset = 0;
                    period = 1;
                }
            };
        }
        (left, period)
    }

    fn reverse_maximal_suffix(arr: &[Self], known_period: usize, order: Ordering) -> usize {
        let mut left = 0; // Corresponds to i in the paper
        let mut right = 1; // Corresponds to j in the paper
        let mut offset = 0; // Corresponds to k in the paper, but starting at 0
                            // to match 0-based indexing.
        let mut period = 1; // Corresponds to p in the paper
        let n = arr.len();

        while right + offset < n {
            let a = &arr[n - (1 + right + offset)];
            let b = &arr[n - (1 + left + offset)];
            match a.cmp(b) {
                Ordering::Equal => {
                    // Advance through repetition of the current period.
                    if offset + 1 == period {
                        right += offset + 1;
                        offset = 0;
                    } else {
                        offset += 1;
                    }
                }
                o if o == order => {
                    // Suffix is smaller, period is entire prefix so far.
                    right += offset + 1;
                    offset = 0;
                    period = right - left;
                }
                _ => {
                    // Suffix is larger, start over from current location.
                    left = right;
                    right += 1;
                    offset = 0;
                    period = 1;
                }
            }
            if period == known_period {
                break;
            }
        }
        debug_assert!(period <= known_period);
        left
    }
}

//------------------------------------------------------------------------------
// Two way searcher
//------------------------------------------------------------------------------

struct LongPeriod;
struct ShortPeriod;

trait Period {
    const IS_LONG_PERIOD: bool;
}
impl Period for LongPeriod {
    const IS_LONG_PERIOD: bool = true;
}
impl Period for ShortPeriod {
    const IS_LONG_PERIOD: bool = false;
}

#[derive(Debug)]
pub(crate) struct TwoWaySearcher<'p, T: 'p> {
    // constants
    /// critical factorization index
    crit_pos: usize,
    /// critical factorization index for reversed needle
    crit_pos_back: usize,

    period: usize,

    /// `byteset` is an extension (not part of the two way algorithm);
    /// it's a 64-bit "fingerprint" where each set bit `j` corresponds
    /// to a (byte & 63) == j present in the needle.
    byteset: FastSkipByteset,

    needle: &'p [T],

    // variables
    /// index into needle before which we have already matched
    memory: usize,
    /// index into needle after which we have already matched
    memory_back: usize,
}

impl<'p, T: 'p> Clone for TwoWaySearcher<'p, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'p, T: 'p> Copy for TwoWaySearcher<'p, T> {}

impl<'p, T> TwoWaySearcher<'p, T>
where
    T: PartialEq + 'p,
{
    #[inline]
    fn do_next<P: Period>(&mut self, hay: &[T], range: Range<usize>) -> Option<Range<usize>> {
        let needle = self.needle;

        let mut position = range.start;
        'search: loop {
            // Check that we have room to search in
            // position + needle_last can not overflow if we assume slices
            // are bounded by isize's range.
            let i = position + (needle.len() - 1);
            if i >= range.end {
                return None;
            }
            // let tail_item = &hay[i]; // using get_unchecked here would be slower
            let tail_item = unsafe { hay.get_unchecked(i) };

            // Quickly skip by large portions unrelated to our substring
            if !self.byteset_contains(tail_item) {
                position += needle.len();
                if !P::IS_LONG_PERIOD {
                    self.memory = 0;
                }
                continue 'search;
            }

            // See if the right part of the needle matches
            let start = if P::IS_LONG_PERIOD {
                self.crit_pos
            } else {
                max(self.crit_pos, self.memory)
            };
            for i in start..needle.len() {
                if unsafe { needle.get_unchecked(i) != hay.get_unchecked(position + i) } {
                    position += i - self.crit_pos + 1;
                    if !P::IS_LONG_PERIOD {
                        self.memory = 0;
                    }
                    continue 'search;
                }
            }

            // See if the left part of the needle matches
            let start = if P::IS_LONG_PERIOD { 0 } else { self.memory };
            for i in (start..self.crit_pos).rev() {
                if unsafe { needle.get_unchecked(i) != hay.get_unchecked(position + i) } {
                    position += self.period;
                    if !P::IS_LONG_PERIOD {
                        self.memory = needle.len() - self.period;
                    }
                    continue 'search;
                }
            }

            // We have found a match!
            // Note: add self.period instead of needle.len() to have overlapping matches
            if !P::IS_LONG_PERIOD {
                self.memory = 0; // set to needle.len() - self.period for overlapping matches
            }
            return Some(position..(position + needle.len()));
        }
    }

    #[inline]
    pub(crate) fn next(&mut self, hay: &[T], range: Range<usize>) -> Option<Range<usize>> {
        if self.memory != usize::MAX {
            self.do_next::<ShortPeriod>(hay, range)
        } else {
            self.do_next::<LongPeriod>(hay, range)
        }
    }

    #[inline]
    fn do_next_back<P: Period>(&mut self, hay: &[T], range: Range<usize>) -> Option<Range<usize>> {
        let needle = self.needle;
        let mut end = range.end;
        'search: loop {
            // Check that we have room to search in
            // end - needle.len() will wrap around when there is no more room,
            // but due to slice length limits it can never wrap all the way back
            // into the length of hay.
            if needle.len() + range.start > end {
                return None;
            }
            let front_item = unsafe { hay.get_unchecked(end.wrapping_sub(needle.len())) };

            // Quickly skip by large portions unrelated to our substring
            if !self.byteset_contains(front_item) {
                end -= needle.len();
                if !P::IS_LONG_PERIOD {
                    self.memory_back = needle.len();
                }
                continue 'search;
            }

            // See if the left part of the needle matches
            let crit = if P::IS_LONG_PERIOD {
                self.crit_pos_back
            } else {
                min(self.crit_pos_back, self.memory_back)
            };
            for i in (0..crit).rev() {
                if unsafe { needle.get_unchecked(i) != hay.get_unchecked(end - needle.len() + i) } {
                    end -= self.crit_pos_back - i;
                    if !P::IS_LONG_PERIOD {
                        self.memory_back = needle.len();
                    }
                    continue 'search;
                }
            }

            // See if the right part of the needle matches
            let needle_end = if P::IS_LONG_PERIOD { needle.len() } else { self.memory_back };
            for i in self.crit_pos_back..needle_end {
                if unsafe { needle.get_unchecked(i) != hay.get_unchecked(end - needle.len() + i) } {
                    end -= self.period;
                    if !P::IS_LONG_PERIOD {
                        self.memory_back = self.period;
                    }
                    continue 'search;
                }
            }

            // We have found a match!
            if !P::IS_LONG_PERIOD {
                self.memory_back = needle.len();
            }
            return Some((end - needle.len())..end);
        }
    }

    #[inline]
    pub(crate) fn next_back(&mut self, hay: &[T], range: Range<usize>) -> Option<Range<usize>> {
        if self.memory != usize::MAX {
            self.do_next_back::<ShortPeriod>(hay, range)
        } else {
            self.do_next_back::<LongPeriod>(hay, range)
        }
    }

    #[inline]
    pub(crate) fn new(needle: &'p [T]) -> Self {
        let res_lt = T::maximal_suffix(needle, Ordering::Less);
        let res_gt = T::maximal_suffix(needle, Ordering::Greater);
        let (crit_pos, period) = max(res_lt, res_gt);

        let byteset = Self::byteset_create(needle);

        // A particularly readable explanation of what's going on here can be found
        // in Crochemore and Rytter's book "Text Algorithms", ch 13. Specifically
        // see the code for "Algorithm CP" on p. 323.
        //
        // What's going on is we have some critical factorization (u, v) of the
        // needle, and we want to determine whether u is a suffix of
        // &v[..period]. If it is, we use "Algorithm CP1". Otherwise we use
        // "Algorithm CP2", which is optimized for when the period of the needle
        // is large.
        if needle[..crit_pos] == needle[period..(period + crit_pos)] {
            // short period case -- the period is exact
            // compute a separate critical factorization for the reversed needle
            // x = u' v' where |v'| < period(x).
            //
            // This is sped up by the period being known already.
            // Note that a case like x = "acba" may be factored exactly forwards
            // (crit_pos = 1, period = 3) while being factored with approximate
            // period in reverse (crit_pos = 2, period = 2). We use the given
            // reverse factorization but keep the exact period.
            let crit_pos_back = needle.len() - max(
                T::reverse_maximal_suffix(needle, period, Ordering::Greater),
                T::reverse_maximal_suffix(needle, period, Ordering::Less),
            );

            Self {
                crit_pos,
                crit_pos_back,
                period,
                byteset,
                needle,
                memory: 0,
                memory_back: needle.len(),
            }
        } else {
            Self {
                crit_pos,
                crit_pos_back: crit_pos,
                period: max(crit_pos, needle.len() - crit_pos) + 1,
                byteset,
                needle,
                memory: usize::MAX, // Dummy value to signify that the period is long
                memory_back: usize::MAX,
            }
        }
    }

    #[inline]
    fn byteset_create(needle: &[T]) -> FastSkipByteset {
        needle.iter().fold(0, |a, b| b.byteset_mask() | a)
    }
    #[inline]
    fn byteset_contains(&self, item: &T) -> bool {
        (self.byteset & item.byteset_mask()) != 0
    }
}

//------------------------------------------------------------------------------
// Empty searcher
//------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct EmptySearcher {
    consumed_start: bool,
    consumed_end: bool,
}

impl EmptySearcher {
    #[inline]
    fn next(&mut self, range: Range<usize>) -> Option<Range<usize>> {
        let mut start = range.start;
        if !self.consumed_start {
            self.consumed_start = true;
        } else if range.is_empty() {
            return None;
        } else {
            start += 1;
        }
        Some(start..start)
    }

    #[inline]
    fn next_back(&mut self, range: Range<usize>) -> Option<Range<usize>> {
        let mut end = range.end;
        if !self.consumed_end {
            self.consumed_end = true;
        } else if range.is_empty() {
            return None;
        } else {
            end -= 1;
        }
        Some(end..end)
    }
}

//------------------------------------------------------------------------------
// Slice searcher
//------------------------------------------------------------------------------

#[derive(Debug)]
enum SliceSearcherImpl<'p, T: 'p> {
    TwoWay(TwoWaySearcher<'p, T>),
    Empty(EmptySearcher),
}

#[derive(Debug)]
pub struct SliceSearcher<'p, T: 'p>(SliceSearcherImpl<'p, T>);

#[derive(Debug)]
pub struct SliceChecker<'p, T: 'p>(pub(crate) &'p [T]);

unsafe impl<'p, T> Searcher for SliceSearcher<'p, T>
where
    T: PartialEq + 'p,
{
    type Hay = [T];

    #[inline]
    fn search(&mut self, span: SharedSpan<'_, [T]>) -> Option<Range<usize>> {
        let (hay, range) = span.into_parts();
        match &mut self.0 {
            SliceSearcherImpl::TwoWay(searcher) => searcher.next(hay, range),
            SliceSearcherImpl::Empty(searcher) => searcher.next(range),
        }
    }
}

unsafe impl<'p, T> Checker for SliceChecker<'p, T>
where
    T: PartialEq + 'p,
{
    type Hay = [T];

    #[inline]
    fn is_prefix_of(self, hay: &[T]) -> bool {
        hay.get(..self.0.len()) == Some(self.0)
    }

    #[inline]
    fn trim_start(&mut self, hay: &[T]) -> usize {
        let needle_len = self.0.len();
        if needle_len == 0 {
            return 0;
        }
        let mut i = 0;
        loop {
            let j = i + needle_len;
            if j > hay.len() {
                return hay.len();
            }
            if unsafe { hay.get_unchecked(i..j) != self.0 } {
                return i;
            }
            i = j;
        }
    }
}

unsafe impl<'p, T> ReverseSearcher for SliceSearcher<'p, T>
where
    T: PartialEq + 'p,
{
    #[inline]
    fn rsearch(&mut self, span: SharedSpan<'_, [T]>) -> Option<Range<usize>> {
        let (hay, range) = span.into_parts();
        match &mut self.0 {
            SliceSearcherImpl::TwoWay(searcher) => searcher.next_back(hay, range),
            SliceSearcherImpl::Empty(searcher) => searcher.next_back(range),
        }
    }
}

unsafe impl<'p, T> ReverseChecker for SliceChecker<'p, T>
where
    T: PartialEq + 'p,
{
    #[inline]
    fn is_suffix_of(self, hay: &[T]) -> bool {
        if self.0.len() > hay.len() {
            return false;
        }
        unsafe { hay.get_unchecked((hay.len() - self.0.len())..) == self.0 }
    }

    #[inline]
    fn trim_end(&mut self, hay: &[T]) -> usize {
        let mut j = hay.len();
        if !self.0.is_empty() {
            loop {
                if j < self.0.len() {
                    break;
                }
                if unsafe { hay.get_unchecked((j - self.0.len())..j) } != self.0 {
                    break;
                }
                j -= self.0.len();
            }
        }
        j
    }
}

macro_rules! impl_pattern {
    (<[$($gen:tt)*]> $ty:ty) => {
        impl<$($gen)*> Pattern<$ty> for &'p [T]
        where
            T: PartialEq + 'p,
        {
            type Searcher = SliceSearcher<'p, T>;
            type Checker = SliceChecker<'p, T>;

            #[inline]
            fn into_searcher(self) -> Self::Searcher {
                SliceSearcher(if self.is_empty() {
                    SliceSearcherImpl::Empty(EmptySearcher::default())
                } else {
                    SliceSearcherImpl::TwoWay(TwoWaySearcher::new(self))
                })
            }

            #[inline]
            fn into_checker(self) -> Self::Checker {
                SliceChecker(self)
            }
        }
    }
}

impl_pattern!(<['p, 'h, T]> &'h [T]);
impl_pattern!(<['p, 'h, T]> &'h mut [T]);
impl_pattern!(<['p, T]> Vec<T>);
