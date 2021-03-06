//! Pattern traits.

use haystack::{Haystack, Hay, Span};

use std::ops::Range;

/// A searcher, for searching a [`Pattern`] from a [`Hay`].
///
/// This trait provides methods for searching for non-overlapping matches of a
/// pattern starting from the front (left) of a hay.
///
/// # Safety
///
/// This trait is marked unsafe because the range returned by its methods are
/// required to lie on valid codeword boundaries in the haystack. This enables
/// users of this trait to slice the haystack without additional runtime checks.
///
/// # Examples
///
/// Implement a searcher which matches `b"Aaaa"` from a byte string.
///
/// ```rust
/// extern crate pattern_3;
/// use pattern_3::*;
/// use std::ops::Range;
///
/// // The searcher for searching `b"Aaaa"`, using naive search.
/// // We are going to use this as a pattern too.
/// struct Aaaa;
///
/// unsafe impl Searcher<[u8]> for Aaaa {
///     // search for an `b"Aaaa"` in the middle of the string, returns its range.
///     fn search(&mut self, span: Span<&[u8]>) -> Option<Range<usize>> {
///         let (hay, range) = span.into_parts();
///
///         let start = range.start;
///         for (i, window) in hay[range].windows(4).enumerate() {
///             if *window == b"Aaaa"[..] {
///                 // remember to include the range offset
///                 return Some((start + i)..(start + i + 4));
///             }
///         }
///
///         None
///     }
///
///     // checks if an `b"Aaaa" is at the beginning of the string, returns the end index.
///     fn consume(&mut self, span: Span<&[u8]>) -> Option<usize> {
///         let (hay, range) = span.into_parts();
///         let end = range.start.checked_add(4)?;
///         if end <= range.end && hay[range.start..end] == b"Aaaa"[..] {
///             Some(end)
///         } else {
///             None
///         }
///     }
/// }
///
/// impl<H: Haystack<Target = [u8]>> pattern_3::Pattern<H> for Aaaa {
///     type Searcher = Self;
///     fn into_searcher(self) -> Self { self }
/// }
///
/// // test with some standard algorithms.
/// let haystack = &b"Aaaaa!!!Aaa!!!Aaaaaaaaa!!!"[..];
/// assert_eq!(
///     ext::split(haystack, Aaaa).collect::<Vec<_>>(),
///     vec![
///         &b""[..],
///         &b"a!!!Aaa!!!"[..],
///         &b"aaaaa!!!"[..],
///     ]
/// );
/// assert_eq!(
///     ext::match_ranges(haystack, Aaaa).collect::<Vec<_>>(),
///     vec![
///         (0..4, &b"Aaaa"[..]),
///         (14..18, &b"Aaaa"[..]),
///     ]
/// );
/// assert_eq!(
///     ext::trim_start(haystack, Aaaa),
///     &b"a!!!Aaa!!!Aaaaaaaaa!!!"[..]
/// );
/// ```
pub unsafe trait Searcher<A: Hay + ?Sized> {
    /// Searches for the first range which the pattern can be found in the span.
    ///
    /// This method is used to support the following standard algorithms:
    ///
    /// * [`matches`](::ext::matches)
    /// * [`contains`](::ext::contains)
    /// * [`match_indices`](::ext::match_indices)
    /// * [`find`](::ext::find)
    /// * [`match_ranges`](::ext::match_ranges)
    /// * [`find_range`](::ext::find_range)
    /// * [`split`](::ext::split)
    /// * [`split_terminator`](::ext::split_terminator)
    /// * [`splitn`](::ext::splitn)
    /// * [`replace_with`](::ext::replace_with)
    /// * [`replacen_with`](::ext::replacen_with)
    ///
    /// The hay and the restricted range for searching can be recovered by
    /// calling `span`[`.into_parts()`](Span::into_parts). The returned range
    /// should be relative to the hay and must be contained within the
    /// restricted range from the span.
    ///
    /// If the pattern is not found, this method should return `None`.
    ///
    /// # Examples
    ///
    /// Search for the locations of a substring inside a string, using the
    /// searcher primitive.
    ///
    /// ```
    /// extern crate pattern_3;
    /// use pattern_3::{Searcher, Pattern, Span};
    ///
    /// let mut searcher = Pattern::<&str>::into_searcher("::");
    /// let span = Span::from("lion::tiger::leopard");
    /// //                     ^   ^      ^        ^
    /// // string indices:     0   4     11       20
    ///
    /// // found the first "::".
    /// assert_eq!(searcher.search(span.clone()), Some(4..6));
    ///
    /// // slice the span to skip the first match.
    /// let span = unsafe { span.slice_unchecked(6..20) };
    ///
    /// // found the second "::".
    /// assert_eq!(searcher.search(span.clone()), Some(11..13));
    ///
    /// // should found nothing now.
    /// let span = unsafe { span.slice_unchecked(13..20) };
    /// assert_eq!(searcher.search(span.clone()), None);
    /// ```
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>>;

