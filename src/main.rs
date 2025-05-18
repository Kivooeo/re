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
use std::net::SocketAddr;
use std::str;
use tokio::fs::read_dir;
use tokio::io::AsyncWriteExt;
use tokio::{fs::File, net::TcpListener};
use tokio_util::io::ReaderStream;

use whoami;

const NOTFOUND: &[u8] = b"Not Found";
// const FAVICON: &[u8] = include_bytes!("/app/static/favicon.gif");
// const FONT: &[u8] = include_bytes!("/app/static/monocraft.ttc");
// const MONOFONT: &[u8] = include_bytes!("/app/static/jetbrs.ttf");
// const PARROT: &[u8] = include_bytes!("/app/static/lesson.gif");

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    dbg!(std::path::new("/app/static/");

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

async fn handle_favicon() -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/gif")
        .body(
            Full::new(Bytes::from("asd"))
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap())
}

async fn parrot() -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "gif")
        .body(
            Full::new(Bytes::from("asd"))
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap())
}

async fn handle_font() -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from("asd")).map_err(|e| match e {}).boxed())
        .unwrap())
}

async fn handle_monofont() -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(
            Full::new(Bytes::from("asd"))
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap())
}

async fn response_examples(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    dbg!(req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/favicon.ico") => handle_favicon().await,
        (&Method::GET, "/font") => handle_font().await,
        (&Method::GET, "/monofont") => handle_monofont().await,
        (&Method::GET, "/carrot.jpg") => parrot().await,

        (&Method::GET, "/") => {
            match tokio::fs::create_dir_all(format!("C:/Users/{}/.shared", whoami::username()))
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    dbg!(&e);
                }
            };
            list_files().await
        }
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
        (&Method::POST, "/create_file") => {
            let query = req.uri().query().unwrap_or("").to_string();
            let filename = query
                .split("=")
                .nth(1)
                .and_then(|x| percent_decode_str(x).decode_utf8().ok())
                .unwrap_or_else(|| "edited_file.txt".into());

            let body_bytes: Vec<u8> = req.into_body().collect().await.unwrap().to_bytes().to_vec();

            create_file(&filename, &body_bytes).await
        }

        (&Method::POST, "/download") => {
            let query = req.uri().query().unwrap_or("").to_string();
            let filename = query
                .split("=")
                .nth(1)
                .and_then(|x| percent_decode_str(x).decode_utf8().ok())
                .unwrap_or_else(|| "edited_file.txt".into()); // just copied this once again hehehehehehe

            let body_bytes: Vec<u8> = req.into_body().collect().await.unwrap().to_bytes().to_vec();

            save_edited_file(&filename, &body_bytes).await
        }
        (&Method::POST, "/delete_all") => remove_all_files().await,
        (&Method::DELETE, "/delete") => {
            let query = req.uri().query().unwrap_or("").to_string();
            let filename = query
                .split("=")
                .nth(1)
                .and_then(|x| percent_decode_str(x).decode_utf8().ok())
                .unwrap_or_else(|| "uploaded file".into());
            // println!("{body_bytes:?}");
            dbg!(&filename);
            tokio::fs::remove_file(format!(
                "C:/Users/{}/.shared/{filename}",
                whoami::username()
            ))
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

    let file_path = format!(
        "C:/Users/{}/.shared/{}",
        whoami::username(),
        decoded_filename
    );
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

    let file_path = format!(
        "C:/Users/{}/.shared/{}",
        whoami::username(),
        decoded_filename
    );

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
    let file_path = format!(
        "C:/Users/{}/.shared/{}",
        whoami::username(),
        decoded_filename
    );
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

async fn create_file(
    filename: &str,
    filecontent: &[u8],
) -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    let decoded_filename = match percent_decode_str(filename).decode_utf8() {
        Ok(decoded) => decoded.to_string(),
        Err(_) => return Ok(not_found()),
    };

    let file_path = format!(
        "C:/Users/{}/.shared/{}",
        whoami::username(),
        decoded_filename
    );
    let file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)
        .await;
    let _ = file.unwrap().write_all(filecontent).await;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(
            Full::new(Bytes::from("File uploaded successfully"))
                .map_err(|e| match e {})
                .boxed(),
        )
        .unwrap())
}

async fn remove_all_files() -> Result<Response<BoxBody<Bytes, std::io::Error>>> {
    let x = format!("C:/Users/{}/.shared", whoami::username());
    dbg!(&x);
    dbg!(tokio::fs::remove_dir_all(x).await.unwrap());
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
    let x = format!("C:/Users/{}/.shared", whoami::username());
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

    // First part of html

    let mut html = include_str!("index.html").to_string();

    while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
        let path = entry.path();
        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => continue,
        };

        html.push_str(&format!(
            "<li style=\"display: flex; align-items: center; justify-content: space-between;\">
                <span>
                    <a href=\"#\" onclick=\"loadFileContent('{}', event)\">{}</a>
                </span>
                <span style=\"position: relative;\">
                    <button onclick=\"toggleMenu(event, '{}')\" style=\"background: #3c3836; border: 1px solid #504945; color: #d4be98; cursor: pointer; margin-left: 2px; font-size: 16px; padding: 5px; border-radius: 4px;\">
                        ...
                    </button>
                    <div class=\"context-menu\" id=\"menu-{}\" style=\"display: none; position: absolute; right: 0; background: #fbf1c7; border: 1px solid #504945; border-radius: 4px; z-index: 1000;\">
                        <button class=\"aboba\" onclick=\"editFileContent('{}', event)\" style=\"background: #3c3836; border: 1px solid #504945; color: #b8bb26; cursor: pointer; display: block; width: 100%; text-align: left; padding: 5px 10px;\">Edit</button>
                        <button class=\"aboba\" onclick=\"deleteFile('{}', event)\" style=\"background: #3c3836; border: 1px solid #504945; color: #fb4943; cursor: pointer; display: block; width: 100%; text-align: left; padding: 5px 10px;\">Delete</button>
                        <button class=\"aboba\" onclick=\"downloadFile('{}', event)\" style=\"background: #3c3836; border: 1px solid #504945; color: #689d6a; cursor: pointer; display: block; width: 100%; text-align: left; padding: 5px 10px; font-size: 14px;\">Download</button>
                    </div>
                </span>
            </li>",
            file_name, file_name, file_name, file_name, file_name, file_name, file_name
        ));
    }

    // Last part

    html.push_str(include_str!("scripts.html"));

    let response_body = Full::new(Bytes::from(html)).map_err(|e| match e {}).boxed();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(response_body)
        .unwrap())
}
