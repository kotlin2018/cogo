# cogo
cogo coroutine for rust

Cogo is a high-performance library for programming stackful coroutines with which you can easily develop and maintain massive concurrent programs. It can be thought as the Rust version of the popular [Goroutine][go].


Initial code frok from [May](https://github.com/Xudong-Huang/may) and we add Many improvements(Inspired by ```Golang``` and [crossbeam](https://github.com/crossbeam-rs/crossbeam))

----------

## Performance

* platform(16CPU/32 threads,32GB mem,Os:Unbutu-20.04)
* [TechEmpowerBench fork project](https://github.com/zhuxiujia/FrameworkBenchmarks)

![per](docs/629a066aaa37b4c295fa794c5ebdf31.png)

----------

----------
# cogo-std libraray
* ``` cogo/std/http ``` An HTTP server is available，An HTTP Client(TODO)
* ``` cogo/std/queue ```
* ``` cogo/std/sync ```  Includes basic mutex, WaitGroup, and other common synchronization constructs

## Crates based on cogo implementation
* [cdbc](https://github.com/co-rs/cdbc)  High-performance database drivers include mysql, Postgres, AND SQLite
* [fast_log](https://github.com/co-rs/fast_log) an fast log impl
* [cogo-redis](https://github.com/co-rs/cogo-redis) TODO: an redis client
* [cogo-grpc](https://github.com/co-rs/cogo-grpc) TODO: an grpc server/client

## Features
* The stackful coroutine implementation is based on [generator][generator];
* Support schedule on a configurable number of threads for multi-core systems;
* Support coroutine version of a local storage ([CLS][cls]);
* Support efficient asynchronous network I/O;
* Support efficient timer management;
* Support standard synchronization primitives, a semaphore, an MPMC channel, etc;
* Support cancellation of coroutines;
* Support graceful panic handling that will not affect other coroutines;
* Support scoped coroutine creation;
* Support general selection for all the coroutine API;
* All the coroutine API are compatible with the standard library semantics;
* All the coroutine API can be safely called in multi-threaded context;
* Both stable, beta, and nightly channels are supported;
* x86_64 GNU/Linux, x86_64 Windows, x86_64 Mac, aarch64 Linux OS are supported.


* Support High performance channel(2 times better performance, Support the buffer);
* Support WaitGroup(like golang)
* Support Disable stealing and commit directly to the local queue(3 times better go!()/spawn() performance)

----------

## Usage
A naive echo server implemented with Cogo:
```rust
#[macro_use]
extern crate cogo;

use cogo::net::TcpListener;
use std::io::{Read, Write};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();
    while let Ok((mut stream, _)) = listener.accept() {
        go!(move || {
            let mut buf = vec![0; 1024 * 16]; // alloc in heap!
            while let Ok(n) = stream.read(&mut buf) {
                if n == 0 {
                    break;
                }
                stream.write_all(&buf[0..n]).unwrap();
            }
        });
    }
}

```

----------

## More examples

### The I/O heavy bound examples
* [An echo server](examples/src/echo.rs)
* [An echo client](examples/src/echo_client.rs)
* [simple HTTP](examples/src/http.rs)
* [simple HTTPS](examples/src/https.rs)
* [tiny  HTTP](examples/src/http-tiny.rs)
* [WebSockets](examples/src/websocket.rs)


## Caveat
There is a detailed [document][caveat] that describes Cogo's main restrictions. In general, there are four things you should follow when writing programs that use coroutines:
* Don't call thread-blocking API (It will hurt the performance);
* Carefully use Thread Local Storage (access TLS in coroutine might trigger undefined behavior).

> It's considered **unsafe** with the following pattern:
> ```rust
> set_tls();
> // Or another coroutine API that would cause scheduling:
> coroutine::yield_now(); 
> use_tls();
> ```
> but it's **safe** if your code is not sensitive about the previous state of TLS. Or there is no coroutines scheduling between **set** TLS and **use** TLS.

* Don't run CPU bound tasks for long time, but it's ok if you don't care about fairness;
* Don't exceed the coroutine stack. There is a guard page for each coroutine stack. When stack overflow occurs, it will trigger segment fault error.

**Note:**
> The first three rules are common when using cooperative asynchronous libraries in Rust. Even using a futures-based system also have these limitations. So what you should really focus on is a coroutine stack size, make sure it's big enough for your applications.

----------

## How to tune a stack size

```rust
cogo::config().set_stack_size(8*1024);//default is 4k=4*1024
```

----------
