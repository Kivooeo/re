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

    let mut html = String::from(
        r#"
        <html>
        <head>
        <meta charset="Unicode">
        <script src="https://cdnjs.cloudflare.com/ajax/libs/mammoth/0.3.0/mammoth.browser.min.js"></script>
        <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/default.min.css">
        <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
        <link rel="stylesheet" href="https://unpkg.com/highlightjs@9.16.2/styles/gruvbox-dark.css">
         <link href="https://fonts.googleapis.com/css2?family=Cousine:ital,wght@0,400;0,700;1,400;1,700&family=Space+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap" rel="stylesheet">
            <style>
    body {
        background-color: #1d2021;
        font-family: 'Space Mono', sans-serif;
        font-size: 16px;
        color: #d4be98;
        line-height: 1.6;
        margin: 0;
        padding-bottom: 0px;
        padding-left: 20px;
        padding-right: 20px;
        padding-top: 0px;
        display: flex;
        height: 100vh;
        overflow-x: hidden;  
    }
    #file-list {
        width: 25%;
        max-width: 300px;
        overflow-y: auto;  
        overflow-x: hidden;  
        padding-right: 20px;
        border-right: 2px solid #3c3836;
        position: relative;
    }
    #file-list ul {
        list-style-type: none;
        padding: 0;
        margin: 0;
    }
    #file-list li {
        margin-bottom: 0px;
    }
    #file-list a {
        color: #83a598;
        text-decoration: none;
        font-weight: bold;
        display: block;
        padding: 3px;
        transition: background-color 0.3s;

        overflow: hidden;

    }
    #file-list a:hover {
        background-color: #3c3836;
    }
    #preview {
        flex-grow: 1;
        padding-left: 20px;
        padding-top: 20px;
        overflow-y: auto;
    }
img {
    max-width: 95%;
    max-height: 80%;
    display: block;
    margin: auto;
    padding-top: 20px;
}
    vid {
    max-width: 95%;
    max-height: 80%;
    display: block;
    margin: auto;
    padding-top: 20px;
}
    video {
    max-width: 95%;
    max-height: 80%;
    display: block;
    margin: auto;
    padding-top: 20px;
}


    pre {
        background-color: #1d2021;
        color: #ebdbb2;
        padding: 10px;
        border: 1px solid #504945;
        border-radius: 4px;
        white-space: pre-wrap;
    }
    #search {
        font-family: 'Space Mono', sans-serif;
        font-size: 16px;
        color: #d4be98;
        width: 100%;
        padding: 8px;
        margin-bottom: 10px;
        margin-top: 10px;
        border: 1px solid #504945;
        border-radius: 4px;
        background-color: #1d2021;
        color: #d4be98;
        position: sticky;
        top: 0;
        z-index: 10;
    }
    #search:focus {
        outline: none;
        border-color: #3c3836;
        box-shadow: 0 0 5px #3c3836;
    }
#drop-area {
    border: 2px dashed #d3869b;
    padding: 8px;
    border-radius: 10px;
    background-color: #1d2021;
    color: #d4be98;
    text-align: center;
    margin-bottom: 0;
    margin-top: 10px;
    position: fixed;
    left: 88%;
    bottom: 20px;
    width: 10%;
    z-index: 10;

    
}
/* Gruvbox dark theme for file content editor */
#file-content {
    background-color: #282828;  /* Gruvbox dark background */
    color: #d4be98;  /* Gruvbox light text */
    border: 1px solid #504945;  /* Dark border for contrast */
    padding: 10px;
    width: 100%;
    height: 300px;
    font-family: 'Space Mono', sans-serif;  /* Optional font */
    font-size: 16px;
    resize: none;  /* Disable resizing */
    border-radius: 4px;  /* Optional rounded corners */
    outline: none;  /* Remove default outline */
}

/* Focused state */
#file-content:focus {
    border-color: #83a598;  /* Gruvbox accent color on focus */
    box-shadow: 0 0 5px #83a598;  /* Gruvbox focus shadow */
}

