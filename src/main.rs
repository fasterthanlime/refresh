use tokio::{process::Command, net::{TcpListener, TcpStream}};



#[tokio::main]
async fn main() {
    tokio::spawn(async {
        _ = tokio::signal::ctrl_c().await;
        eprintln!("Violently shutting down");
        std::process::exit(0);
    });

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

    tokio::spawn(async move {
        child.wait().await.unwrap();
    });

    // listen on port 8000
    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    let address = listener.local_addr().unwrap();
    println!("Actually listening on http://{address:?}");

    loop {
        // proxy to port 3001
        let (mut downstream, addr) = listener.accept().await.unwrap();
        println!("Accepted connection from {addr}");
        tokio::spawn(async move {
            let mut upstream = TcpStream::connect("127.0.0.1:3001").await.unwrap();

            tokio::io::copy_bidirectional(&mut downstream, &mut upstream).await.unwrap();
        });
    }
}
