use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: orin <model.gguf>");
        return ExitCode::from(2);
    };

    let backend = match llama_cpp_2::llama_backend::LlamaBackend::init() {
        Ok(backend) => backend,
        Err(err) => {
            eprintln!("llama backend init failed: {err:?}");
            return ExitCode::from(1);
        }
    };
    println!("supports_gpu_offload={}", backend.supports_gpu_offload());

    let params =
        llama_cpp_2::model::params::LlamaModelParams::default().with_n_gpu_layers(u32::MAX);
    match llama_cpp_2::model::LlamaModel::load_from_file(&backend, &path, &params) {
        Ok(_) => {
            println!("loaded {path}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("load failed: {err:?}");
            ExitCode::from(1);
        }
    }
}
