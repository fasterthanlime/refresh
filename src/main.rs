use std::{process::Command, os::unix::process::CommandExt, net::TcpStream};


fn main() {
    let mut cmd = Command::new("deno");
    cmd.arg("run").arg("-A").arg("main.ts");
    cmd.env("PORT", "3001");
    unsafe {
        cmd.pre_exec(|| {
            let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            if ret != 0 {
                panic!("prctl failed");
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().unwrap();

    std::thread::spawn(move || {
        child.wait().unwrap();
    });

    // listen on port 8000
    let listener = std::net::TcpListener::bind("0.0.0.0:8000").unwrap();
    let address = listener.local_addr().unwrap();
    println!("Actually listening on http://{address:?}");

    loop {
        // proxy to port 3001
        let (downstream, addr) = listener.accept().unwrap();
        println!("Accepted connection from {addr}");
        std::thread::spawn(move || {
            let upstream = TcpStream::connect("127.0.0.1:3001").unwrap();
            let mut downstream_w = downstream.try_clone().unwrap();
            let mut downstream_r = downstream;

            let mut upstream_w = upstream.try_clone().unwrap();
            let mut upstream_r = upstream;

            std::thread::spawn(move || {
                std::io::copy(&mut downstream_r, &mut upstream_w).unwrap();
                println!("Down copying downstream->upstream");
            });
            std::thread::spawn(move || {
                std::io::copy(&mut upstream_r, &mut downstream_w).unwrap();
                println!("Down copying upstream->downstream");
            });
        });
    }
}
