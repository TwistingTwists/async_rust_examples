//! This example demonstrates manual future implementation and waking mechanism in Rust.
//! It shows how futures work under the hood with a waker system for async/await.

use std::{sync::{Arc, Mutex}, task::Waker, future::Future, pin::Pin, task::Context, task::Poll};

/// A custom Future implementation that can be manually woken up.
/// Uses shared state to communicate between the future and the waker.
struct MyFuture {
    state: Arc<Mutex<MyFutureState>>,
}

struct MyFutureState {
    /// Holds the data that will be returned when the future completes
    data: Option<String>,
    /// Stores the waker that will be used to notify the runtime when data is ready
    waker_state: Option<Waker>    
}

impl MyFuture {
    /// Creates a new MyFuture instance along with its shared state.
    /// Returns a tuple of (MyFuture, SharedState) where SharedState can be used
    /// to wake up the future from another task.
    pub fn new() -> (Self, Arc<Mutex<MyFutureState>>) {
        let state = Arc::new(Mutex::new(MyFutureState { data: None, waker_state: None }));
        (Self { state: state.clone() }, state)
    }
}

impl Future for MyFuture {
    type Output = String;

    /// Implementation of the Future trait's poll method.
    /// This is called by the async runtime to check if the future is complete.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("my_future::Polling the future");
        let mut state = self.state.lock().unwrap();
        if state.data.is_some() {
            let data = state.data.take().unwrap();
            println!("my_future:: Poll::Ready");
            Poll::Ready(data)
        } else {
            state.waker_state = Some(cx.waker().clone());
            println!("my_future waker_state set :: Poll::Pending");
            Poll::Pending
        }
    }
}

#[tokio::main]
async fn main() {
    // Create our custom future and get its shared state
    let (my_future, state) = MyFuture::new();  
    println!("initialised myfuture");

    // Create a channel for communicating between tasks
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(1);

    // Spawn a task that will eventually wake up our future
    let trigger_task = tokio::task::spawn(async move {
        println!("trigger_task:: waiting for data");
        let received_data = rx.recv().await.unwrap();
        println!("trigger_task:: received data: {}", received_data);

        // When data is received, set it in the future's state and wake the future
        let mut state = state.lock().unwrap();
        state.data = Some("coming from trigger_task".to_string());
        if let Some(waker) = state.waker_state.take() {
            println!("trigger_task::waking up the future");
            waker.wake();
        }
    });

    // Spawn our future in a separate task
    let my_future_task = tokio::task::spawn(async move {
        println!("my_future_task:: started");
        my_future.await
    });

    // Send data through the channel to trigger the future
    let _a = tx.send("Hello, world!".to_string()).await.unwrap();

    // Give the future task a chance to start and register its waker
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Wait for both tasks to complete
    let _my_future_data = my_future_task.await.unwrap();
    // [DOES NOT WORK] let _a = tx.send("Hello, world!".to_string()).await.unwrap();

    trigger_task.await.unwrap();
    // [DOES NOT WORK] let _a = tx.send("Hello, world!".to_string()).await.unwrap();

    println!("main::result: {}", _my_future_data);
}
