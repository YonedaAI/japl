// JAPL HTTP Adapter
// Bridges wasi:http/incoming-handler to JAPL's __handle_http flat function
//
// wasmCloud/wasmtime calls: handle(incoming-request, response-outparam)
// This adapter reads method/path/body, calls __handle_http, returns response

wit_bindgen::generate!({
    path: "wit",
    world: "adapter",
    generate_all,
});

struct JaplHttpAdapter;

export!(JaplHttpAdapter);

impl exports::wasi::http::incoming_handler::Guest for JaplHttpAdapter {
    fn handle(request: wasi::http::types::IncomingRequest, outparam: wasi::http::types::ResponseOutparam) {
        use wasi::http::types::*;

        // Extract method
        let method_val = request.method();
        let method = match &method_val {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Patch => "PATCH",
            Method::Connect => "CONNECT",
            Method::Trace => "TRACE",
            Method::Other(s) => s.as_str(),
        };

        // Extract path
        let path = request.path_with_query().unwrap_or_else(|| "/".to_string());

        // Extract body
        let body = match request.consume() {
            Ok(incoming_body) => {
                match incoming_body.stream() {
                    Ok(stream) => {
                        let mut body_bytes = Vec::new();
                        loop {
                            match stream.read(64 * 1024) {
                                Ok(chunk) => {
                                    if chunk.is_empty() {
                                        break;
                                    }
                                    body_bytes.extend_from_slice(&chunk);
                                }
                                Err(_) => break,
                            }
                        }
                        drop(stream);
                        let _trailers = IncomingBody::finish(incoming_body);
                        String::from_utf8(body_bytes).unwrap_or_default()
                    }
                    Err(_) => String::new(),
                }
            }
            Err(_) => String::new(),
        };

        // Call the JAPL app handler
        let response_body = japl::app::handler::handle_http(method, &path, &body);

        // Build HTTP response
        let headers = Fields::new();
        let _ = headers.append(
            &"content-type".to_string(),
            &b"text/plain; charset=utf-8".to_vec(),
        );
        let _ = headers.append(
            &"content-length".to_string(),
            &response_body.len().to_string().into_bytes(),
        );

        let response = OutgoingResponse::new(headers);
        response.set_status_code(200).unwrap();

        let out_body = response.body().unwrap();
        ResponseOutparam::set(outparam, Ok(response));

        let stream = out_body.write().unwrap();
        stream.blocking_write_and_flush(response_body.as_bytes()).unwrap();
        drop(stream);
        OutgoingBody::finish(out_body, None).unwrap();
    }
}
