pub mod grpcweb;

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/querypie.rs"));
}

pub use grpcweb::{Client, GrpcError};
