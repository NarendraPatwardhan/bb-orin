//! CLI entry for bindgen 0.72.1. Same flags as bindgen-cli.
use std::env;

use bindgen::builder_from_flags;

fn main() {
    match builder_from_flags(env::args()) {
        Ok((builder, output, verbose)) => {
            std::panic::set_hook(Box::new(move |info| {
                if verbose {
                    eprintln!("Bindgen unexpectedly panicked");
                }
                eprintln!("{info}");
            }));

            let bindings = match builder.generate() {
                Ok(bindings) => bindings,
                Err(err) => {
                    eprintln!("Unable to generate bindings: {err}");
                    std::process::exit(1);
                }
            };

            let _ = std::panic::take_hook();
            bindings.write(output).expect("Unable to write output");
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