    /// Checks if the pattern can be found at the beginning of the span.
    ///
    /// This method is used to implement the standard algorithm
    /// [`starts_with()`](::ext::starts_with) as well as providing the default
    /// implementation for [`.trim_start()`](Searcher::trim_start).
    ///
    /// The hay and the restricted range for searching can be recovered by
    /// calling `span`[`.into_parts()`](Span::into_parts). If a pattern can be
    /// found starting at `range.start`, this method should return the end index
    /// of the pattern relative to the hay.
    ///
    /// If the pattern cannot be found at the beginning of the span, this method
    /// should return `None`.
    ///
    /// # Examples
    ///
    /// Consumes ASCII characters from the beginning.
    ///
    /// ```
    /// extern crate pattern_3;
    /// use pattern_3::{Searcher, Pattern, Span};
    ///
    /// let mut consumer = Pattern::<&str>::into_consumer(|c: char| c.is_ascii());
    /// let span = Span::from("Hi😋!!");
    ///
    /// // consumes the first ASCII character
    /// assert_eq!(consumer.consume(span.clone()), Some(1));
    ///
    /// // slice the span to skip the first match.
    /// let span = unsafe { span.slice_unchecked(1..8) };
    ///
    /// // matched the second ASCII character
    /// assert_eq!(consumer.consume(span.clone()), Some(2));
    ///
    /// // should match nothing now.
    /// let span = unsafe { span.slice_unchecked(2..8) };
    /// assert_eq!(consumer.consume(span.clone()), None);
    /// ```
    fn consume(&mut self, span: Span<&A>) -> Option<A::Index>;

    /// Repeatedly removes prefixes of the hay which matches the pattern.
    ///
    /// This method is used to implement the standard algorithm
    /// [`trim_start()`](::ext::trim_start).
    ///
    /// Returns the start index of the slice after all prefixes are removed.
    ///
    /// A fast generic implementation in terms of
    /// [`.consume()`](Searcher::consume) is provided by default. Nevertheless,
    /// many patterns allow a higher-performance specialization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate pattern_3;
    /// use pattern_3::{Searcher, Pattern, Span};
    ///
    /// let mut consumer = Pattern::<&str>::into_consumer('x');
    /// assert_eq!(consumer.trim_start("xxxyy"), 3);
    ///
    /// let mut consumer = Pattern::<&str>::into_consumer('x');
    /// assert_eq!(consumer.trim_start("yyxxx"), 0);
    /// ```
    #[inline]
    fn trim_start(&mut self, hay: &A) -> A::Index {
        let mut offset = hay.start_index();
        let mut span = Span::from(hay);
        while let Some(pos) = self.consume(span.clone()) {
            offset = pos;
            let (hay, range) = span.into_parts();
            if pos == range.start {
                break;
            }
            span = unsafe { Span::from_parts(hay, pos..range.end) };
        }
        offset
    }
}

