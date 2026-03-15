pub mod nexode {
    pub mod hypervisor {
        pub mod v2 {
            tonic::include_proto!("nexode.hypervisor.v2");
        }
    }
}

pub use nexode::hypervisor::v2::*;
