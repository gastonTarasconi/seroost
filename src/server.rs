use std::str;
use std::io;
use std::sync::{Arc, Mutex};

use super::model::*;

use serde_json::json;
use tiny_http::{Server, Request, Response, Header, Method, StatusCode};

fn serve_error(request: Request, message: &str, status_code: StatusCode, log: bool, error: &str) -> io::Result<()> {
    let content_type_header = Header::from_bytes("Content-Type", "application/json")
        .expect("That we didn't put any garbage in the headers");

    if log {
        eprintln!("ERROR: {error}");
    }

    let error = json!({
        "message": message,
    });

    request.respond(Response::from_string(serde_json::to_string(&error).unwrap())
        .with_status_code(status_code)
        .with_header(content_type_header)
    )
}

fn serve_404(request: Request) -> io::Result<()> {
    serve_error(request, "ERROR: Route not found", StatusCode(404), false, "")
}

fn serve_500(request: Request, message: &str, error: &str) -> io::Result<()> {
    serve_error(request, message, StatusCode(500), true, error)
}

fn serve_400(request: Request, message: &str, error: &str) -> io::Result<()> {
    serve_error(request, message, StatusCode(400), true, error)
}

fn serve_bytes(request: Request, bytes: &[u8], content_type: &str) -> io::Result<()> {
    let content_type_header = Header::from_bytes("Content-Type", content_type)
        .expect("That we didn't put any garbage in the headers");
    request.respond(Response::from_data(bytes).with_header(content_type_header))
}

fn serve_api_search(model: Arc<Mutex<Model>>, mut request: Request, content_type_header: Header) -> io::Result<()> {
    let mut buf = Vec::new();
    if let Err(err) = request.as_reader().read_to_end(&mut buf) {
        return serve_500(request, "Could not read the body of the request", &err.to_string());
    }

    let body = match str::from_utf8(&buf) {
        Ok(body) => body.chars().collect::<Vec<_>>(),
        Err(err) => {
            return serve_400(request, "Body must be a valid UTF-8 string", &err.to_string());
        }
    };

    let model = model.lock().unwrap();
    let result = model.search_query(&body);

    match serde_json::to_string(&result.iter().take(20).collect::<Vec<_>>()) {
        Ok(json) => {
            request.respond(Response::from_string(&json).with_header(content_type_header))
        },
        Err(err) => {
            serve_500(request, "Error happened serving api search", &err.to_string())
        }
    }
}

fn serve_api_stats(model: Arc<Mutex<Model>>, request: Request, content_type_header: Header) -> io::Result<()> {
    use serde::Serialize;

    #[derive(Default, Serialize)]
    struct Stats {
        docs_count: usize,
        terms_count: usize,
    }

    let mut stats: Stats = Default::default();
    {
        let model = model.lock().unwrap();
        stats.docs_count = model.docs.len();
        stats.terms_count = model.df.len();
    }

    match serde_json::to_string(&stats) {
        Ok(json) => {
            return request.respond(Response::from_string(&json).with_header(content_type_header));
        }
        Err(err) => {
            return serve_500(request, "Error happened serving api stats", &err.to_string());
        }
    };
}

fn serve_request(model: Arc<Mutex<Model>>, request: Request) -> io::Result<()> {
    let json_content_type = Header::from_bytes("Content-Type", "application/json")
        .expect("That we didn't put any garbage in the headers");

    println!("received request! method: {:?}, url: {:?}", request.method(), request.url());

    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            serve_api_search(model, request, json_content_type)
        }
        (Method::Get, "/api/stats") => {
            serve_api_stats(model, request, json_content_type)
        }
        (Method::Get, "/index.css") => {
            serve_bytes(request, include_bytes!("../public/index.css"), "text/css")
        }
        (Method::Get, "/favicon.ico") => {
            serve_bytes(request, include_bytes!("../public/favicon.ico"), "image/x-icon")
        }
        (Method::Get, "/index.js") => {
            serve_bytes(request, include_bytes!("../public/index.js"), "text/javascript; charset=utf-8")
        }
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_bytes(request, include_bytes!("../public/index.html"), "text/html; charset=utf-8")
        }
        _ => {
            serve_404(request)
        }
    }
}

pub fn start(address: &str, model: Arc<Mutex<Model>>) -> Result<(), ()> {
    let server = Server::http(&address).map_err(|err| {
        eprintln!("ERROR: could not start HTTP server at {address}: {err}");
    })?;

    println!("listening at http://{address}/");

    for request in server.incoming_requests() {
        serve_request(Arc::clone(&model), request).map_err(|err| {
            eprintln!("ERROR: could not serve the response: {err}");
        }).ok(); // <- don't stop on errors, keep serving
    }

    eprintln!("ERROR: the server socket has shutdown");
    Err(())
}
