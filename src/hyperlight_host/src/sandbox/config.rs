use std::cmp::max;
use std::time::Duration;

use tracing::{instrument, Span};

use crate::mem::pe::pe_info::PEInfo;
use crate::option_when;

/// The complete set of configuration needed to create a Sandbox
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct SandboxConfiguration {
    /// The maximum size of the guest error buffer.
    guest_error_buffer_size: usize,
    /// The size of the memory buffer that is made available for Guest Function
    /// Definitions
    host_function_definition_size: usize,
    /// The size of the memory buffer that is made available for serialising
    /// Host Exceptions
    host_exception_size: usize,
    /// The size of the memory buffer that is made available for input to the
    /// Guest Binary
    input_data_size: usize,
    /// The size of the memory buffer that is made available for input to the
    /// Guest Binary
    output_data_size: usize,
    /// The stack size to use in the guest sandbox. If set to 0, the stack
    /// size will be determined from the PE file header
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    stack_size_override: u64,
    /// The heap size to use in the guest sandbox. If set to 0, the heap
    /// size will be determined from the PE file header
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    heap_size_override: u64,
    /// The max_execution_time of a guest execution in milliseconds. If set to 0, the max_execution_time
    /// will be set to the default value of 1000ms if the guest execution does not complete within the time specified
    /// then the execution will be cancelled, the minimum value is 1ms
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    max_execution_time: u16,
    /// The max_wait_for_cancellation represents the maximum time the host should wait for a guest execution to be cancelled
    /// If set to 0, the max_wait_for_cancellation will be set to the default value of 10ms.
    /// The minimum value is 1ms.
    ///
    /// Note: this is a C-compatible struct, so even though this optional
    /// field should be represented as an `Option`, that type is not
    /// FFI-safe, so it cannot be.
    max_wait_for_cancellation: u8,
    /// The size of the memory buffer that is made available for serializing
    /// guest panic context
    guest_panic_context_buffer_size: usize,
}

impl SandboxConfiguration {
    /// The default size of input data
    pub const DEFAULT_INPUT_SIZE: usize = 0x4000;
    /// The minimum size of input data
    pub const MIN_INPUT_SIZE: usize = 0x2000;
    /// The default size of output data
    pub const DEFAULT_OUTPUT_SIZE: usize = 0x4000;
    /// The minimum size of output data
    pub const MIN_OUTPUT_SIZE: usize = 0x2000;
    /// The default size of host function definitions
    /// Host function definitions has its own page in memory, in order to be READ-ONLY
    /// from a guest's perspective.
    pub const DEFAULT_HOST_FUNCTION_DEFINITION_SIZE: usize = 0x1000;
    /// The minimum size of host function definitions
    pub const MIN_HOST_FUNCTION_DEFINITION_SIZE: usize = 0x1000;
    /// The default size for host exceptions
    pub const DEFAULT_HOST_EXCEPTION_SIZE: usize = 0x4000;
    /// The minimum size for host exceptions
    pub const MIN_HOST_EXCEPTION_SIZE: usize = 0x4000;
    /// The default size for guest error messages
    pub const DEFAULT_GUEST_ERROR_BUFFER_SIZE: usize = 0x100;
    /// The minimum size for guest error messages
    pub const MIN_GUEST_ERROR_BUFFER_SIZE: usize = 0x80;
    /// The default and minimum values for max execution time (in milliseconds)
    pub const DEFAULT_MAX_EXECUTION_TIME: u16 = 1000;
    /// The minimum value for max execution time (in milliseconds)
    pub const MIN_MAX_EXECUTION_TIME: u16 = 1;
    /// The default and minimum values for max wait for cancellation (in milliseconds)
    pub const DEFAULT_MAX_WAIT_FOR_CANCELLATION: u8 = 100;
    /// The minimum value for max wait for cancellation (in milliseconds)
    pub const MIN_MAX_WAIT_FOR_CANCELLATION: u8 = 10;
    /// The default and minimum values for guest panic context data
    pub const DEFAULT_GUEST_PANIC_CONTEXT_BUFFER_SIZE: usize = 0x400;
    /// The minimum value for guest panic context data
    pub const MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE: usize = 0x400;

