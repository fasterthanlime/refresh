use std::{collections::HashSet, fmt, path::Path};

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::Response,
    routing::post,
    Router,
};
use clap::{Parser, Subcommand};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgListener, PgPoolOptions},
    FromRow, PgPool,
};
use tokio::{
    net::{TcpListener, TcpStream},
    process::Command,
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Deploy,
    Serve,
}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        _ = tokio::signal::ctrl_c().await;
        eprintln!("Violently shutting down");
        std::process::exit(0);
    });

    let cli = Cli::parse();
    match cli.command {
        Commands::Deploy => deploy().await,
        Commands::Serve => {
            let mode = std::env::var("SERVE_MODE")
                .expect("SERVE_MODE must be set to DEPLOY_INGEST or SERVE_FRESH");
            let mode: ServeMode = match mode.as_str() {
                "DEPLOY_INGEST" => ServeMode::DeployIngest,
                "SERVE_FRESH" => ServeMode::ServeFresh,
                _ => panic!("SERVE_MODE must be set to DEPLOY_INGEST or SERVE_FRESH"),
            };
            match mode {
                ServeMode::DeployIngest => serve_deploy_ingest().await,
                ServeMode::ServeFresh => serve_fresh().await,
            }
        }
    }
}

enum ServeMode {
    DeployIngest,
    ServeFresh,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
struct PathAndHash(String);

impl PathAndHash {
    async fn from_path(path: &Path) -> Self {
        let path = path.to_str().unwrap();
        assert!(!path.contains('#'));
        let contents = tokio::fs::read(path).await.unwrap();
        let hash = seahash::hash(&contents[..]);
        let hash = format!("{:08x}", hash);
        Self(format!("{path}#{hash}"))
    }

    fn parts(&self) -> (&str, &str) {
        if let [path, hash] = self.0.split('#').collect::<Vec<_>>()[..] {
            (path, hash)
        } else {
            unreachable!()
        }
    }

    fn path(&self) -> &str {
        self.parts().0
    }

