use std::{collections::HashMap, fmt::{format, Formatter, Debug}, pin::Pin};

use crate::{environment::EnvironmentRef, thread_pool::ThreadPoolRef};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wry::{
    application::{
        event_loop::{ControlFlow, EventLoopProxy},
        window::Window,
    },
    webview::WebView,
};

#[derive(Deserialize, Clone)]
pub struct ApiArgs(Value);

impl ApiArgs {
    pub fn get_single<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_value::<(T,)>(self.0.clone())?.0)
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_value(self.0.clone())?)
    }

    pub fn get_with_optional<T: serde::de::DeserializeOwned>(&self, args_size: usize) -> Result<T> {
        let mut args = serde_json::from_value::<Vec<serde_json::Value>>(self.0.clone())?;
        args.resize(args_size, json!(null));
        let args = json!(args);
        Ok(serde_json::from_value(args)?)
    }
}

#[derive(Deserialize, Clone)]
pub struct ApiRequest(pub u32, pub String, pub ApiArgs);

impl ApiRequest {
    pub fn err<C: Into<i32>, S: Into<String>>(code: C, msg: S) -> ApiResponse {
        ApiResponse(code.into(), msg.into(), json!(null))
    }

    pub fn ok<D: Serialize>(data: D) -> ApiResponse {
        ApiResponse(0, "ok".to_string(), json!(data))
    }
}

#[derive(Serialize, Clone)]
pub struct ApiResponse(pub i32, pub String, pub Value);

pub struct ApiEventCallback(Pin<Box<dyn Fn(&WebView, &mut ControlFlow)>>);

impl Debug for ApiEventCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiEventCallback").finish()
    }
}

pub type ApiInstance = Pin<Box<dyn Fn(EnvironmentRef, &Window, ApiRequest)>>;

pub struct ApiManager {
    env: EnvironmentRef,
    task_pool: ThreadPoolRef,
    event_loop: EventLoopProxy<ApiEventCallback>,
    apis: HashMap<String, (bool, ApiInstance)>,
}

impl ApiManager {
    pub fn new(
        env: EnvironmentRef,
        task_pool: ThreadPoolRef,
        event_loop: EventLoopProxy<ApiEventCallback>,
    ) -> Self {
        Self {
            env,
            task_pool,
            event_loop,
            apis: std::collections::HashMap::new(),
        }
    }

    pub fn register_api<'t, S: Into<String>, T: Serialize + 'static>(
        &mut self,
        name: S,
        api_func: fn(EnvironmentRef, &Window, ApiRequest) -> Result<T>,
    ) {
        let event_loop = self.event_loop.clone();
        let api_instance: ApiInstance = Box::pin(move |env, win, request| {
            let result = api_func(env, win, request);
            let response = match result {
                Ok(data) => ApiRequest::ok(data),
                Err(err) => ApiRequest::err(-1, err.to_string()),
            };
            event_loop
                .send_event(ApiEventCallback(Box::pin(move |webview, &mut _| {
                    Self::send_ipc_callback(webview, response.clone());
                })))
                .unwrap();
        });
        self.apis.insert(name.into(), (false, api_instance));
    }

    // pub fn register_api<>(&mut self, name: &str, api: ApiInstance) {
    //     self.apis.insert(name.to_string(), api);
    // }

    fn send_ipc_callback(webview: &WebView, response: ApiResponse) -> Result<()> {
        let data_str = serde_json::to_string(&response)?;
        Ok(
            webview
                .evaluate_script(&format!("TauriLite.__emit__(\"ipc.callback\", {data_str})"))?,
        )
    }
}