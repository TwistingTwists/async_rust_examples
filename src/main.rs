use std::{sync::{Arc, Mutex}, task::Waker, future::Future, pin::Pin, task::Context, task::Poll};



struct MyFuture {
    state: Arc<Mutex<MyFutureState>>,
}

struct MyFutureState {

 data: Option<String>, 
 waker_state: Option<Waker>    
}

impl MyFuture {

 pub fn new() -> (Self, Arc<Mutex<MyFutureState>>) {
    let state = Arc::new(Mutex::new(MyFutureState { data: None, waker_state: None }));
    (Self { state: state.clone() }, state)
    
 }
}

impl Future for MyFuture {
    type Output = String;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)
    -> Poll<Self::Output> {
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

    let (my_future, state) = MyFuture::new();  

    println!("initialised myfuture");

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(1);

    let trigger_task = tokio::task::spawn(async move {

        println!("trigger_task:: waiting for data");
        let received_data = rx.recv().await.unwrap();
        println!("trigger_task:: received data: {}", received_data);

        let mut state = state.lock().unwrap();
        state.data = Some("coming from trigger_task".to_string());
        if let Some(waker) = state.waker_state.take() {
            println!("trigger_task::waking up the future");
            waker.wake();
        }
        
    });

    let my_future_task = tokio::task::spawn(async move {
        println!("my_future_task:: started");
        my_future.await
    });
    let _a = tx.send("Hello, world!".to_string()).await.unwrap();

    // Give the future task a chance to start and register its waker
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;


    let _my_future_data = my_future_task.await.unwrap();
    // [DOES NOT WORK] let _a = tx.send("Hello, world!".to_string()).await.unwrap();

    trigger_task.await.unwrap();
    // [DOES NOT WORK] let _a = tx.send("Hello, world!".to_string()).await.unwrap();

    println!("main::result: {}", _my_future_data);
}