/// A searcher which can be searched from the end.
///
/// This trait provides methods for searching for non-overlapping matches of a
/// pattern starting from the back (right) of a hay.
///
/// # Safety
///
/// This trait is marked unsafe because the range returned by its methods are
/// required to lie on valid codeword boundaries in the haystack. This enables
/// users of this trait to slice the haystack without additional runtime checks.
pub unsafe trait ReverseSearcher<A: Hay + ?Sized>: Searcher<A> {
    /// Searches for the last range which the pattern can be found in the span.
    ///
    /// This method is used to support the following standard algorithms:
    ///
    /// * [`rmatches`](::ext::rmatches)
    /// * [`rmatch_indices`](::ext::rmatch_indices)
    /// * [`rfind`](::ext::find)
    /// * [`rmatch_ranges`](::ext::rmatch_ranges)
    /// * [`rfind_range`](::ext::rfind_range)
    /// * [`rsplit`](::ext::rsplit)
    /// * [`rsplit_terminator`](::ext::rsplit_terminator)
    /// * [`rsplitn`](::ext::rsplitn)
    ///
    /// The hay and the restricted range for searching can be recovered by
    /// calling `span`[`.into_parts()`](Span::into_parts). The returned range
    /// should be relative to the hay and must be contained within the
    /// restricted range from the span.
    ///
    /// If the pattern is not found, this method should return `None`.
    ///
    /// # Examples
    ///
    /// Search for the locations of a substring inside a string, using the
    /// searcher primitive.
    ///
    /// ```
    /// extern crate pattern_3;
    /// use pattern_3::{ReverseSearcher, Pattern, Span};
    ///
    /// let mut searcher = Pattern::<&str>::into_searcher("::");
    /// let span = Span::from("lion::tiger::leopard");
    /// //                     ^   ^      ^
    /// // string indices:     0   4     11
    ///
    /// // found the last "::".
    /// assert_eq!(searcher.rsearch(span.clone()), Some(11..13));
    ///
    /// // slice the span to skip the last match.
    /// let span = unsafe { span.slice_unchecked(0..11) };
    ///
    /// // found the second to last "::".
    /// assert_eq!(searcher.rsearch(span.clone()), Some(4..6));
    ///
    /// // should found nothing now.
    /// let span = unsafe { span.slice_unchecked(0..4) };
    /// assert_eq!(searcher.rsearch(span.clone()), None);
    /// ```
    fn rsearch(&mut self, span: Span<&A>) -> Option<Range<A::Index>>;

    /// Checks if the pattern can be found at the end of the span.
    ///
    /// This method is used to implement the standard algorithm
    /// [`ends_with()`](::ext::ends_with) as well as providing the default
    /// implementation for [`.trim_end()`](ReverseSearcher::trim_end).
    ///
    /// The hay and the restricted range for searching can be recovered by
    /// calling `span`[`.into_parts()`](Span::into_parts). If a pattern can be
    /// found ending at `range.end`, this method should return the start index
    /// of the pattern relative to the hay.
    ///
    /// If the pattern cannot be found at the end of the span, this method
    /// should return `None`.
    ///
    /// # Examples
    ///
    /// Consumes ASCII characters from the end.
    ///
    /// ```
    /// extern crate pattern_3;
    /// use pattern_3::{ReverseSearcher, Pattern, Span};
    ///
    /// let mut consumer = Pattern::<&str>::into_consumer(|c: char| c.is_ascii());
    /// let span = Span::from("Hi😋!!");
    ///
    /// // consumes the last ASCII character
    /// assert_eq!(consumer.rconsume(span.clone()), Some(7));
    ///
    /// // slice the span to skip the first match.
    /// let span = unsafe { span.slice_unchecked(0..7) };
    ///
    /// // matched the second to last ASCII character
    /// assert_eq!(consumer.rconsume(span.clone()), Some(6));
    ///
    /// // should match nothing now.
    /// let span = unsafe { span.slice_unchecked(0..6) };
    /// assert_eq!(consumer.rconsume(span.clone()), None);
    /// ```
    fn rconsume(&mut self, hay: Span<&A>) -> Option<A::Index>;

    /// Repeatedly removes suffixes of the hay which matches the pattern.
    ///
    /// This method is used to implement the standard algorithm
    /// [`trim_end()`](::ext::trim_end).
    ///
    /// A fast generic implementation in terms of
    /// [`.rconsume()`](ReverseSearcher::rconsume) is provided by default.
    /// Nevertheless, many patterns allow a higher-performance specialization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// extern crate pattern_3;
    /// use pattern_3::{ReverseSearcher, Pattern, Span};
    ///
    /// let mut consumer = Pattern::<&str>::into_consumer('x');
    /// assert_eq!(consumer.trim_end("yyxxx"), 2);
    ///
    /// let mut consumer = Pattern::<&str>::into_consumer('x');
    /// assert_eq!(consumer.trim_end("xxxyy"), 5);
    /// ```
    #[inline]
    fn trim_end(&mut self, hay: &A) -> A::Index {
        let mut offset = hay.end_index();
        let mut span = Span::from(hay);
        while let Some(pos) = self.rconsume(span.clone()) {
            offset = pos;
            let (hay, range) = span.into_parts();
            if pos == range.end {
                break;
            }
            span = unsafe { Span::from_parts(hay, range.start..pos) };
        }
        offset
    }
}