/* Optional button styles */
button {
    background-color: transparent;
    color: #83a598;  /* Gruvbox accent color */
    border: none;
    cursor: pointer;
    font-size: 14px;
    padding: 5px 10px;
    border-radius: 4px;
}

/* Button hover effect */
button:hover {
    background-color: #3c3836;  /* Darker hover effect */
    color: #d4be98;  /* Light text color on hover */
}



#drop-area.highlight {
    background-color: #282828;
}
 
::-webkit-scrollbar {
    width: 8px;         
    height: 8px;        
}


::-webkit-scrollbar-track {
    background: #282828;  


::-webkit-scrollbar-thumb {
    background: #ebdbb2;    
}


::-webkit-scrollbar-thumb:hover {
    background: #ebdbb2;      
}
/* Custom styling for the textarea to match the Gruvbox Dark theme */




</style>

        </head>
        <body>
            <div id="file-list">
                            
                <input type="text" id="search" placeholder="Search files..." onkeyup="filterFiles()">

                <ul id="file-items">
    "#,
    );

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

    html.push_str(
        r#"
                </ul>
            </div>
            <div id="preview">Select a file to preview</div> 
            <div id="drop-area">
    <p>Drag & Drop</p>
</div>
            <script>
async function loadFileContent(fileName, event) {
    event.preventDefault();
    const previewDiv = document.getElementById('preview');
    
    previewDiv.innerHTML = 'Loading...';


    const encodedFileName = encodeURIComponent(fileName);

    try {
        const response = await fetch(`/${encodedFileName}`);
        if (response.ok) {
            const contentType = response.headers.get('Content-Type');
            if (contentType.startsWith('image/')) {
                const blob = await response.blob();
                const url = URL.createObjectURL(blob);
                previewDiv.innerHTML = `<img src="${url}" alt="${fileName}">`;
            } else if (fileName.endsWith('.docx')) {
              
                const arrayBuffer = await response.arrayBuffer();
                mammoth.convertToHtml({ arrayBuffer: arrayBuffer })
                    .then(function(result) {
                        previewDiv.innerHTML = result.value;
                    })
                    .catch(function(err) {
                        previewDiv.innerHTML = 'Error converting .docx file.';
                        console.error('Mammoth.js error:', err);
                    });
            }else if (contentType.startsWith('video/')) {
                const blob = await response.blob();
                const url = URL.createObjectURL(blob);
                previewDiv.innerHTML = `<video controls><source src="${url}" type="${contentType}">Your browser does not support the video tag.</video>`;
            } else if (fileName.endsWith('.pdf')) {
         
                    const blob = await response.blob();
                    const url = URL.createObjectURL(blob);
                    previewDiv.innerHTML = `<embed src="${url}" type="application/pdf" width="100%" height="800px">`;
                    
    }else {
    const text = await response.text();


    let fileExt = fileName.split('.').pop().toLowerCase();
    let languageClass = '';

    switch (fileExt) {
        case 'rs':
            languageClass = 'rust';
            break;
        case 'py':
            languageClass = 'python';
            break;
        case 'js':
            languageClass = 'javascript';
            break;
        case 'html':
            languageClass = 'html';
            break;
        case 'css':
            languageClass = 'css';
            break;
        case 'json':
            languageClass = 'json';
            break;
        case 'toml':
            languageClass = 'toml';
            break;
        case 'yaml':
        case 'yml':
            languageClass = 'yaml';
            break;
        case 'md':
            languageClass = 'markdown';
            break;
        case 'sh':
            languageClass = 'bash';
            break;
        case 'c':
        case 'h':
            languageClass = 'c';
            break;
        case 'cpp':
        case 'cc':
        case 'cxx':
            languageClass = 'cpp';
            break;
        default:
            languageClass = 'plaintext';
    }


    previewDiv.innerHTML = `<pre><code class="language-${languageClass}">${text}</code></pre>`;


    hljs.highlightAll();
}
        } else {
            previewDiv.innerHTML = 'Error loading file';
        }
    } catch (error) {
        previewDiv.innerHTML = 'Error loading file';
    }
}


             
                function filterFiles() {
                    const searchInput = document.getElementById('search').value.toLowerCase();
                    const fileItems = document.querySelectorAll('#file-items li');

                    fileItems.forEach(item => {
                        const fileName = item.textContent.toLowerCase();
                        if (fileName.includes(searchInput)) {
                            item.style.display = '';
                        } else {
                            item.style.display = 'none';
                        }
                    });
                }
                const dropArea = document.getElementById('drop-area');

dropArea.addEventListener('dragover', (event) => {
    event.preventDefault();
    dropArea.classList.add('highlight');
});

dropArea.addEventListener('dragleave', () => {
    dropArea.classList.remove('highlight');
});

dropArea.addEventListener('drop', (event) => {
    event.preventDefault();
    dropArea.classList.remove('highlight');
    const files = event.dataTransfer.files;
    handleFiles(files);
});

async function handleFiles(files) {
    for (const file of files) {
        try {
      
            const fileArrayBuffer = await file.arrayBuffer();
            const fileBytes = new Uint8Array(fileArrayBuffer);

            const response = await fetch(`/upload?filename=${encodeURIComponent(file.name)}`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/octet-stream', 
                    'Content-Length': file.size.toString(),    
                },
                body: fileBytes,  
            });

            if (response.ok) {
                updateFileList();
            } else {

            }
        } catch (error) {
            console.error('Error uploading file:', error);
            alert('Error uploading file');
        }
    }
}

