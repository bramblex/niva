use winit::application::ApplicationHandler;



#[derive(Default)]
pub struct App {
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
    }
}



impl ApplicationHandler for App {
}