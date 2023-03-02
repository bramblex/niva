
use std::env;
use std::fs;
use std::path::Path;

fn main() {
	let out_dir = env::var("OUT_DIR").unwrap();
	let dest_path = Path::new(&out_dir).join("preload.rs");
	let preload_path = env::current_dir().unwrap().join("src/preload.js");
	let preload_content = fs::read_to_string(&preload_path).unwrap();

	let mut content: String = String::new(); 
	content += "const PRELOAD_JS: &'static str = \""; 
	content += &preload_content.escape_default().to_string();
	content += "\";";

	fs::write(dest_path, content).unwrap();
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=src/preload.js");
}