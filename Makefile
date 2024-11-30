aarch64_linux:
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc cargo build --release --target aarch64-unknown-linux-gnu

native:
	cargo build --release

transfer:
	multipass transfer -r ../evprofiler focal:apps/evprofiler  

clean: 
	rm -rf target .cargo