async function updateFileList() {
    try {
        const response = await fetch('/');
        if (response.ok) {
            const html = await response.text();
            const fileListContainer = document.getElementById('file-items');
            fileListContainer.innerHTML = '';  

            const tempDiv = document.createElement('div');
            tempDiv.innerHTML = html;
            const newFileListItems = tempDiv.querySelectorAll('#file-items li');

            newFileListItems.forEach(item => {
                fileListContainer.appendChild(item);
            });
        } else {
            console.error('Failed to fetch file list');
        }
    } catch (error) {
        console.error('Error fetching file list:', error);
    }
}
    async function deleteFile(fileName, event) {

    event.preventDefault();

    const confirmation = confirm(`are you sure that you want to delete this file: ${fileName}?`);
    if (!confirmation) {
        return; 
    }

    
    const fileItem = event.target.closest('li'); 
    try {
       
        const response = await fetch(`/delete?filename=${encodeURIComponent(fileName)}`, {
            method: 'DELETE'
        });

        if (response.ok) {
          
            fileItem.remove();

           
         

            alert(`file ${fileName} deleted.`);
        } else {
          
            alert('error while deleting.');
        }
    } catch (error) {
      
        console.error('error while deleeting:', error);
        alert('error while deleting file unwak');
    }
}
async function editFileContent(fileName, event) {
    event.preventDefault();
    const previewDiv = document.getElementById('preview');
    
    previewDiv.innerHTML = 'Editing...';

    const encodedFileName = encodeURIComponent(fileName);

    try {
        const response = await fetch(`/${encodedFileName}`);
        if (response.ok) {
            const text = await response.text();
            previewDiv.innerHTML = `
                <textarea id="file-content">${text}</textarea>
                <button onclick="saveEditedFile('${encodedFileName}')">Save</button>
            `;
        } else {
            previewDiv.innerHTML = 'Error loading file for editing';
        }
    } catch (error) {
        previewDiv.innerHTML = 'Error loading file for editing';
    }
}

async function saveEditedFile(fileName) {
    const editedContent = document.getElementById('file-content').value;  // Fixed ID here

    try {
        const response = await fetch(`/edit?filename=${encodeURIComponent(fileName)}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'text/plain',
            },
            body: editedContent,
        });

        if (response.ok) {
            alert('File saved successfully!');
        } else {
            alert('Error saving file.');
        }
    } catch (error) {
        console.error('Error saving file:', error);
        alert('Error saving file.');
    }
}


            </script>
        </body>
        </html>
    "#,
    );

    let response_body = Full::new(Bytes::from(html)).map_err(|e| match e {}).boxed();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(response_body)
        .unwrap())
}
