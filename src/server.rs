use std::io;

use form_urlencoded::parse;
use rand::Rng;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use diceroll::run;

pub fn serve<R: Rng>(rng: &mut R, port: u16) -> io::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(addr).map_err(|err| io::Error::other(err.to_string()))?;
    for request in server.incoming_requests() {
        handle_request(request, rng);
    }
    Ok(())
}

fn handle_request<R: Rng>(mut request: Request, rng: &mut R) {
    let response = build_response(&mut request, rng);
    let _ = request.respond(response);
}

fn build_response<R: Rng>(
    request: &mut Request,
    rng: &mut R,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let wants_json = wants_json(request);
    let (path, query) = split_path_and_query(request.url());

    if path != "/roll" {
        return not_found(wants_json);
    }

    let expr = match *request.method() {
        Method::Get => {
            let Some(expr) = query_param(query, "q") else {
                return bad_request(wants_json, "missing query parameter q");
            };
            expr
        }
        Method::Post => {
            let mut expr = String::new();
            if let Err(err) = std::io::Read::read_to_string(request.as_reader(), &mut expr) {
                return bad_request(wants_json, &format!("unable to read request body: {err}"));
            }
            expr
        }
        _ => {
            let mut response = response_with_body(405, wants_json, "method not allowed");
            if let Some(header) = allow_header() {
                response = response.with_header(header);
            }
            return response;
        }
    };

    match run(expr.trim(), rng) {
        Ok(result) => {
            if wants_json {
                response_with_body(200, true, &result.json())
            } else {
                response_with_body(200, false, &result.formatted(false, false))
            }
        }
        Err(err) => bad_request(wants_json, &format!("parse error: {err}")),
    }
}

fn response_with_body(
    status: u16,
    wants_json: bool,
    body: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut response = Response::from_string(body.to_owned()).with_status_code(StatusCode(status));
    let content_type = if wants_json {
        "application/json; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    };
    if let Some(header) = header("Content-Type", content_type) {
        response = response.with_header(header);
    }
    response
}

fn bad_request(wants_json: bool, message: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    if wants_json {
        response_with_body(
            400,
            true,
            &serde_json::json!({ "error": message }).to_string(),
        )
    } else {
        response_with_body(400, false, message)
    }
}

fn not_found(wants_json: bool) -> Response<std::io::Cursor<Vec<u8>>> {
    response_with_body(404, wants_json, "not found")
}

fn allow_header() -> Option<Header> {
    header("Allow", "GET, POST")
}

fn header(name: &str, value: &str) -> Option<Header> {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).ok()
}

fn wants_json(request: &Request) -> bool {
    request.headers().iter().any(|header| {
        header.field.equiv("Accept") && header.value.as_str().contains("application/json")
    })
}