    #[allow(clippy::too_many_arguments)]
    /// Create a new configuration for a sandbox with the given sizes.
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn new(
        input_data_size: usize,
        output_data_size: usize,
        function_definition_size: usize,
        host_exception_size: usize,
        guest_error_buffer_size: usize,
        stack_size_override: Option<u64>,
        heap_size_override: Option<u64>,
        max_execution_time: Option<Duration>,
        max_wait_for_cancellation: Option<Duration>,
        guest_panic_context_buffer_size: usize,
    ) -> Self {
        Self {
            input_data_size: max(input_data_size, Self::MIN_INPUT_SIZE),
            output_data_size: max(output_data_size, Self::MIN_OUTPUT_SIZE),
            host_function_definition_size: max(
                function_definition_size,
                Self::MIN_HOST_FUNCTION_DEFINITION_SIZE,
            ),
            host_exception_size: max(host_exception_size, Self::MIN_HOST_EXCEPTION_SIZE),
            guest_error_buffer_size: max(
                guest_error_buffer_size,
                Self::MIN_GUEST_ERROR_BUFFER_SIZE,
            ),
            stack_size_override: stack_size_override.unwrap_or(0),
            heap_size_override: heap_size_override.unwrap_or(0),
            max_execution_time: {
                match max_execution_time {
                    Some(max_execution_time) => match max_execution_time.as_millis() {
                        0 => Self::DEFAULT_MAX_EXECUTION_TIME,
                        1..=65_535u128 => max(
                            max_execution_time.as_millis(),
                            Self::MIN_MAX_EXECUTION_TIME.into(),
                        ) as u16,
                        _ => Self::DEFAULT_MAX_EXECUTION_TIME,
                    },
                    None => Self::DEFAULT_MAX_EXECUTION_TIME,
                }
            },
            max_wait_for_cancellation: {
                match max_wait_for_cancellation {
                    Some(max_wait_for_cancellation) => {
                        match max_wait_for_cancellation.as_millis() {
                            0 => Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
                            1..=255u128 => max(
                                max_wait_for_cancellation.as_millis(),
                                Self::MIN_MAX_WAIT_FOR_CANCELLATION.into(),
                            ) as u8,
                            _ => Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
                        }
                    }
                    None => Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
                }
            },
            guest_panic_context_buffer_size: max(
                guest_panic_context_buffer_size,
                Self::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE,
            ),
        }
    }

    /// Set the size of the memory buffer that is made available for input to the guest
    /// the minimum value is MIN_INPUT_SIZE
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_input_data_size(&mut self, input_data_size: usize) {
        self.input_data_size = max(input_data_size, Self::MIN_INPUT_SIZE);
    }

    /// Set the size of the memory buffer that is made available for output from the guest
    /// the minimum value is MIN_OUTPUT_SIZE
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_output_data_size(&mut self, output_data_size: usize) {
        self.output_data_size = max(output_data_size, Self::MIN_OUTPUT_SIZE);
    }

    /// Set the size of the memory buffer that is made available for serialising host function definitions
    /// the minimum value is MIN_HOST_FUNCTION_DEFINITION_SIZE
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_host_function_definition_size(&mut self, host_function_definition_size: usize) {
        self.host_function_definition_size = max(
            host_function_definition_size,
            Self::MIN_HOST_FUNCTION_DEFINITION_SIZE,
        );
    }

    /// Set the size of the memory buffer that is made available for serialising host exceptions
    /// the minimum value is MIN_HOST_EXCEPTION_SIZE
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_host_exception_size(&mut self, host_exception_size: usize) {
        self.host_exception_size = max(host_exception_size, Self::MIN_HOST_EXCEPTION_SIZE);
    }

    /// Set the size of the memory buffer that is made available for serialising guest error messages
    /// the minimum value is MIN_GUEST_ERROR_BUFFER_SIZE
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_guest_error_buffer_size(&mut self, guest_error_buffer_size: usize) {
        self.guest_error_buffer_size =
            max(guest_error_buffer_size, Self::MIN_GUEST_ERROR_BUFFER_SIZE);
    }

