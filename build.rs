use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Compile protobuf files
    tonic_build::configure()
        .build_server(false) // We only need the client
        .build_client(true)
        .out_dir(&out_dir)
        .compile(
            &["proto/pumpswap.proto"],
            &["proto/"],
        )?;
    
    // Recompile if proto files change
    println!("cargo:rerun-if-changed=proto/pumpswap.proto");
    
    Ok(())
}
