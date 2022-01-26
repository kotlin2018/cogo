#[macro_use]
extern crate cogo;

use std::time::Duration;

use cogo::coroutine;
use cogo::net::TcpListener;
use cogo::std::sync::channel::channel;

// general select example that use cqueue
fn main() {
    let (tx1, rx1) = channel();
    let (tx2, rx2) = channel();
    let listener = TcpListener::bind(("0.0.0.0", 1234)).unwrap();

    go!(move || {
        tx2.send("hello").unwrap();
        coroutine::sleep(Duration::from_millis(100));
        tx1.send(42).unwrap();
    });

    let id = select!{
        _ = listener.accept() => {
            println!("got connected")
        },
        _ = coroutine::sleep(Duration::from_millis(1000)) => {

        },
        v = rx1.recv() => {
            println!("rx1 received {:?}",v)
        },
        a = rx2.recv() => {
            println!("rx2 received, a={:?}", a)
        }
    };

    assert_eq!(id, 3);
    assert_eq!(rx1.recv(), Ok(42));
}