/// A searcher which can be searched from both end with consistent results.
///
/// Implementing this marker trait enables the following standard algorithms to
/// return [`DoubleEndedIterator`](std::iter::DoubleEndedIterator)s:
///
/// * [`matches`](::ext::matches) / [`rmatches`](::ext::rmatches)
/// * [`match_indices`](::ext::match_indices) / [`rmatch_indices`](::ext::rmatch_indices)
/// * [`match_ranges`](::ext::match_ranges) / [`rmatch_ranges`](::ext::rmatch_ranges)
/// * [`split`](::ext::split) / [`rsplit`](::ext::rsplit)
/// * [`split_terminator`](::ext::split_terminator) / [`rsplit_terminator`](::ext::rsplit_terminator)
/// * [`splitn`](::ext::splitn) / [`rsplitn`](::ext::rsplitn)
///
/// It is also used to support the following standard algorithm:
///
/// * [`trim`](::ext::trim)
///
/// The `trim` function is implemented by calling
/// [`trim_start`](::ext::trim_start) and [`trim_end`](::ext::trim_end) together.
/// This trait encodes the fact that we can call these two functions in any order.
///
/// # Examples
///
/// The searcher of a character implements `DoubleEndedSearcher`, while that of
/// a string does not.
///
/// `match_indices` and `rmatch_indices` are reverse of each other only for a
/// `DoubleEndedSearcher`.
///
/// ```rust
/// extern crate pattern_3;
/// use pattern_3::ext::{match_indices, rmatch_indices};
///
/// // `match_indices` and `rmatch_indices` are exact reverse of each other for a `char` pattern.
/// let forward = match_indices("xxxxx", 'x').collect::<Vec<_>>();
/// let mut rev_backward = rmatch_indices("xxxxx", 'x').collect::<Vec<_>>();
/// rev_backward.reverse();
///
/// assert_eq!(forward, vec![(0, "x"), (1, "x"), (2, "x"), (3, "x"), (4, "x")]);
/// assert_eq!(rev_backward, vec![(0, "x"), (1, "x"), (2, "x"), (3, "x"), (4, "x")]);
/// assert_eq!(forward, rev_backward);
///
/// // this property does not exist on a `&str` pattern in general.
/// let forward = match_indices("xxxxx", "xx").collect::<Vec<_>>();
/// let mut rev_backward = rmatch_indices("xxxxx", "xx").collect::<Vec<_>>();
/// rev_backward.reverse();
///
/// assert_eq!(forward, vec![(0, "xx"), (2, "xx")]);
/// assert_eq!(rev_backward, vec![(1, "xx"), (3, "xx")]);
/// assert_ne!(forward, rev_backward);
/// ```
///
/// `trim` is implemented only for a `DoubleEndedSearcher`.
///
/// ```rust
/// extern crate pattern_3;
/// use pattern_3::ext::{trim_start, trim_end, trim};
///
/// // for a `char`, we get the same trim result no matter which function is called first.
/// let trim_start_first = trim_end(trim_start("xyxyx", 'x'), 'x');
/// let trim_end_first = trim_start(trim_end("xyxyx", 'x'), 'x');
/// let trim_together = trim("xyxyx", 'x');
/// assert_eq!(trim_start_first, "yxy");
/// assert_eq!(trim_end_first, "yxy");
/// assert_eq!(trim_together, "yxy");
///
/// // this property does not exist for a `&str` in general.
/// let trim_start_first = trim_end(trim_start("xyxyx", "xyx"), "xyx");
/// let trim_end_first = trim_start(trim_end("xyxyx", "xyx"), "xyx");
/// // let trim_together = trim("xyxyx", 'x'); // cannot be defined
/// assert_eq!(trim_start_first, "yx");
/// assert_eq!(trim_end_first, "xy");
/// // assert_eq!(trim_together, /*????*/); // cannot be defined
/// ```
pub unsafe trait DoubleEndedSearcher<A: Hay + ?Sized>: ReverseSearcher<A> {}