    #[allow(dead_code)]
    fn hash(&self) -> &str {
        self.parts().1
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum ApiRequest {
    ListMissingFiles {
        candidates: Vec<PathAndHash>,
    },
    UploadFiles {
        files: Vec<UploadedFile>,
    },
    MakeRevision {
        // revision id is generated
        files: Vec<PathAndHash>,
    },
}

#[derive(Serialize, Deserialize)]
struct UploadedFile {
    pah: PathAndHash,
    #[serde(with = "serde_bytes")]
    contents: Vec<u8>,
}

impl fmt::Debug for UploadedFile {
    // only show contents length
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadedFile")
            .field("pah", &self.pah)
            .field("contents_len", &self.contents.len())
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum ApiResponse {
    ListMissingFiles { missing: Vec<PathAndHash> },
    UploadFiles { success: bool },
    MakeRevision { success: bool, revision_id: String },
}

async fn serve_deploy_ingest() {
    let pool = mk_pool().await;

    let app = Router::new().route("/api", post(api_post)).with_state(pool);
    let server =
        axum::Server::bind(&"0.0.0.0:9000".parse().unwrap()).serve(app.into_make_service());

    server.await.unwrap();
}

#[axum::debug_handler]
async fn api_post(State(pool): State<PgPool>, payload: Bytes) -> Response<Body> {
    let payload: ApiRequest = postcard::from_bytes(&payload[..]).unwrap();
    println!("Got payload {payload:#?}");

    let payload = match payload {
        ApiRequest::ListMissingFiles { candidates } => {
            let candidates: HashSet<PathAndHash> = candidates.into_iter().collect();

            #[derive(FromRow)]
            struct Row {
                path_and_hash: String,
            }

            let rows: Vec<Row> = {
                let candidates_list = candidates.iter().map(|s| s.0.clone()).collect::<Vec<_>>();
                sqlx::query_as("SELECT path_and_hash FROM files WHERE path_and_hash = ANY($1)")
                    .bind(&candidates_list[..])
                    .fetch_all(&pool)
                    .await
                    .unwrap()
            };

            let present: HashSet<PathAndHash> = rows
                .into_iter()
                .map(|r| PathAndHash(r.path_and_hash))
                .collect();
            let missing: Vec<PathAndHash> = candidates.difference(&present).cloned().collect();

            ApiResponse::ListMissingFiles { missing }
        }
        ApiRequest::UploadFiles { files } => {
            for uf in files {
                let path_and_hash = uf.pah.0;
                let contents = uf.contents.into_boxed_slice();
                sqlx::query("INSERT INTO files (path_and_hash, data) VALUES ($1, $2)")
                    .bind(&path_and_hash)
                    .bind(&contents[..])
                    .execute(&pool)
                    .await
                    .unwrap();
            }

            ApiResponse::UploadFiles { success: true }
        }
        ApiRequest::MakeRevision { files } => {
            let revision_id = rusty_ulid::generate_ulid_string();

            for file in files {
                // insert into revision_files
                sqlx::query(
                    "INSERT INTO revision_files (revision_id, path_and_hash) VALUES ($1, $2)",
                )
                .bind(&revision_id)
                .bind(&file.0)
                .execute(&pool)
                .await
                .unwrap();
            }

            sqlx::query("INSERT INTO latest_revision (latest, revision_id) VALUES ($1, $2) ON CONFLICT (latest) DO UPDATE SET revision_id = $2")
                .bind("yes")
                .bind(&revision_id)
                .execute(&pool)
                .await
                .unwrap();

            sqlx::query("NOTIFY revision").execute(&pool).await.unwrap();

            ApiResponse::MakeRevision {
                success: true,
                revision_id,
            }
        }
    };

    let body = postcard::to_allocvec(&payload).unwrap();
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/postcard")
        .body(body.into())
        .unwrap()
}

async fn serve_fresh() {
    let pool = mk_pool().await;

    tokio::spawn({
        let pool = pool.clone();
        async move {
            let mut listener = PgListener::connect_with(&pool).await.unwrap();
            listener.listen("revision").await.unwrap();
            loop {
                let _notification = listener.recv().await.unwrap();
                println!("Got new revision notification!");
            }
        }
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

            tokio::io::copy_bidirectional(&mut downstream, &mut upstream)
                .await
                .unwrap();
        });
    }
}

async fn deploy() {
    let deploy_ingest_address =
        std::env::var("DEPLOY_INGEST_ADDRESS").expect("DEPLOY_INGEST_ADDRESS must be set");

    let mut candidates = vec![];

    for result in ignore::Walk::new("./") {
        // Each item yielded by the iterator is either a directory entry or an
        // error, so either print the path or the error.
        match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let ph = PathAndHash::from_path(entry.path()).await;
                        candidates.push(ph);
                    }
                }
            }
            Err(err) => println!("ERROR: {}", err),
        }
    }

    let client = reqwest::Client::new();

    let response: ApiResponse = postcard::from_bytes(
        &client
            .post(&format!("{}/api", deploy_ingest_address))
            .body(
                postcard::to_allocvec(&ApiRequest::ListMissingFiles {
                    candidates: candidates.clone(),
                })
                .unwrap(),
            )
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
    )
    .unwrap();

    let missing = match response {
        ApiResponse::ListMissingFiles { missing } => missing,
        _ => unreachable!(),
    };

    let mut files = vec![];
    for f in missing {
        let contents = tokio::fs::read(f.path()).await.unwrap();
        files.push(UploadedFile { pah: f, contents });
    }

    println!("Uploading {} files", files.len());

    // TODO: batch this
    let response = postcard::from_bytes(
        &client
            .post(&format!("{}/api", deploy_ingest_address))
            .body(postcard::to_allocvec(&ApiRequest::UploadFiles { files }).unwrap())
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
    )
    .unwrap();

    match response {
        ApiResponse::UploadFiles { success } => assert!(success),
        _ => unreachable!(),
    }

    println!("All files uploaded");

    // Now make a new revision from "candidates"

    let response = postcard::from_bytes(
        &client
            .post(&format!("{}/api", deploy_ingest_address))
            .body(postcard::to_allocvec(&ApiRequest::MakeRevision { files: candidates }).unwrap())
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
    )
    .unwrap();

    match response {
        ApiResponse::MakeRevision {
            success,
            revision_id,
        } => {
            assert!(success);
            println!("New revision id: {}", revision_id);
        }
        _ => unreachable!(),
    }
}

async fn mk_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();

    // create the "files" table, indexed by a TEXT column named "path_and_hash"
    // and with a BYTEA column named "data"
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS files (
            path_and_hash TEXT PRIMARY KEY,
            data BYTEA NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    // create the "revision_files" table indexed by a TEXT column named
    // "revision", a TEXT column named "path_and_hash"
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS revision_files (
            revision_id TEXT NOT NULL,
            path_and_hash TEXT NOT NULL,
            PRIMARY KEY (revision_id, path_and_hash)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    // create the "latest_revision" table indexed by a TEXT column named "latest"
    // with a "revision_id" TEXT column
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS latest_revision (
            latest TEXT PRIMARY KEY,
            revision_id TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    println!("All migrations applied");

    pool
}
