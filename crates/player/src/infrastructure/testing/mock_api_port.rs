use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{de::DeserializeOwned, Serialize};

use crate::ports::outbound::{ApiError, ApiPort};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Method {
    Get,
    GetOptional,
    Post,
    PostNoResponse,
    PostEmpty,
    Put,
    PutNoResponse,
    PutEmpty,
    PutEmptyWithResponse,
    Patch,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    method: Method,
    path: String,
}

#[derive(Debug, Clone)]
pub struct RequestRecord {
    pub method: &'static str,
    pub path: String,
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
enum Response {
    Json(serde_json::Value),
    Ok,
    NotFound,
    Err(ApiError),
}

#[derive(Default)]
struct State {
    responses: HashMap<Key, Response>,
    requests: Vec<RequestRecord>,
}

/// Mock implementation of `ApiPort` for unit tests.
///
/// Stores a map of pre-programmed responses keyed by method + path,
/// and records all requests made for assertion.
#[derive(Clone, Default)]
pub struct MockApiPort {
    state: Arc<Mutex<State>>,
}

impl MockApiPort {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn requests(&self) -> Vec<RequestRecord> {
        self.state.lock().unwrap().requests.clone()
    }

    fn when_json(&self, method: Method, path: &str, json: serde_json::Value) {
        let mut state = self.state.lock().unwrap();
        state.responses.insert(
            Key {
                method,
                path: path.to_string(),
            },
            Response::Json(json),
        );
    }

    fn when_ok(&self, method: Method, path: &str) {
        let mut state = self.state.lock().unwrap();
        state.responses.insert(
            Key {
                method,
                path: path.to_string(),
            },
            Response::Ok,
        );
    }

    fn when_not_found(&self, method: Method, path: &str) {
        let mut state = self.state.lock().unwrap();
        state.responses.insert(
            Key {
                method,
                path: path.to_string(),
            },
            Response::NotFound,
        );
    }

    fn when_err(&self, method: Method, path: &str, err: ApiError) {
        let mut state = self.state.lock().unwrap();
        state.responses.insert(
            Key {
                method,
                path: path.to_string(),
            },
            Response::Err(err),
        );
    }

    fn record(&self, method: &'static str, path: &str, body: Option<serde_json::Value>) {
        let mut state = self.state.lock().unwrap();
        state.requests.push(RequestRecord {
            method,
            path: path.to_string(),
            body,
        });
    }

    fn take_response(&self, key: Key) -> Result<Response, ApiError> {
        let mut state = self.state.lock().unwrap();
        match state.responses.remove(&key) {
            Some(resp) => Ok(resp),
            None => Err(ApiError::RequestFailed(format!(
                "No mock response configured for {:?} {}",
                key.method, key.path
            ))),
        }
    }

    fn decode<T: DeserializeOwned>(resp: Response) -> Result<T, ApiError> {
        match resp {
            Response::Json(v) => {
                serde_json::from_value::<T>(v).map_err(|e| ApiError::ParseError(e.to_string()))
            }
            Response::Err(e) => Err(e),
            Response::NotFound => Err(ApiError::HttpError(404, "Not found".to_string())),
            Response::Ok => Err(ApiError::ParseError("Expected JSON response".to_string())),
        }
    }
}

// Public helper API for configuring responses (no need to expose internal enums).
impl MockApiPort {
    pub fn when_get_json(&self, path: &str, json: serde_json::Value) {
        self.when_json(Method::Get, path, json);
    }

    pub fn when_get_err(&self, path: &str, err: ApiError) {
        self.when_err(Method::Get, path, err);
    }

    pub fn when_get_optional_json(&self, path: &str, json: serde_json::Value) {
        self.when_json(Method::GetOptional, path, json);
    }

    pub fn when_get_optional_not_found(&self, path: &str) {
        self.when_not_found(Method::GetOptional, path);
    }

    pub fn when_get_optional_err(&self, path: &str, err: ApiError) {
        self.when_err(Method::GetOptional, path, err);
    }

    pub fn when_post_json(&self, path: &str, json: serde_json::Value) {
        self.when_json(Method::Post, path, json);
    }

    pub fn when_post_err(&self, path: &str, err: ApiError) {
        self.when_err(Method::Post, path, err);
    }

    pub fn when_post_no_response_ok(&self, path: &str) {
        self.when_ok(Method::PostNoResponse, path);
    }

    pub fn when_post_empty_ok(&self, path: &str) {
        self.when_ok(Method::PostEmpty, path);
    }

    pub fn when_put_json(&self, path: &str, json: serde_json::Value) {
        self.when_json(Method::Put, path, json);
    }

