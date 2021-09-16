fn main() {
    prost_build::compile_protos(&["src/peer_id.proto"],&["src"]).unwrap();
}