/// A pattern, a type which can be converted into a searcher.
///
/// When using search algorithms like [`split()`](::ext::split), users will
/// search with a `Pattern` e.g. a `&str`. A pattern is usually stateless,
/// however for efficient searching, we often need some preprocessing and
/// maintain a mutable state. The preprocessed structure is called the
/// [`Searcher`] of this pattern.
///
/// The relationship between `Searcher` and `Pattern` is similar to `Iterator`
/// and `IntoIterator`.
pub trait Pattern<H: Haystack>: Sized
where H::Target: Hay // FIXME: RFC 2089 or 2289
{
    /// The searcher associated with this pattern.
    type Searcher: Searcher<H::Target>;

    /// Produces a searcher for this pattern.
    ///
    /// You should only call the [`.search()`](Searcher::search) and
    /// [`.rsearch()`](ReverseSearcher::rsearch) methods of the returned
    /// instance. Calling other methods may cause panic.
    ///
    /// Use [`.into_consumer()`](Pattern::into_consumer) if you need to execute
    /// [`.consume()`](Searcher::consume) instead.
    fn into_searcher(self) -> Self::Searcher;

    /// Produces a consumer for this pattern.
    ///
    /// You should only call the [`.consume()`](Searcher::consume),
    /// [`.rconsume()`](ReverseSearcher::rconsume),
    /// [`.trim_start()`](Searcher::trim_start) and
    /// [`.trim_end()`](ReverseSearcher::trim_end) methods of the returned
    /// instance. Calling other methods may cause panic.
    ///
    /// Use [`.into_searcher()`](Pattern::into_searcher) if you need to execute
    /// [`.search()`](Searcher::search) instead.
    ///
    /// By default a consumer and a searcher are the equivalent instance (thus
    /// all methods would be available). Some pattern may override this method
    /// when the two needs different optimization strategies. String searching
    /// is an example of this: we use the Two-Way Algorithm when searching for
    /// substrings, which needs to preprocess the pattern. However this is
    /// irrelevant for consuming, which only need to check for string equality
    /// once. Therefore the Searcher for a string would be an `enum` with a
    /// searcher part (using Two-Way Algorithm) and a consumer part (using naive
    /// search).
    #[inline]
    fn into_consumer(self) -> Self::Searcher {
        self.into_searcher()
    }
}

/// Searcher of an empty pattern.
///
/// This searcher will find all empty subslices between any codewords in a
/// haystack.
#[derive(Clone, Debug, Default)]
pub struct EmptySearcher {
    consumed_start: bool,
    consumed_end: bool,
}

unsafe impl<A: Hay + ?Sized> Searcher<A> for EmptySearcher {
    #[inline]
    fn search(&mut self, span: Span<&A>) -> Option<Range<A::Index>> {
        let (hay, range) = span.into_parts();
        let start = if !self.consumed_start {
            self.consumed_start = true;
            range.start
        } else if range.start == range.end {
            return None;
        } else {
            unsafe { hay.next_index(range.start) }
        };
        Some(start..start)
    }

    #[inline]
    fn consume(&mut self, span: Span<&A>) -> Option<A::Index> {
        let (_, range) = span.into_parts();
        Some(range.start)
    }

    #[inline]
    fn trim_start(&mut self, hay: &A) -> A::Index {
        hay.start_index()
    }
}

unsafe impl<A: Hay + ?Sized> ReverseSearcher<A> for EmptySearcher {
    #[inline]
    fn rsearch(&mut self, span: Span<&A>) -> Option<Range<A::Index>> {
        let (hay, range) = span.into_parts();
        let end = if !self.consumed_end {
            self.consumed_end = true;
            range.end
        } else if range.start == range.end {
            return None;
        } else {
            unsafe { hay.prev_index(range.end) }
        };
        Some(end..end)
    }

    #[inline]
    fn rconsume(&mut self, span: Span<&A>) -> Option<A::Index> {
        let (_, range) = span.into_parts();
        Some(range.end)
    }

    #[inline]
    fn trim_end(&mut self, hay: &A) -> A::Index {
        hay.end_index()
    }
}

unsafe impl<A: Hay + ?Sized> DoubleEndedSearcher<A> for EmptySearcher {}
