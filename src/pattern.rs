//! Pattern traits.

use haystack::Hay;

use std::fmt;
use std::mem::replace;

// Axioms:
//
// 1. `p == start_to_end_cursor(end_to_start_cursor(p))`
// 2. `p == end_to_start_cursor(start_to_end_cursor(p))`
// 3. `start_cursor_to_offset(p) == end_cursor_to_offset(start_to_end_cursor(p))`
// 4. `end_cursor_to_offset(p) == start_cursor_to_offset(end_to_start_cursor(p))`
// 5. If `start_cursor_to_offset(b) == end_cursor_to_offset(e)`, then `b.eq_or_before(e)`
// 6. `cursor_range().0 == start_to_end_cursor(cursor_range().0)`
// 7. `cursor_range().1 == end_to_start_cursor(cursor_range().1)`

pub unsafe trait Searcher: Sized {
    type Hay: Hay + ?Sized;

    fn search(&mut self, hay: &Self::Hay) -> Option<(
        <Self::Hay as Hay>::StartCursor,
        <Self::Hay as Hay>::EndCursor,
    )>;
}

pub unsafe trait ReverseSearcher: Searcher {
    fn rsearch(&mut self, hay: &Self::Hay) -> Option<(
        <Self::Hay as Hay>::StartCursor,
        <Self::Hay as Hay>::EndCursor,
    )>;
}

pub unsafe trait DoubleEndedSearcher: ReverseSearcher {}


pub(crate) enum EitherSearcher<T, U> {
    Left(T),
    Right(U),
}

unsafe impl<T, U> Searcher for EitherSearcher<T, U>
where
    T: Searcher,
    U: Searcher<Hay = T::Hay>,
{
    type Hay = T::Hay;

    fn search(&mut self, hay: &T::Hay) -> Option<(
        <T::Hay as Hay>::StartCursor,
        <T::Hay as Hay>::EndCursor,
    )> {
        match self {
            EitherSearcher::Left(left) => left.search(hay),
            EitherSearcher::Right(right) => right.search(hay),
        }
    }
}

unsafe impl<T, U> ReverseSearcher for EitherSearcher<T, U>
where
    T: ReverseSearcher,
    U: ReverseSearcher<Hay = T::Hay>,
{
    fn rsearch(&mut self, hay: &T::Hay) -> Option<(
        <T::Hay as Hay>::StartCursor,
        <T::Hay as Hay>::EndCursor,
    )> {
        match self {
            EitherSearcher::Left(left) => left.rsearch(hay),
            EitherSearcher::Right(right) => right.rsearch(hay),
        }
    }
}



// pub struct SearchDriver<H, P>
// where
//     H: Haystack,
//     P: Pattern<H>,
// {
//     searcher: P::Searcher,
//     rest: H,
//     is_begin: bool,
//     is_end: bool,
// }

// impl<H, P> Clone for SearchDriver<H, P>
// where
//     H: Haystack + Clone,
//     P: Pattern<H>,
//     P::Searcher: Clone,
// {
//     fn clone(&self) -> Self {
//         SearchDriver {
//             searcher: self.searcher.clone(),
//             rest: self.rest.clone(),
//             is_begin: self.is_begin,
//             is_end: self.is_end,
//         }
//     }

//     fn clone_from(&mut self, other: &Self) {
//         self.searcher.clone_from(&other.searcher);
//         self.rest.clone_from(&other.rest);
//         self.is_begin = other.is_begin;
//         self.is_end = other.is_end;
//     }
// }

// impl<H, P> fmt::Debug for SearchDriver<H, P>
// where
//     H: Haystack + fmt::Debug,
//     P: Pattern<H>,
//     P::Searcher: fmt::Debug,
// {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.debug_struct("SearchDriver")
//             .field("searcher", &self.searcher)
//             .field("rest", &self.rest)
//             .field("is_begin", &self.is_begin)
//             .field("is_end", &self.is_end)
//             .finish()
//     }
// }

/*
pub struct SearchDriver<H, P>
where
    H: Haystack,
    P: Pattern<H>,
{
    remaining: Option<H>,
    searcher: P::Searcher,
}

impl_clone_and_debug_for_wrapper! {
    [<H, P>] where [H: Haystack, P: Pattern<H>]
    SearchDriver<H, P> => (H, P::Searcher);
    fields(remaining, searcher)
}

impl<H, P> SearchDriver<H, P>
where
    H: Haystack,
    P: Pattern<H>,
{
    #[inline]
    pub fn new(pattern: P, haystack: H) -> Self {
        Self {
            remaining: Some(haystack),
            searcher: pattern.into_searcher(),
        }
    }

    #[inline]
    pub fn search(&mut self) -> Option<(H, Option<H>)> {
        let remaining = self.remaining.take()?;
        match self.searcher.search(remaining) {
            SearchOutput::Match { before, found, after } => {
                self.remaining = Some(after);
                Some((before, Some(found)))
            }
            SearchOutput::Reject { remaining } => {
                Some((remaining, None))
            }
        }
    }

    #[inline]
    pub fn try_fold<B, F, R>(&mut self, init: B, f: F) -> R
    where
        F: FnMut(B, H, Option<H>) -> R,
        R: Try<Ok = B>,
    {
        if let Some(remaining) = self.remaining.take() {
            let (result, remaining) = self.searcher.try_fold(remaining, init, f);
            self.remaining = remaining;
            result
        } else {
            R::from_ok(init)
        }
    }

    #[inline]
    pub fn remaining(&mut self) -> Option<H> {
        self.remaining.take()
    }
}

impl<H, P> SearchDriver<H, P>
where
    H: Haystack,
    P: Pattern<H>,
    P::Searcher: ReverseSearcher<H>,
{
    #[inline]
    pub fn rsearch(&mut self) -> Option<(Option<H>, H)> {
        let remaining = self.remaining.take()?;
        match self.searcher.rsearch(remaining) {
            SearchOutput::Match { before, found, after } => {
                self.remaining = Some(before);
                Some((Some(found), after))
            }
            SearchOutput::Reject { remaining } => {
                Some((None, remaining))
            }
        }
    }

    #[inline]
    pub fn try_rfold<B, F, R>(&mut self, init: B, f: F) -> R
    where
        F: FnMut(B, Option<H>, H) -> R,
        R: Try<Ok = B>,
    {
        if let Some(remaining) = self.remaining.take() {
            let (result, remaining) = self.searcher.try_rfold(remaining, init, f);
            self.remaining = remaining;
            result
        } else {
            R::from_ok(init)
        }
    }
}
*/

/// A pattern
pub trait Pattern<H>: Sized
where
    H: Hay + ?Sized,
{
    type Searcher: Searcher<Hay = H>;

    fn into_searcher(self) -> Self::Searcher;

    fn is_prefix_of(self, hay: &H) -> bool;

    fn trim_start(&mut self, hay: &H) -> H::StartCursor;

    fn is_suffix_of(self, hay: &H) -> bool
    where
        Self::Searcher: ReverseSearcher;

    fn trim_end(&mut self, haystack: &H) -> H::EndCursor
    where
        Self::Searcher: ReverseSearcher;
}
