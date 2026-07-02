fn main() {
    println!("cargo:rerun-if-changed=proto/querypie.proto");
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("find vendored protoc");
    std::env::set_var("PROTOC", protoc);
    prost_build::Config::new()
        .compile_protos(&["proto/querypie.proto"], &["proto"])
        .expect("compile querypie protobuf schema");
}
