use sqlx::postgres::PgPoolOptions;
use tokio::{process::Command, net::{TcpListener, TcpStream}};

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        _ = tokio::signal::ctrl_c().await;
        eprintln!("Violently shutting down");
        std::process::exit(0);
    });

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await.unwrap();

    // create the "files" table, indexed by a TEXT column named "path_and_hash"
    // and with a BYTEA column named "data"
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS files (
            path_and_hash TEXT PRIMARY KEY,
            data BYTEA NOT NULL
        )"
    ).execute(&pool).await.unwrap();

    // create the "revision_files" table indexed by a TEXT column named
    // "revision", a TEXT column named "path_and_hash"
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS revision_files (
            revision_id TEXT NOT NULL,
            path_and_hash TEXT NOT NULL,
            PRIMARY KEY (revision_id, path_and_hash)
        )"
    ).execute(&pool).await.unwrap();

    // create the "latest_revision" table indexed by a TEXT column named "latest"
    // with a "revision_id" TEXT column
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS latest_revision (
            latest TEXT PRIMARY KEY,
            revision_id TEXT NOT NULL
        )"
    ).execute(&pool).await.unwrap();

    println!("All migrations applied");

    for result in ignore::Walk::new("./") {
        // Each item yielded by the iterator is either a directory entry or an
        // error, so either print the path or the error.
        match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        let path = path.to_str().unwrap();
                        let contents = tokio::fs::read(path).await.unwrap();
                        let hash = seahash::hash(&contents[..]);
                        let hash = format!("{:08x}", hash);
                        println!("- found {path}#{hash}");
                    }
                }
            },
            Err(err) => println!("ERROR: {}", err),
        }
    }    

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