    /// Set the stack size to use in the guest sandbox. If set to 0, the stack size will be determined from the PE file header
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_stack_size(&mut self, stack_size: u64) {
        self.stack_size_override = stack_size;
    }

    /// Set the heap size to use in the guest sandbox. If set to 0, the heap size will be determined from the PE file header
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_heap_size(&mut self, heap_size: u64) {
        self.heap_size_override = heap_size;
    }

    /// Set the maximum execution time of a guest function execution. If set to 0, the max_execution_time
    /// will be set to the default value of DEFAULT_MAX_EXECUTION_TIME if the guest execution does not complete within the time specified
    /// then the execution will be cancelled, the minimum value is MIN_MAX_EXECUTION_TIME
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_max_execution_time(&mut self, max_execution_time: Duration) {
        match max_execution_time.as_millis() {
            0 => self.max_execution_time = Self::DEFAULT_MAX_EXECUTION_TIME,
            1..=65_535u128 => {
                self.max_execution_time = max(
                    max_execution_time.as_millis(),
                    Self::MIN_MAX_EXECUTION_TIME.into(),
                ) as u16
            }
            _ => self.max_execution_time = Self::DEFAULT_MAX_EXECUTION_TIME,
        }
    }

    /// Set the maximum time to wait for guest execution calculation. If set to 0, the maximum cancellation time
    /// will be set to the default value of DEFAULT_MAX_WAIT_FOR_CANCELLATION if the guest execution cancellation does not complete within the time specified
    /// then an eror will be returned, the minimum value is MIN_MAX_WAIT_FOR_CANCELLATION
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub fn set_max_execution_cancel_wait_time(&mut self, max_wait_for_cancellation: Duration) {
        match max_wait_for_cancellation.as_millis() {
            0 => self.max_wait_for_cancellation = Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
            1..=255u128 => {
                self.max_wait_for_cancellation = max(
                    max_wait_for_cancellation.as_millis(),
                    Self::MIN_MAX_WAIT_FOR_CANCELLATION.into(),
                ) as u8
            }
            _ => self.max_wait_for_cancellation = Self::DEFAULT_MAX_WAIT_FOR_CANCELLATION,
        }
    }

