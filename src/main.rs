use bytes::Bytes;
use futures_util::TryStreamExt;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, Result, StatusCode};
use hyper_util::rt::TokioIo;
use mime_guess::from_path;
use percent_encoding::percent_decode_str;
use std::net::{SocketAddr, ToSocketAddrs};
use std::str;
use tokio::fs::read_dir;
use tokio::io::AsyncWriteExt;
use tokio::{fs::File, net::TcpListener};
use tokio_util::io::ReaderStream;
use whoami;
static NOTFOUND: &[u8] = b"Not Found";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    match tokio::fs::create_dir_all(format!("/home/{}/.shared", whoami::username())).await {
        Ok(_) => {}
        Err(e) => {
            dbg!(&e);
        }
    }
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    println!("{:?}", std::env::current_dir());

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(response_examples))
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

async fn response_examples(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    dbg!(req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => list_files().await,
        (&Method::GET, a) => simple_file_send(a).await,
        (&Method::POST, "/upload") => {
            let query = req.uri().query().unwrap_or("").to_string();
            let filename = query
                .split("=")
                .nth(1)
                .and_then(|x| percent_decode_str(x).decode_utf8().ok())
                .unwrap_or_else(|| "uploaded file".into());
            let body_bytes: Vec<u8> = req.into_body().collect().await.unwrap().to_bytes().to_vec();
            // println!("{body_bytes:?}");
            simple_flie_load(&filename, &body_bytes).await
        }
        (&Method::POST, "/edit") => {
            let query = req.uri().query().unwrap_or("").to_string();
            let filename = query
                .split("=")
                .nth(1)
                .and_then(|x| percent_decode_str(x).decode_utf8().ok())
                .unwrap_or_else(|| "edited_file.txt".into());

            let body_bytes: Vec<u8> = req.into_body().collect().await.unwrap().to_bytes().to_vec();

            save_edited_file(&filename, &body_bytes).await
        }

        (&Method::DELETE, _) => {
            let query = req.uri().query().unwrap_or("").to_string();
            let filename = query
                .split("=")
                .nth(1)
                .and_then(|x| percent_decode_str(x).decode_utf8().ok())
                .unwrap_or_else(|| "uploaded file".into());
            // println!("{body_bytes:?}");
            tokio::fs::remove_file(format!("/home/{}/.shared/{filename}", whoami::username()))
                .await
                .unwrap();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(
                    Full::new(Bytes::from("File uploaded successfully"))
                        .map_err(|e| match e {})
                        .boxed(),
                )
                .unwrap())
        }
        _ => Ok(not_found()),
    }
}

fn not_found() -> Response<BoxBody<Bytes, std::io::Error>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(NOTFOUND.into()).map_err(|e| match e {}).boxed())
        .unwrap()
}

async fn simple_file_send(filename: &str) -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    let decoded_filename = match percent_decode_str(filename).decode_utf8() {
        Ok(decoded) => decoded.to_string(),
        Err(_) => return Ok(not_found()),
    };

    let file_path = format!("/home/{}/.shared/{}", whoami::username(), decoded_filename);
    let file = File::open(&file_path).await;

    if file.is_err() {
        eprintln!("ERROR: Unable to open file: {}", file_path);
        return Ok(not_found());
    }

    let file = file.unwrap();
    let reader_stream = ReaderStream::new(file);

    let mime_type = from_path(&file_path).first_or_octet_stream();
    let content_type = mime_type.to_string();

    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    let boxed_body = stream_body.boxed();

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .body(boxed_body)
        .unwrap();

    Ok(response)
}

async fn save_edited_file(
    filename: &str,
    filecontent: &[u8],
) -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    let decoded_filename = match percent_decode_str(filename).decode_utf8() {
        Ok(decoded) => decoded.to_string(),
        Err(_) => return Ok(not_found()),
    };

    let file_path = format!("/home/{}/.shared/{}", whoami::username(), decoded_filename);

    // Create or overwrite the file with the new content
    let mut file = tokio::fs::File::create(file_path).await.unwrap();
    file.write_all(filecontent).await.unwrap();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(
            Full::new(Bytes::from("File uploaded successfully"))
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap())
}
async fn simple_flie_load(
    filename: &str,
    filecontent: &[u8],
) -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    let decoded_filename = match percent_decode_str(filename).decode_utf8() {
        Ok(decoded) => decoded.to_string(),
        Err(_) => return Ok(not_found()),
    };
    let file_path = format!("/home/{}/.shared/{}", whoami::username(), decoded_filename);
    let file = tokio::fs::File::create_new(file_path).await;

    if file.is_err() {
        eprintln!("Unable to create file {}", filename);
        return Ok(not_found());
    }

    let mut file = file.unwrap();
    match file.write_all(filecontent).await {
        Ok(_) => println!("succeffulyy"),
        Err(e) => println!("error while writing {e:?}"),
    };
    dbg!(&file);
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(
            Full::new(Bytes::from("File uploaded successfully"))
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap())
}

async fn list_files() -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    let x = format!("/home/{}/.shared", whoami::username());
    let base_dir = std::path::Path::new(&x);
    dbg!(&base_dir);
    let mut entries = match read_dir(base_dir).await {
        Ok(entries) => entries,
        Err(e) => {
            dbg!(&e);
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(
                    Full::new(Bytes::from_static(b"Directory not found"))
                        .map_err(|e| match e {})
                        .boxed(),
                )
                .unwrap());
        }
    };

    let mut html = include_str!("index.html").to_string();

    while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
        let path = entry.path();
        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => continue,
        };

        html.push_str(&format!(
            "<li style=\"display: flex; align-items: center;\">
                <a href=\"#\" onclick=\"loadFileContent('{}', event)\">{}</a>
                <button onclick=\"editFileContent('{}', event)\" style=\"background: none; border: none; color: blue; cursor: pointer; margin-left: 10px; font-size: 16px; padding: 0;\">
                    ✏️
                </button>
                <button onclick=\"deleteFile('{}', event)\" style=\"background: none; border: none; color: red; cursor: pointer; margin-left: 10px; font-size: 16px; padding: 0;\">
                    🗑️
                </button>
            </li>",
            file_name, file_name, file_name, file_name
        ));
    }

    html.push_str(include_str!("scripts.html"));

    let response_body = Full::new(Bytes::from(html)).map_err(|e| match e {}).boxed();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(response_body)
        .unwrap())
}