fn query_param(url: &str, key: &str) -> Option<String> {
    parse(url.as_bytes())
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn split_path_and_query(url: &str) -> (&str, &str) {
    url.split_once('?').unwrap_or((url, ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use tiny_http::{Header, Method, TestRequest};

    fn req(method: Method, path: &str, accept: Option<&str>) -> Request {
        let mut request = TestRequest::new().with_method(method).with_path(path);
        if let Some(accept) = accept {
            request = request.with_header(Header::from_bytes("Accept", accept).unwrap());
        }
        Request::from(request)
    }

    #[test]
    fn get_roll_plain_text() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut request = req(Method::Get, "/roll?q=2d6", None);
        let response = build_response(&mut request, &mut rng);

        let mut expected_rng = StdRng::seed_from_u64(7);
        let expected = run("2d6", &mut expected_rng)
            .unwrap()
            .formatted(false, false);

        assert_eq!(response.status_code(), StatusCode(200));
        assert_eq!(
            response
                .headers()
                .iter()
                .find(|h| h.field.equiv("Content-Type"))
                .map(|h| h.value.as_str()),
            Some("text/plain; charset=utf-8")
        );
        assert_eq!(collect_body(response), expected);
    }

    #[test]
    fn get_roll_json() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut request = req(Method::Get, "/roll?q=2d6", Some("application/json"));
        let response = build_response(&mut request, &mut rng);

        let mut expected_rng = StdRng::seed_from_u64(7);
        let expected = run("2d6", &mut expected_rng).unwrap().json();

        assert_eq!(response.status_code(), StatusCode(200));
        assert_eq!(
            response
                .headers()
                .iter()
                .find(|h| h.field.equiv("Content-Type"))
                .map(|h| h.value.as_str()),
            Some("application/json; charset=utf-8")
        );
        assert_eq!(collect_body(response), expected);
    }

    #[test]
    fn missing_query_param_is_bad_request() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut request = req(Method::Get, "/roll", None);
        let response = build_response(&mut request, &mut rng);
        assert_eq!(response.status_code(), StatusCode(400));
        assert_eq!(collect_body(response), "missing query parameter q");
    }

    #[test]
    fn unknown_path_is_not_found() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut request = req(Method::Get, "/bogus?q=2d6", None);
        let response = build_response(&mut request, &mut rng);
        assert_eq!(response.status_code(), StatusCode(404));
        assert_eq!(collect_body(response), "not found");
    }

    #[test]
    fn invalid_expression_is_bad_request() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut request = req(Method::Get, "/roll?q=foo", Some("application/json"));
        let response = build_response(&mut request, &mut rng);
        assert_eq!(response.status_code(), StatusCode(400));
        assert!(collect_body(response).contains(r#""error":"parse error:"#));
    }

    #[test]
    fn non_get_is_method_not_allowed() {
        let mut rng = StdRng::seed_from_u64(0);
        let mut request = req(Method::Put, "/roll?q=2d6", None);
        let response = build_response(&mut request, &mut rng);
        let allow = response
            .headers()
            .iter()
            .find(|h| h.field.equiv("Allow"))
            .map(|h| h.value.as_str().to_owned());
        assert_eq!(response.status_code(), StatusCode(405));
        assert_eq!(collect_body(response), "method not allowed");
        assert_eq!(allow.as_deref(), Some("GET, POST"));
    }

    #[test]
    fn post_roll_plain_text() {
        let mut rng = StdRng::seed_from_u64(7);
        let request = TestRequest::new()
            .with_method(Method::Post)
            .with_path("/roll")
            .with_body("2d6+3");
        let mut request: Request = request.into();
        let response = build_response(&mut request, &mut rng);

        let mut expected_rng = StdRng::seed_from_u64(7);
        let expected = run("2d6+3", &mut expected_rng)
            .unwrap()
            .formatted(false, false);

        assert_eq!(response.status_code(), StatusCode(200));
        assert_eq!(collect_body(response), expected);
    }

    #[test]
    fn post_roll_json() {
        let mut rng = StdRng::seed_from_u64(7);
        let request = TestRequest::new()
            .with_method(Method::Post)
            .with_path("/roll")
            .with_body("2d6+3")
            .with_header(Header::from_bytes("Accept", "application/json").unwrap());
        let mut request: Request = request.into();
        let response = build_response(&mut request, &mut rng);

        let mut expected_rng = StdRng::seed_from_u64(7);
        let expected = run("2d6+3", &mut expected_rng).unwrap().json();

        assert_eq!(response.status_code(), StatusCode(200));
        assert_eq!(collect_body(response), expected);
    }

    #[test]
    fn query_decoding_supports_plus_and_percent() {
        assert_eq!(query_param("q=2d6%2B3", "q").as_deref(), Some("2d6+3"));
        assert_eq!(query_param("q=2d6+3", "q").as_deref(), Some("2d6 3"));
    }

    fn collect_body(response: Response<std::io::Cursor<Vec<u8>>>) -> String {
        use std::io::Read;

        let mut reader = response.into_reader();
        let mut body = Vec::new();
        reader.read_to_end(&mut body).unwrap();
        String::from_utf8(body).unwrap()
    }
}
