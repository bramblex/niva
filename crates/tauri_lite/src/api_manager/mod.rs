use std::{collections::HashMap, pin::Pin};

use crate::{
    environment::EnvironmentRef,
    thread_pool::ThreadPoolRef, event_loop::{MainEventLoopProxy, CallbackEvent},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wry::{
    application::{event_loop::ControlFlow, window::Window},
    webview::WebView
};

#[derive(Deserialize, Clone)]
pub struct ApiArgs(Value);

impl ApiArgs {
    pub fn get_single<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_value::<(T,)>(self.0.clone())?.0)
    }

    #[allow(dead_code)]
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
    pub fn err<C: Into<i32>, S: Into<String>>(&self, code: C, msg: S) -> ApiResponse {
        ApiResponse(self.0, code.into(), msg.into(), json!(null))
    }

    pub fn ok<D: Serialize>(&self, data: D) -> ApiResponse {
        ApiResponse(self.0, 0, "ok".to_string(), json!(data))
    }

    pub fn args (&self) -> &ApiArgs {
        &self.2
    }
}

#[derive(Serialize, Clone)]
pub struct ApiResponse(pub u32, pub i32, pub String, pub Value);

pub type ApiInstance = Pin<Box<dyn Fn(&Window, ApiRequest)>>;

pub struct ApiManager {
    env: EnvironmentRef,
    task_pool: ThreadPoolRef,
    event_loop: MainEventLoopProxy,
    apis: HashMap<String, ApiInstance>,
}

impl ApiManager {
    pub fn new(
        env: EnvironmentRef,
        task_pool: ThreadPoolRef,
        event_loop: MainEventLoopProxy,
    ) -> Self {
        Self {
            env,
            task_pool,
            event_loop,
            apis: std::collections::HashMap::new(),
        }
    }

    pub fn register_async_api<S: Into<String>, T: Serialize + 'static>(
        &mut self,
        name: S,
        api_func: fn(EnvironmentRef, ApiRequest) -> Result<T>,
    ) {
        let event_loop = self.event_loop.clone();
        let task_pool = self.task_pool.clone();
        let env = self.env.clone();
        let api_instance: ApiInstance = Box::pin(move |_, request| {
            let event_loop = event_loop.clone();
            let env = env.clone();
            task_pool.lock().unwrap().run(move || {
                let result = api_func(env, request.clone());
                let response = match result {
                    Ok(data) => request.ok(data),
                    Err(err) => request.err(-1, err.to_string()),
                };
                event_loop.ipc_send_callback(response).unwrap();
            });
        });
        self.apis.insert(name.into(), api_instance);
    }

    pub fn register_api<S: Into<String>, T: Serialize + 'static>(
        &mut self,
        name: S,
        api_func: fn(EnvironmentRef, &Window, ApiRequest) -> Result<T>,
    ) {
        let event_loop = self.event_loop.clone();
        let env = self.env.clone();
        let api_instance: ApiInstance = Box::pin(move |win, request| {
            let result = api_func(env.clone(), win, request.clone());
            let response = match result {
                Ok(data) => request.ok(data),
                Err(err) => request.err(-1, err.to_string()),
            };
            event_loop.ipc_send_callback(response).unwrap();
        });
        self.apis.insert(name.into(), api_instance);
    }

    pub fn register_event_api<S: Into<String>, T: Serialize + 'static>(
        &mut self,
        name: S,
        api_func: fn(
            EnvironmentRef,
            &WebView,
            ApiRequest,
            &mut ControlFlow,
        ) -> Result<T>,
    ) {
        let event_loop = self.event_loop.clone();
        let env = self.env.clone();
        let api_instance: ApiInstance = Box::pin(move |_, request| {
            let env = env.clone();
            let request = request;
            event_loop
                .send_event(CallbackEvent(Box::pin(move |webview, _, control_flow| {
                    let result = api_func(env.clone(), webview, request.clone(), control_flow);
                    match result {
                        Ok(data) => {
                            let response = request.ok(data);
                            Self::_send_ipc_callback(webview, response).unwrap();
                        }
                        Err(err) => {
                            let response = request.err(-1, err.to_string());
                            Self::_send_ipc_callback(webview, response).unwrap();
                        }
                    }
                })))
                .unwrap();
        });
        self.apis.insert(name.into(), api_instance);
    }

    pub fn call_api(&self, win: &Window, request_str: String) {
        let request = serde_json::from_str::<ApiRequest>(&request_str);
        if let Err(err) = &request {
            self.event_loop
                .ipc_send_event("ipc.error", json!(err.to_string()))
                .unwrap();
            return;
        }
        let request = request.unwrap();

        match self.apis.get(&request.1) {
            Some(api) => api(win, request),
            None => {
                let response = request.err(-1, format!("api {} not found", request.1));
                self.event_loop.ipc_send_callback(response).unwrap();
            }
        }
    }

    fn _send_ipc_callback(webview: &WebView, response: ApiResponse) -> Result<()> {
        let data_str = serde_json::to_string(&response)?;
        Ok(
            webview
                .evaluate_script(&format!("TauriLite.__emit__(\"ipc.callback\", {data_str})"))?,
        )
    }
}
