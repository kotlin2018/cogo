#[macro_use]
extern crate cogo;

use std::io::{self, Read, Write};
use std::time::Duration;

use cogo::io::CoIo;

// create the io object that can be used in coroutine
// note that we can only access the io ojbect in one thread/coroutin
// this example can't run on windows
// because stdin/stdout can't resiger on IOCP
fn main() {
    // run every thing in a single thread to verify the aysnc io
    cogo::config().set_workers(1);
    let b_run = true;

    let r = join!(
        {
            // this will block the whole thread, use Coio instead!!
            // let mut stdin = io::stdin();

            // the CoIo object will not block the thread.
            let mut stdin = CoIo::new(io::stdin()).expect("failed to create stdio");
            while b_run {
                let mut msg = [0, 4];
                stdin.read(&mut msg).expect("failed ot read stdio");
                println!("another coroutine, msg={:?}", msg);
            }
        },
        {
            let mut stdout = CoIo::new(io::stdout()).expect("failed to create stdout");
            while b_run {
                write!(stdout, "write from coroutine\n").expect("failed to write");
                cogo::coroutine::sleep(Duration::from_millis(500));
            }
        }
    );
}
