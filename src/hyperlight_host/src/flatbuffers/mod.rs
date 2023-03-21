#[allow(dead_code, unused_imports, clippy::all)]
pub mod hyperlight {
    use super::*;
    pub mod generated {
        use super::*;
        mod parameter_value_generated;
        pub use self::parameter_value_generated::*;
        mod parameter_type_generated;
        pub use self::parameter_type_generated::*;
        mod return_type_generated;
        pub use self::return_type_generated::*;
        mod return_value_generated;
        pub use self::return_value_generated::*;
        mod hlint_generated;
        pub use self::hlint_generated::*;
        mod hllong_generated;
        pub use self::hllong_generated::*;
        mod hlstring_generated;
        pub use self::hlstring_generated::*;
        mod hlbool_generated;
        pub use self::hlbool_generated::*;
        mod hlvecbytes_generated;
        pub use self::hlvecbytes_generated::*;
        mod hlvoid_generated;
        pub use self::hlvoid_generated::*;
        mod parameter_generated;
        pub use self::parameter_generated::*;
        mod function_call_generated;
        pub use self::function_call_generated::*;
        mod error_code_generated;
        pub use self::error_code_generated::*;
        mod guest_error_generated;
        pub use self::guest_error_generated::*;
    }
}
