fn main() -> Result<(), tonic_buf_build::error::TonicBufBuildError> {
    let config = tonic_buf_build::TonicBufConfig {
        buf_dir: Some("proto"),
    };
    tonic_buf_build::compile_from_buf_with_config(
        tonic_build::configure().build_client(false),
        None,
        config,
    )?;
    Ok(())
}
