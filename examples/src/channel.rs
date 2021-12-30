use std::time::Duration;
use cogo::coroutine::sleep;
use cogo::go;
use cogo::std::sync::mpsc::channel;


fn main() {
    let (s,r) = channel();
    go!(move ||{
         println!("will sleep 1s");
         sleep(Duration::from_secs(1));
         println!("send");
         s.send(1);
    });
    let rv=r.recv().unwrap();
    println!("recv = {}",rv);
}