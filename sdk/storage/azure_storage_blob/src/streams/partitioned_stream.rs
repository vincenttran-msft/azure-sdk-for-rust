use std::{
    mem,
    pin::{pin, Pin},
    task::Poll,
};

use azure_core::stream::SeekableStream;
use bytes::Bytes;
use futures::{ready, stream::FusedStream, AsyncRead, Stream};

type AzureResult<T> = azure_core::Result<T>;

pub(crate) struct PartitionedStream {
    inner: Box<dyn SeekableStream>, // Underlying "standard" stream (really from Azure Core)
    buf: Vec<u8>,                   // Buffer
    partition_len: usize,           // Length of each partition
    buf_offset: usize,              // Current buffer offset
    total_read: usize,              // Total bytes read
    inner_complete: bool,           //Whether inner still has stream or not
}

impl PartitionedStream {
    pub(crate) fn new(inner: Box<dyn SeekableStream>, partition_len: usize) -> Self {
        assert!(partition_len > 0); // Ensure non-zero partition size
        Self {
            buf: vec![0u8; std::cmp::min(partition_len, inner.len())], // Ensure buffer is of specified partition_len, or edge case where input stream is smaller than specified partition_len
            inner,
            partition_len,
            buf_offset: 0,
            total_read: 0,
            inner_complete: false,
        }
    }

    // This function returns the current buffer and prepares the PartitionedStream for the next iteration
    fn take(&mut self) -> Vec<u8> {
        let mut ret = mem::replace(
            //mem::replace swaps the current buffer (self.buf) with a new buffer
            &mut self.buf,
            vec![0u8; std::cmp::min(self.partition_len, self.inner.len() - self.total_read)], // Again same logic here to ensure buffer is of partition length or remaining length if less than partition length
        );
        ret.truncate(self.buf_offset); // Trims the buffer to the size of current bytes read in this partition (buf_offset)
        self.buf_offset = 0; // After trimming, reset buf_offset for next partition
        ret
    }
}

impl Stream for PartitionedStream {
    type Item = AzureResult<Bytes>;

    fn poll_next(
        self: Pin<&mut Self>, // Pinned once again so that it stays on the heap
        cx: &mut std::task::Context<'_>, // Context, this is the struct that contains the waker (used to wake up the task when it's ready to make progress and not needlessly polling)
    ) -> Poll<Option<Self::Item>> {
        // Returns Poll, which will either be Ready or Pending
        let this = self.get_mut(); // Get mutable reference to Self so that we can access all the struct fields

        // Infinite loop
        loop {
            if this.inner_complete || this.buf_offset >= this.buf.len() {
                // LHS: Inner stream is exhausted, RHS: Current buffer is full
                let ret = this.take(); // Get next partition
                return if ret.is_empty() {
                    // If empty, stream is done
                    Poll::Ready(None)
                } else {
                    // Otherwise, more data to process
                    Poll::Ready(Some(Ok(Bytes::from(ret))))
                };
                // Hotpath: If buffer isn't full and inner stream isn't exhausted, read more data
            } else {
                // Attempt to read from inner stream into current buffer starting at current buffer offset
                match ready!(pin!(&mut this.inner).poll_read(cx, &mut this.buf[this.buf_offset..])) // ready! here ensures it only goes in if Poll returned Ready
                {
                    // If so, update all offsets and totals by bytes read
                    Ok(bytes_read) => {
                        this.buf_offset += bytes_read;
                        this.total_read += bytes_read;
                        this.inner_complete = bytes_read == 0; // Marks inner as exhausted if no bytes were read (since that means there is no more data to read)
                    }
                    Err(e) => {
                        return Poll::Ready(Some(Err(e.into())));
                    }
                }
            }
        }
    }
}

// This is what makes a FusedStream a fused stream -- has to have the mechanism to let user know if the stream is exhausted
impl FusedStream for PartitionedStream {
    fn is_terminated(&self) -> bool {
        self.inner_complete && self.buf.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use azure_core::stream::BytesStream;
    use futures::TryStreamExt;

    use super::*;

    fn get_random_data(len: usize) -> Vec<u8> {
        let mut data: Vec<u8> = vec![0; len];
        rand::fill(&mut data[..]);
        data
    }

    #[tokio::test]
    async fn partitions_exact_multiple() -> AzureResult<()> {
        for part_count in [2usize, 3, 11, 16] {
            for part_len in [1024usize, 1000, 9999, 1] {
                let data = get_random_data(part_len * part_count);
                let stream =
                    PartitionedStream::new(Box::new(BytesStream::new(data.clone())), part_len);

                let parts: Vec<_> = stream.try_collect().await?;

                assert_eq!(parts.len(), part_count);
                for (i, bytes) in parts.iter().enumerate() {
                    assert_eq!(bytes.len(), part_len);
                    assert_eq!(bytes[..], data[i * part_len..i * part_len + part_len]);
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn partitions_leftover_multiple() -> AzureResult<()> {
        for part_count in [2usize, 3, 11, 16] {
            for part_len in [1024usize, 1000, 9999] {
                for dangling_len in [part_len / 2, 100, 128, 99] {
                    let data = get_random_data(part_len * (part_count - 1) + dangling_len);
                    let stream =
                        PartitionedStream::new(Box::new(BytesStream::new(data.clone())), part_len);

                    let parts: Vec<_> = stream.try_collect().await?;

                    assert_eq!(parts.len(), part_count);
                    for (i, bytes) in parts[..parts.len()].iter().enumerate() {
                        if i == parts.len() - 1 {
                            assert_eq!(bytes.len(), dangling_len);
                            assert_eq!(bytes[..], data[i * part_len..]);
                        } else {
                            assert_eq!(bytes.len(), part_len);
                            assert_eq!(bytes[..], data[i * part_len..i * part_len + part_len]);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn partitions_exactly_one() -> AzureResult<()> {
        for len in [1024usize, 1000, 9999, 1] {
            let data = get_random_data(len);
            let mut stream = PartitionedStream::new(Box::new(BytesStream::new(data.clone())), len);

            let single_partition = stream.try_next().await?.unwrap();

            assert!(stream.try_next().await?.is_none());
            assert_eq!(single_partition[..], data[..]);
        }
        Ok(())
    }

    #[tokio::test]
    async fn partitions_less_than_one() -> AzureResult<()> {
        let part_len = 99999usize;
        for len in [1024usize, 1000, 9999, 1] {
            let data = get_random_data(len);
            let mut stream =
                PartitionedStream::new(Box::new(BytesStream::new(data.clone())), part_len);

            let single_partition = stream.try_next().await?.unwrap();

            assert!(stream.try_next().await?.is_none());
            assert_eq!(single_partition[..], data[..]);
        }
        Ok(())
    }

    #[tokio::test]
    async fn partitions_none() -> AzureResult<()> {
        for part_len in [1024usize, 1000, 9999, 1] {
            let data = get_random_data(0);
            let mut stream =
                PartitionedStream::new(Box::new(BytesStream::new(data.clone())), part_len);

            assert!(stream.try_next().await?.is_none());
        }
        Ok(())
    }
}
