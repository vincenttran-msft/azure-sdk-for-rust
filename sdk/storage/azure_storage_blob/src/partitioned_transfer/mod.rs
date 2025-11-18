use std::{future::Future, num::NonZero};

use futures::{
    future::{self},
    Stream, TryStreamExt,
};

type AzureResult<T> = azure_core::Result<T>; // Creates a convenient type alias so we don't have to give the full qualified path

// TFut and TError are generic types so that they can work with the different types of futures and errors
async fn run_all_with_concurrency_limit<TFut, TErr>(
    mut ops_queue: impl Stream<Item = Result<impl FnOnce() -> TFut, TErr>> + Unpin, // This closure (impl FnOnce() -> TFut) defines that each operation should only be called exactly once
    parallel: NonZero<usize>, // NonZero for >=1
) -> Result<(), TErr>
where
    TFut: Future<Output = Result<(), TErr>>, // This constraint ensures that the future is a contract promised to return a Result or TErr in error case
{
    let parallel = parallel.get();

    // Get next operation from the operations queue stream
    let first_op = match ops_queue.try_next().await? {
        Some(item) => item,    // Successfully matched an operation
        None => return Ok(()), // No operations in the queue, return early
    };

    // Calling first_op() actually executes the closure and returns a Future
    // Box::Pin to keep the future on the heap
    // future::select_all() creates a future that will complete when any of the futures in the vector are complete
    // So on first execution, this will just be the first operation we got from the queue
    // This is the events of "currently running operations"
    let mut get_next_completed_op_future = future::select_all(vec![Box::pin(first_op())]);

    // This is the next closure from the queue to be executed
    // New events will come from here
    let mut get_next_queue_op_future = ops_queue.try_next();
    loop {
        // while max parallel running ops, focus on just running ops
        let mut running_ops = get_next_completed_op_future.into_inner(); // Gets the underlying vector of futures (these are the currently running operations)
        while running_ops.len() >= parallel {
            // Loop condition is if we are not at max parallelism
            let result; // This will hold the Result of the completed operation, if any
            (result, _, running_ops) = future::select_all(running_ops).await; // This returns (Result of completed Future, idx of Future [ignored], remaining Futures)
            result? // Propagate any errors
        }
        get_next_completed_op_future = future::select_all(running_ops); // Put back the remaining running ops into the select_all future for next iteration

        // Future::select() creates a race condition between get_next_queue_op_future and get_next_completed_op_future
        // This select() will return whichever Future comes back first
        match future::select(get_next_queue_op_future, get_next_completed_op_future).await {
            // Below is just error handling for both futures in either case
            future::Either::Left((Err(e), _)) => return Err(e), // New operation from input queue failed
            future::Either::Right(((Err(e), _, _), _)) => return Err(e), // An operation from currently running ops failed

            // This is the matching case for new operation from input queue returns first

            // next op in the queue arrived first
            // Ok(next_op_in_queue) is the new operation from the input queue that finished first
            // running_ops_fut is the future of currently running operations that didn't finish first
            future::Either::Left((Ok(next_op_in_queue), running_ops_fut)) => {
                get_next_queue_op_future = ops_queue.try_next(); // Prepare next operation from input queue for next iteration since we just finished the old one
                get_next_completed_op_future = running_ops_fut; // Put back the running ops future for next iteration

                // Examine what we got from the queue
                match next_op_in_queue {
                    Some(op) => {
                        // Operation found (not end of queue)
                        running_ops = get_next_completed_op_future.into_inner(); // Get the inner Vec of currently running operations
                        running_ops.push(Box::pin(op())); // Push the completed operation onto the running ops
                        get_next_completed_op_future = future::select_all(running_ops);
                    }
                    // Queue is empty, no more operations to start
                    // queue was finished, race is over
                    None => break,
                }
            }
            // This is the matching case for a running operation returns first

            // a running op completed first
            // Ok(_) ignores the result, _ is the idx, remaining_running_ops is the remaining running ops, next_op_fut is the next op from the queue (didn't win in this case)
            future::Either::Right(((Ok(_), _, remaining_running_ops), next_op_fut)) => {
                // select panics on empty iter, so we can't race in this case.
                // forcibly wait for next op in queue and handle it before continuing.
                if remaining_running_ops.is_empty() {
                    // As comments state above, this is a special case we need to handle for empty vector
                    let next_op = match next_op_fut.await? {
                        // Forcibly wait for next operation to complete
                        Some(item) => item,    // If there is one match it
                        None => return Ok(()), // If there is no more operations left
                    };
                    get_next_queue_op_future = ops_queue.try_next(); // Prepare for next iteration since we just consumed current next_op_fut
                    get_next_completed_op_future = future::select_all(vec![Box::pin(next_op())]);
                // Puts the next_op we just completed into the completed ops future

                // This is the normal case where the Vector is non-empty and there are still jobs to run
                } else {
                    get_next_queue_op_future = next_op_fut; // Put back next_op_fut back for next iteration since it didn't win the race in this case
                    get_next_completed_op_future = future::select_all(remaining_running_ops);
                    // Put back remaining running ops for next iteration
                }
            }
        }
    }

    // This is to handle after breaking out of the main loop
    // Gets the underlying Vec of running operations, and if non-empty waits for them to complete
    // This only runs all currently running operations to completion after the input queue is exhausted, as no new operations need to be started (there are none left to start)

    let mut running_ops = get_next_completed_op_future.into_inner();
    while !running_ops.is_empty() {
        let result;
        (result, _, running_ops) = future::select_all(running_ops).await;
        result?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use futures::{ready, FutureExt};

    use super::*;
    use std::{pin::Pin, sync::mpsc::channel, task::Poll, time::Duration};

    #[tokio::test]
    async fn limit_ops() -> AzureResult<()> {
        let parallel = 4usize;
        let num_ops = parallel + 1;
        let wait_time_millis = 10u64;
        let op_time_millis = wait_time_millis + 50;

        let (sender, receiver) = channel();

        // setup a series of operations that send a unique number to a channel
        // we can then assert the expected numbers made it to the channel at expected times
        let ops = (0..num_ops).map(|i| {
            let s = sender.clone();
            Ok(async move || {
                s.send(i).unwrap();
                tokio::time::sleep(Duration::from_millis(op_time_millis)).await;
                AzureResult::<()>::Ok(())
            })
        });

        let race = future::select(
            Box::pin(run_all_with_concurrency_limit(
                futures::stream::iter(ops),
                NonZero::new(parallel).unwrap(),
            )),
            Box::pin(tokio::time::sleep(Duration::from_millis(wait_time_millis))),
        )
        .await;
        match race {
            future::Either::Left(_) => panic!("Wrong future won the race."),
            future::Either::Right((_, run_all_fut)) => {
                let mut items: Vec<_> = receiver.try_iter().collect();
                items.sort();
                assert_eq!(items, (0..parallel).collect::<Vec<_>>());

                run_all_fut.await?;
                assert_eq!(receiver.try_iter().collect::<Vec<_>>().len(), 1);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn slow_stream() -> AzureResult<()> {
        let parallel = 10;
        let num_ops = 5;
        let op_time_millis = 10;
        let stream_time_millis = op_time_millis + 10;
        // setup a series of operations that send a unique number to a channel
        // we can then assert the expected numbers made it to the channel at expected times
        let ops = (0..num_ops).map(|_| {
            Ok(async move || {
                tokio::time::sleep(Duration::from_millis(op_time_millis)).await;
                AzureResult::<()>::Ok(())
            })
        });

        run_all_with_concurrency_limit(
            SlowStream::new(ops, Duration::from_millis(stream_time_millis)),
            NonZero::new(parallel).unwrap(),
        )
        .await
    }

    #[tokio::test]
    async fn empty_ops() -> AzureResult<()> {
        let parallel = 4usize;

        // not possible to manually type what we need
        // make a vec with a concrete element and then remove it to get the desired typing
        let op = || future::ready::<Result<(), azure_core::Error>>(Ok(()));
        let mut ops = vec![Ok(op)];
        ops.pop();

        run_all_with_concurrency_limit(futures::stream::iter(ops), NonZero::new(parallel).unwrap())
            .await
    }

    struct SlowStream<Iter> {
        sleep: Pin<Box<tokio::time::Sleep>>,
        interval: Duration,
        iter: Iter,
    }
    impl<Iter> SlowStream<Iter> {
        fn new(iter: Iter, interval: Duration) -> Self {
            Self {
                sleep: Box::pin(tokio::time::sleep(interval)),
                interval,
                iter,
            }
        }
    }
    impl<Iter: Iterator + Unpin> Stream for SlowStream<Iter> {
        type Item = Iter::Item;

        fn poll_next(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            let this = self.get_mut();
            ready!(this.sleep.poll_unpin(cx));
            this.sleep = Box::pin(tokio::time::sleep(this.interval));
            Poll::Ready(this.iter.next())
        }
    }
}
