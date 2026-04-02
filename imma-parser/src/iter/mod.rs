mod arrow;
mod read;
pub use arrow::*;
pub use read::*;

pub trait FinishingIter {
    type Error;
    fn finish(self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use bytes::Buf;

    use crate::parsers::test::LONG_TEST_BYTES;

    use super::FinishingIter;
    use super::IMMAReadIteratorBuilder;

    use super::IMMAArrowRecordBatchExt;

    #[test]
    fn test_imma_read_iter() {
        let mut i = IMMAReadIteratorBuilder::new(LONG_TEST_BYTES.reader())
            .with_buffer_capacity(10)
            .build();
        let collection: Vec<_> = i.by_ref().collect();
        i.finish().unwrap();
        assert_eq!(collection.len(), 3);
    }

    #[test]
    fn test_imma_arrow_iter() {
        let i = IMMAReadIteratorBuilder::new(LONG_TEST_BYTES.reader()).build();
        let mut arrow_i = i.into_arrow_batches(3, |_| ());
        let collection: Vec<_> = arrow_i.by_ref().collect();
        arrow_i.finish().unwrap();
        assert_eq!(collection.len(), 1);
        assert_eq!(collection[0].1.num_rows(), 3);
    }

    #[test]
    fn test_imma_arrow_iter_with_fn() {
        let mut i = IMMAReadIteratorBuilder::new(LONG_TEST_BYTES.reader()).build();
        let collection: Vec<_> = i.by_ref().collect();
        i.finish().unwrap();
        let i = collection.into_arrow_batches(3, |a| a.position.as_ref().map(|p| p.lo as i64));
        assert_eq!(i.count(), 3);
    }
}