    pub fn when_put_err(&self, path: &str, err: ApiError) {
        self.when_err(Method::Put, path, err);
    }

    pub fn when_put_no_response_ok(&self, path: &str) {
        self.when_ok(Method::PutNoResponse, path);
    }

    pub fn when_put_empty_ok(&self, path: &str) {
        self.when_ok(Method::PutEmpty, path);
    }

    pub fn when_put_empty_with_response_json(&self, path: &str, json: serde_json::Value) {
        self.when_json(Method::PutEmptyWithResponse, path, json);
    }

    pub fn when_delete_ok(&self, path: &str) {
        self.when_ok(Method::Delete, path);
    }

    pub fn when_delete_err(&self, path: &str, err: ApiError) {
        self.when_err(Method::Delete, path, err);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl ApiPort for MockApiPort {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.record("GET", path, None);
        let resp = self.take_response(Key {
            method: Method::Get,
            path: path.to_string(),
        })?;
        Self::decode(resp)
    }

    async fn get_optional<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, ApiError> {
        self.record("GET_OPTIONAL", path, None);
        let resp = self.take_response(Key {
            method: Method::GetOptional,
            path: path.to_string(),
        })?;
        match resp {
            Response::NotFound => Ok(None),
            other => Self::decode(other).map(Some),
        }
    }

    async fn post<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let json_body = serde_json::to_value(body).ok();
        self.record("POST", path, json_body);
        let resp = self.take_response(Key {
            method: Method::Post,
            path: path.to_string(),
        })?;
        Self::decode(resp)
    }

    async fn post_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let json_body = serde_json::to_value(body).ok();
        self.record("POST_NO_RESPONSE", path, json_body);
        let resp = self.take_response(Key {
            method: Method::PostNoResponse,
            path: path.to_string(),
        })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Err(e) => Err(e),
            Response::NotFound => Err(ApiError::HttpError(404, "Not found".to_string())),
            Response::Json(_) => Err(ApiError::ParseError("Expected unit response".to_string())),
        }
    }

    async fn post_empty(&self, path: &str) -> Result<(), ApiError> {
        self.record("POST_EMPTY", path, None);
        let resp = self.take_response(Key {
            method: Method::PostEmpty,
            path: path.to_string(),
        })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Err(e) => Err(e),
            Response::NotFound => Err(ApiError::HttpError(404, "Not found".to_string())),
            Response::Json(_) => Err(ApiError::ParseError("Expected unit response".to_string())),
        }
    }

    async fn put<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let json_body = serde_json::to_value(body).ok();
        self.record("PUT", path, json_body);
        let resp = self.take_response(Key {
            method: Method::Put,
            path: path.to_string(),
        })?;
        Self::decode(resp)
    }

    async fn put_no_response<B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let json_body = serde_json::to_value(body).ok();
        self.record("PUT_NO_RESPONSE", path, json_body);
        let resp = self.take_response(Key {
            method: Method::PutNoResponse,
            path: path.to_string(),
        })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Err(e) => Err(e),
            Response::NotFound => Err(ApiError::HttpError(404, "Not found".to_string())),
            Response::Json(_) => Err(ApiError::ParseError("Expected unit response".to_string())),
        }
    }

    async fn put_empty(&self, path: &str) -> Result<(), ApiError> {
        self.record("PUT_EMPTY", path, None);
        let resp = self.take_response(Key {
            method: Method::PutEmpty,
            path: path.to_string(),
        })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Err(e) => Err(e),
            Response::NotFound => Err(ApiError::HttpError(404, "Not found".to_string())),
            Response::Json(_) => Err(ApiError::ParseError("Expected unit response".to_string())),
        }
    }

    async fn put_empty_with_response<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, ApiError> {
        self.record("PUT_EMPTY_WITH_RESPONSE", path, None);
        let resp = self.take_response(Key {
            method: Method::PutEmptyWithResponse,
            path: path.to_string(),
        })?;
        Self::decode(resp)
    }

    async fn patch<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let json_body = serde_json::to_value(body).ok();
        self.record("PATCH", path, json_body);
        let resp = self.take_response(Key {
            method: Method::Patch,
            path: path.to_string(),
        })?;
        Self::decode(resp)
    }

    async fn delete(&self, path: &str) -> Result<(), ApiError> {
        self.record("DELETE", path, None);
        let resp = self.take_response(Key {
            method: Method::Delete,
            path: path.to_string(),
        })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Err(e) => Err(e),
            Response::NotFound => Err(ApiError::HttpError(404, "Not found".to_string())),
            Response::Json(_) => Err(ApiError::ParseError("Expected unit response".to_string())),
        }
    }
}
