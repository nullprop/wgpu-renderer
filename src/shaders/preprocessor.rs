use wgpu;
use regex::Regex;

use crate::core::resources::load_string;

pub fn preprocess_wgsl(filename: &str) -> wgpu::ShaderSource {
    let source_path = "shaders/".to_owned() + filename;
    println!("preprocess_wgsl: loading source {}", source_path);
    let mut source = load_string(&source_path);

    let re = Regex::new(r"#include (.*?)\n").unwrap();
    for cap in re.captures_iter(&source.clone()) {
        let whole_match = &cap[0];
        let mut full_path: String = source_path.to_owned();
        full_path = full_path.replace(filename, &cap[1]);

        println!("preprocess_wgsl: replacing {} with file {}", whole_match, full_path);
        let nested_source = load_string(&full_path);
        source = source.replace(whole_match, &nested_source);
    }

    wgpu::ShaderSource::Wgsl(source.into())
}
