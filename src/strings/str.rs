use pattern::*;
use haystack::{Span, Haystack};
use slices::slice::{TwoWaySearcher, NaiveSearcher, SliceSearcher};
use std::ops::Range;

unsafe impl<'p> Searcher<str> for TwoWaySearcher<'p, u8> {
    #[inline]
    fn search(&mut self, span: Span<&str>) -> Option<Range<usize>> {
        let (hay, range) = span.into_parts();
        self.next(hay.as_bytes(), range)
    }

    fn consume(&mut self, span: Span<&str>) -> Option<usize> {
        self.consume(span.as_bytes())
    }
}

unsafe impl<'p> ReverseSearcher<str> for TwoWaySearcher<'p, u8> {
    #[inline]
    fn rsearch(&mut self, span: Span<&str>) -> Option<Range<usize>> {
        let (hay, range) = span.into_parts();
        self.next_back(hay.as_bytes(), range)
    }

    fn rconsume(&mut self, span: Span<&str>) -> Option<usize> {
        self.rconsume(span.as_bytes())
    }
}

unsafe impl<'p> Searcher<str> for NaiveSearcher<'p, u8> {
    #[inline]
    fn search(&mut self, span: Span<&str>) -> Option<Range<usize>> {
        self.search(span.as_bytes())
    }

    #[inline]
    fn consume(&mut self, span: Span<&str>) -> Option<usize> {
        self.consume(span.as_bytes())
    }

    #[inline]
    fn trim_start(&mut self, hay: &str) -> usize {
        self.trim_start(hay.as_bytes())
    }
}

unsafe impl<'p> ReverseSearcher<str> for NaiveSearcher<'p, u8> {
    #[inline]
    fn rsearch(&mut self, span: Span<&str>) -> Option<Range<usize>> {
        self.rsearch(span.as_bytes())
    }

    #[inline]
    fn rconsume(&mut self, span: Span<&str>) -> Option<usize> {
        self.rconsume(span.as_bytes())
    }

    #[inline]
    fn trim_end(&mut self, hay: &str) -> usize {
        self.trim_end(hay.as_bytes())
    }
}

macro_rules! impl_pattern {
    (<[$($gen:tt)*]> for $pat:ty) => {
        impl<$($gen)*, H: Haystack<Target = str>> Pattern<H> for $pat {
            type Searcher = SliceSearcher<'p, u8>;

            #[inline]
            fn into_searcher(self) -> Self::Searcher {
                SliceSearcher::new_searcher(self.as_bytes())
            }

            #[inline]
            fn into_consumer(self) -> Self::Searcher {
                SliceSearcher::new_consumer(self.as_bytes())
            }
        }
    }
}

impl_pattern!(<['p]> for &'p str);
#[cfg(feature = "std")]
impl_pattern!(<['p]> for &'p String);
impl_pattern!(<['q, 'p]> for &'q &'p str);
