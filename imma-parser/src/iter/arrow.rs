use arrow::array::RecordBatch;

use crate::{arrow::ArrowSerde, types::IMMARecord};
use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use super::FinishingIter;

pub struct IMMAArrowRecordBatchIterator<I, F, K> {
    inner: I,
    group_func: F,
    max_batch_size: usize,
    builders: HashMap<K, ArrowSerde>,
}

impl<I, F, K, R> Iterator for IMMAArrowRecordBatchIterator<I, F, K>
where
    I: Iterator<Item = R>,
    R: Borrow<IMMARecord>,
    F: FnMut(&<I as Iterator>::Item) -> K,
    K: Hash + Clone + Eq,
{
    type Item = (K, RecordBatch);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.inner.next();
            let r = match next {
                Some(r) => r,
                None => {
                    let non_empty_builder = self.builders.iter_mut().find(|(_, v)| !v.is_empty());
                    let (k, builder) = non_empty_builder?;
                    return Some((k.clone(), builder.serialize()));
                }
            };
            let k = (self.group_func)(&r);
            if !self.builders.contains_key(&k) {
                self.builders.insert(k.clone(), ArrowSerde::new());
            }
            let builder = self.builders.get_mut(&k).unwrap();

            if builder.len() >= self.max_batch_size {
                return Some((k, builder.serialize()));
            } else {
                builder.append(r.borrow());
            }
        }
    }
}

pub trait IMMAArrowRecordBatchExt: IntoIterator + Sized {
    /// Produces an iterator which yields [`RecordBatch`] from the [`IMMARecord`] iterator.
    /// Takes an optional closure which will subdivide the record batches based on the closure, this
    /// is useful if you want to partition the original data differently.
    fn into_arrow_batches<F, K>(
        self,
        max_batch_size: usize,
        f: F,
    ) -> IMMAArrowRecordBatchIterator<<Self as IntoIterator>::IntoIter, F, K>
    where
        F: FnMut(&Self::Item) -> K,
        K: Hash + Clone + Eq,
    {
        IMMAArrowRecordBatchIterator {
            inner: self.into_iter(),
            group_func: f,
            max_batch_size,
            builders: HashMap::with_capacity(1),
        }
    }
}

// A blanket impl
impl<I, T> IMMAArrowRecordBatchExt for I
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: Iterator<Item = T>,
    T: Borrow<IMMARecord>,
{
}

impl<I, F, K> FinishingIter for IMMAArrowRecordBatchIterator<I, F, K>
where
    I: FinishingIter,
{
    type Error = <I as FinishingIter>::Error;

    fn finish(self) -> Result<(), Self::Error> {
        self.inner.finish()
    }
}
