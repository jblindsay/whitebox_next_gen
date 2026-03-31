use std::path::PathBuf;

fn usage() -> &'static str {
    "Usage: cargo run -p wbw_r --example generate_r_wrappers -- [--include-pro] [--tier open|pro|enterprise] [--output path]"
}

fn main() {
    let mut include_pro = false;
    let mut tier = String::from("open");
    let mut output = PathBuf::from("crates/wbw_r/generated/wbw_tools_generated.R");

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--include-pro" => include_pro = true,
            "--tier" => {
                let Some(v) = args.next() else {
                    eprintln!("{}", usage());
                    std::process::exit(2);
                };
                tier = v;
            }
            "--output" => {
                let Some(v) = args.next() else {
                    eprintln!("{}", usage());
                    std::process::exit(2);
                };
                output = PathBuf::from(v);
            }
            "-h" | "--help" => {
                println!("{}", usage());
                return;
            }
            _ => {
                eprintln!("Unknown argument: {arg}");
                eprintln!("{}", usage());
                std::process::exit(2);
            }
        }
    }

    let module_txt = match wbw_r::generate_r_wrapper_module_with_options(include_pro, &tier) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to generate wrappers: {e}");
            std::process::exit(1);
        }
    };

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Failed to create output directory '{}': {e}", parent.display());
                std::process::exit(1);
            }
        }
    }

    if let Err(e) = std::fs::write(&output, module_txt) {
        eprintln!("Failed to write output '{}': {e}", output.display());
        std::process::exit(1);
    }

    println!("Wrote generated wrappers to {}", output.display());
}
