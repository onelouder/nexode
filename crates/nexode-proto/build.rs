fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    // tonic/prost still discover protoc through the environment.
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    tonic_prost_build::configure().compile_protos(&["proto/hypervisor.proto"], &["proto"])?;

    Ok(())
}
