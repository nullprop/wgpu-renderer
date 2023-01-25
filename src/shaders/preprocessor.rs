use wgpu;
use regex::Regex;
use std::fs::read_to_string;

pub fn preprocess_wgsl(filename: &str) -> wgpu::ShaderSource {
    let source_path = env!("CARGO_MANIFEST_DIR").to_owned() + "/src/shaders/wgsl/" + filename;
    println!("preprocess_wgsl: loading source {}", source_path);
    let mut source =
        read_to_string(&source_path)
            .unwrap();

    let re = Regex::new(r"#include (.*?)\n").unwrap();
    for cap in re.captures_iter(&source.clone()) {
        let whole_match = &cap[0];
        let mut full_path: String = source_path.to_owned();
        full_path = full_path.replace(filename, &cap[1]);

        println!("preprocess_wgsl: replacing {} with file {}", whole_match, full_path);
        let nested_source = read_to_string(full_path).unwrap();
        source = source.replace(whole_match, &nested_source);
    }

    return wgpu::ShaderSource::Wgsl(source.into());
}
