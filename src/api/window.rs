pub fn call(request: ApiRequest) -> ApiResponse {
    return match request.method.as_str() {
        _ => ApiResponse::err("Method not found".to_string()),
    };
}
