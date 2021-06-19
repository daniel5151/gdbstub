/// Slice extension trait that provides non-panicing variants of several
/// standard library iterators.
pub trait SliceExt<T> {
    /// Variant of [`core::slice::split_mut`] that elides bound checks.
    fn split_mut_no_panic<F>(&mut self, pred: F) -> SplitMut<'_, T, F>
    where
        F: FnMut(&T) -> bool;

    /// Variant of [`core::slice::splitn_mut`] that elides bound checks.
    fn splitn_mut_no_panic<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, T, F>
    where
        F: FnMut(&T) -> bool;
}

impl<T> SliceExt<T> for [T] {
    fn split_mut_no_panic<F>(&mut self, pred: F) -> SplitMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        SplitMut::new(self, pred)
    }

    fn splitn_mut_no_panic<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        SplitNMut {
            iter: SplitMut::new(self, pred),
            count: n,
        }
    }
}

#[derive(Debug)]
pub struct SplitMut<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    v: &'a mut [T],
    pred: P,
    finished: bool,
}

impl<'a, T: 'a, P: FnMut(&T) -> bool> SplitMut<'a, T, P> {
    #[inline]
    pub fn new(slice: &'a mut [T], pred: P) -> Self {
        Self {
            v: slice,
            pred,
            finished: false,
        }
    }

    #[inline]
    fn finish(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            None
        } else {
            self.finished = true;
            Some(core::mem::replace(&mut self.v, &mut []))
        }
    }
}

impl<'a, T, P> Iterator for SplitMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a mut [T];

    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            return None;
        }

        let idx_opt = {
            // work around borrowck limitations
            let pred = &mut self.pred;
            self.v.iter().position(|x| (*pred)(x))
        };
        match idx_opt {
            None => self.finish(),
            Some(idx) => {
                let tmp = core::mem::replace(&mut self.v, &mut []);
                let (head, tail) = tmp.split_at_mut(idx);
                self.v = tail.get_mut(1..)?; // will never fail
                Some(head)
            }
        }
    }
}

/// An private iterator over subslices separated by elements that
/// match a predicate function, splitting at most a fixed number of
/// times.
#[derive(Debug)]
pub struct SplitNMut<'a, T: 'a, P>
where
    P: FnMut(&T) -> bool,
{
    iter: SplitMut<'a, T, P>,
    count: usize,
}

impl<'a, T, P> Iterator for SplitNMut<'a, T, P>
where
    P: FnMut(&T) -> bool,
{
    type Item = &'a mut [T];

    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        match self.count {
            0 => None,
            1 => {
                self.count -= 1;
                self.iter.finish()
            }
            _ => {
                self.count -= 1;
                self.iter.next()
            }
        }
    }
}