    /// Set the size of the memory buffer that is made available for serializing guest panic context
    /// the minimum value is MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE
    pub fn set_guest_panic_context_buffer_size(&mut self, guest_panic_context_buffer_size: usize) {
        self.guest_panic_context_buffer_size = max(
            guest_panic_context_buffer_size,
            Self::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE,
        );
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_guest_error_buffer_size(&self) -> usize {
        self.guest_error_buffer_size
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_host_function_definition_size(&self) -> usize {
        self.host_function_definition_size
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_host_exception_size(&self) -> usize {
        self.host_exception_size
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_input_data_size(&self) -> usize {
        self.input_data_size
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_output_data_size(&self) -> usize {
        self.output_data_size
    }

    #[instrument(skip_all, parent = Span::current(), level="Trace")]
    pub(crate) fn get_guest_panic_context_buffer_size(&self) -> usize {
        self.guest_panic_context_buffer_size
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_max_execution_time(&self) -> u16 {
        self.max_execution_time
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_max_wait_for_cancellation(&self) -> u8 {
        self.max_wait_for_cancellation
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn stack_size_override_opt(&self) -> Option<u64> {
        option_when(self.stack_size_override, self.stack_size_override > 0)
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn heap_size_override_opt(&self) -> Option<u64> {
        option_when(self.heap_size_override, self.heap_size_override > 0)
    }

    /// If self.stack_size is non-zero, return it. Otherwise,
    /// return pe_info.stack_reserve()
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_stack_size(&self, pe_info: &PEInfo) -> u64 {
        self.stack_size_override_opt()
            .unwrap_or_else(|| pe_info.stack_reserve())
    }

    /// If self.heap_size_override is non-zero, return it. Otherwise,
    /// return pe_info.heap_reserve()
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(crate) fn get_heap_size(&self, pe_info: &PEInfo) -> u64 {
        self.heap_size_override_opt()
            .unwrap_or_else(|| pe_info.heap_reserve())
    }
}

impl Default for SandboxConfiguration {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_INPUT_SIZE,
            Self::DEFAULT_OUTPUT_SIZE,
            Self::DEFAULT_HOST_FUNCTION_DEFINITION_SIZE,
            Self::DEFAULT_HOST_EXCEPTION_SIZE,
            Self::DEFAULT_GUEST_ERROR_BUFFER_SIZE,
            None,
            None,
            None,
            None,
            Self::DEFAULT_GUEST_PANIC_CONTEXT_BUFFER_SIZE,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::SandboxConfiguration;
    use crate::testing::{callback_guest_pe_info, simple_guest_pe_info};

    #[test]
    fn overrides() {
        const STACK_SIZE_OVERRIDE: u64 = 0x10000;
        const HEAP_SIZE_OVERRIDE: u64 = 0x50000;
        const INPUT_DATA_SIZE_OVERRIDE: usize = 0x4000;
        const OUTPUT_DATA_SIZE_OVERRIDE: usize = 0x4001;
        const HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE: usize = 0x4002;
        const HOST_EXCEPTION_SIZE_OVERRIDE: usize = 0x4003;
        const GUEST_ERROR_BUFFER_SIZE_OVERRIDE: usize = 0x40004;
        const MAX_EXECUTION_TIME_OVERRIDE: u16 = 1010;
        const MAX_WAIT_FOR_CANCELLATION_OVERRIDE: u8 = 200;
        const GUEST_PANIC_CONTEXT_BUFFER_SIZE_OVERRIDE: usize = 0x4005;
        let cfg = SandboxConfiguration::new(
            INPUT_DATA_SIZE_OVERRIDE,
            OUTPUT_DATA_SIZE_OVERRIDE,
            HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE,
            HOST_EXCEPTION_SIZE_OVERRIDE,
            GUEST_ERROR_BUFFER_SIZE_OVERRIDE,
            Some(STACK_SIZE_OVERRIDE),
            Some(HEAP_SIZE_OVERRIDE),
            Some(Duration::from_millis(MAX_EXECUTION_TIME_OVERRIDE as u64)),
            Some(Duration::from_millis(
                MAX_WAIT_FOR_CANCELLATION_OVERRIDE as u64,
            )),
            GUEST_PANIC_CONTEXT_BUFFER_SIZE_OVERRIDE,
        );
        let pe_infos = vec![
            simple_guest_pe_info().unwrap(),
            callback_guest_pe_info().unwrap(),
        ];
        for pe_info in pe_infos {
            let stack_size = cfg.get_stack_size(&pe_info);
            let heap_size = cfg.get_heap_size(&pe_info);
            assert_eq!(STACK_SIZE_OVERRIDE, stack_size);
            assert_eq!(HEAP_SIZE_OVERRIDE, heap_size);
        }
        let cfg = SandboxConfiguration::new(
            INPUT_DATA_SIZE_OVERRIDE,
            OUTPUT_DATA_SIZE_OVERRIDE,
            HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE,
            HOST_EXCEPTION_SIZE_OVERRIDE,
            GUEST_ERROR_BUFFER_SIZE_OVERRIDE,
            Some(1024),
            Some(2048),
            Some(Duration::from_millis(MAX_EXECUTION_TIME_OVERRIDE as u64)),
            Some(Duration::from_millis(
                MAX_WAIT_FOR_CANCELLATION_OVERRIDE as u64,
            )),
            GUEST_PANIC_CONTEXT_BUFFER_SIZE_OVERRIDE,
        );
        assert_eq!(1024, cfg.stack_size_override);
        assert_eq!(2048, cfg.heap_size_override);
        assert_eq!(INPUT_DATA_SIZE_OVERRIDE, cfg.input_data_size);
        assert_eq!(OUTPUT_DATA_SIZE_OVERRIDE, cfg.output_data_size);
        assert_eq!(
            HOST_FUNCTION_DEFINITION_SIZE_OVERRIDE,
            cfg.host_function_definition_size
        );
        assert_eq!(HOST_EXCEPTION_SIZE_OVERRIDE, cfg.host_exception_size);
        assert_eq!(
            GUEST_ERROR_BUFFER_SIZE_OVERRIDE,
            cfg.guest_error_buffer_size
        );
        assert_eq!(MAX_EXECUTION_TIME_OVERRIDE, cfg.max_execution_time);
        assert_eq!(
            MAX_WAIT_FOR_CANCELLATION_OVERRIDE,
            cfg.max_wait_for_cancellation
        );
        assert_eq!(
            GUEST_PANIC_CONTEXT_BUFFER_SIZE_OVERRIDE,
            cfg.guest_panic_context_buffer_size
        );
    }

    #[test]
    fn min_sizes() {
        let mut cfg = SandboxConfiguration::new(
            SandboxConfiguration::MIN_INPUT_SIZE - 1,
            SandboxConfiguration::MIN_OUTPUT_SIZE - 1,
            SandboxConfiguration::MIN_HOST_FUNCTION_DEFINITION_SIZE - 1,
            SandboxConfiguration::MIN_HOST_EXCEPTION_SIZE - 1,
            SandboxConfiguration::MIN_GUEST_ERROR_BUFFER_SIZE - 1,
            None,
            None,
            Some(Duration::from_millis(
                SandboxConfiguration::MIN_MAX_EXECUTION_TIME as u64,
            )),
            Some(Duration::from_millis(
                SandboxConfiguration::MIN_MAX_WAIT_FOR_CANCELLATION as u64 - 1,
            )),
            SandboxConfiguration::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE - 1,
        );
        assert_eq!(SandboxConfiguration::MIN_INPUT_SIZE, cfg.input_data_size);
        assert_eq!(SandboxConfiguration::MIN_OUTPUT_SIZE, cfg.output_data_size);
        assert_eq!(
            SandboxConfiguration::MIN_HOST_FUNCTION_DEFINITION_SIZE,
            cfg.host_function_definition_size
        );
        assert_eq!(
            SandboxConfiguration::MIN_HOST_EXCEPTION_SIZE,
            cfg.host_exception_size
        );
        assert_eq!(
            SandboxConfiguration::MIN_GUEST_ERROR_BUFFER_SIZE,
            cfg.guest_error_buffer_size
        );
        assert_eq!(0, cfg.stack_size_override);
        assert_eq!(0, cfg.heap_size_override);
        assert_eq!(
            SandboxConfiguration::MIN_MAX_EXECUTION_TIME,
            cfg.max_execution_time
        );
        assert_eq!(
            SandboxConfiguration::MIN_MAX_WAIT_FOR_CANCELLATION,
            cfg.max_wait_for_cancellation
        );
        assert_eq!(
            SandboxConfiguration::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE,
            cfg.guest_panic_context_buffer_size
        );

        cfg.set_input_data_size(SandboxConfiguration::MIN_INPUT_SIZE - 1);
        cfg.set_output_data_size(SandboxConfiguration::MIN_OUTPUT_SIZE - 1);
        cfg.set_host_function_definition_size(
            SandboxConfiguration::MIN_HOST_FUNCTION_DEFINITION_SIZE - 1,
        );
        cfg.set_host_exception_size(SandboxConfiguration::MIN_HOST_EXCEPTION_SIZE - 1);
        cfg.set_guest_error_buffer_size(SandboxConfiguration::MIN_GUEST_ERROR_BUFFER_SIZE - 1);
        cfg.set_max_execution_time(Duration::from_millis(
            SandboxConfiguration::MIN_MAX_EXECUTION_TIME as u64,
        ));
        cfg.set_max_execution_cancel_wait_time(Duration::from_millis(
            SandboxConfiguration::MIN_MAX_WAIT_FOR_CANCELLATION as u64 - 1,
        ));
        cfg.set_guest_panic_context_buffer_size(
            SandboxConfiguration::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE - 1,
        );

        assert_eq!(SandboxConfiguration::MIN_INPUT_SIZE, cfg.input_data_size);
        assert_eq!(SandboxConfiguration::MIN_OUTPUT_SIZE, cfg.output_data_size);
        assert_eq!(
            SandboxConfiguration::MIN_HOST_FUNCTION_DEFINITION_SIZE,
            cfg.host_function_definition_size
        );
        assert_eq!(
            SandboxConfiguration::MIN_HOST_EXCEPTION_SIZE,
            cfg.host_exception_size
        );
        assert_eq!(
            SandboxConfiguration::MIN_GUEST_ERROR_BUFFER_SIZE,
            cfg.guest_error_buffer_size
        );
        assert_eq!(
            SandboxConfiguration::MIN_MAX_EXECUTION_TIME,
            cfg.max_execution_time
        );
        assert_eq!(
            SandboxConfiguration::MIN_MAX_WAIT_FOR_CANCELLATION,
            cfg.max_wait_for_cancellation
        );
        assert_eq!(
            SandboxConfiguration::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE,
            cfg.guest_panic_context_buffer_size
        );
    }

    mod proptests {
        use proptest::prelude::*;

        use super::SandboxConfiguration;

        proptest! {
            #[test]
            fn error_buffer_size(size in SandboxConfiguration::MIN_GUEST_ERROR_BUFFER_SIZE..=SandboxConfiguration::MIN_GUEST_ERROR_BUFFER_SIZE * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_guest_error_buffer_size(size);
                prop_assert_eq!(size, cfg.get_guest_error_buffer_size());
            }

            #[test]
            fn host_function_definition_size(size in SandboxConfiguration::MIN_HOST_FUNCTION_DEFINITION_SIZE..=SandboxConfiguration::MIN_HOST_FUNCTION_DEFINITION_SIZE * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_host_function_definition_size(size);
                prop_assert_eq!(size, cfg.get_host_function_definition_size());
            }

            #[test]
            fn host_exception_size(size in SandboxConfiguration::MIN_HOST_EXCEPTION_SIZE..=SandboxConfiguration::MIN_HOST_EXCEPTION_SIZE * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_host_exception_size(size);
                prop_assert_eq!(size, cfg.get_host_exception_size());
            }

            #[test]
            fn input_data_size(size in SandboxConfiguration::MIN_INPUT_SIZE..=SandboxConfiguration::MIN_INPUT_SIZE * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_input_data_size(size);
                prop_assert_eq!(size, cfg.get_input_data_size());
            }

            #[test]
            fn output_data_size(size in SandboxConfiguration::MIN_OUTPUT_SIZE..=SandboxConfiguration::MIN_OUTPUT_SIZE * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_output_data_size(size);
                prop_assert_eq!(size, cfg.get_output_data_size());
            }

            #[test]
            fn guest_panic_context_buffer_size(size in SandboxConfiguration::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE..=SandboxConfiguration::MIN_GUEST_PANIC_CONTEXT_BUFFER_SIZE * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_guest_panic_context_buffer_size(size);
                prop_assert_eq!(size, cfg.get_guest_panic_context_buffer_size());
            }

            #[test]
            fn max_execution_time(time in SandboxConfiguration::MIN_MAX_EXECUTION_TIME..=SandboxConfiguration::MIN_MAX_EXECUTION_TIME * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_max_execution_time(std::time::Duration::from_millis(time.into()));
                prop_assert_eq!(time, cfg.get_max_execution_time());
            }

            #[test]
            fn max_wait_for_cancellation(time in SandboxConfiguration::MIN_MAX_WAIT_FOR_CANCELLATION..=SandboxConfiguration::MIN_MAX_WAIT_FOR_CANCELLATION * 10) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_max_execution_cancel_wait_time(std::time::Duration::from_millis(time.into()));
                prop_assert_eq!(time, cfg.get_max_wait_for_cancellation());
            }

            #[test]
            fn stack_size_override(size in 0x1000..=0x10000u64) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_stack_size(size);
                prop_assert_eq!(size, cfg.stack_size_override);
            }

            #[test]
            fn heap_size_override(size in 0x1000..=0x10000u64) {
                let mut cfg = SandboxConfiguration::default();
                cfg.set_heap_size(size);
                prop_assert_eq!(size, cfg.heap_size_override);
            }
        }
    }
}
