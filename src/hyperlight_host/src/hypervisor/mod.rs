use tracing::{instrument, Span};

use crate::error::HyperlightError::ExecutionCanceledByHost;
use crate::hypervisor::metrics::HypervisorMetric::NumberOfCancelledGuestExecutions;
use crate::mem::memory_region::{MemoryRegion, MemoryRegionFlags};
use crate::{int_counter_inc, log_then_return, new_error, HyperlightError, Result};

/// Util for handling x87 fpu state
pub mod fpu;
/// Handlers for Hypervisor custom logic
pub mod handlers;
/// HyperV-on-linux functionality
#[cfg(mshv)]
pub mod hyperv_linux;
#[cfg(target_os = "windows")]
/// Hyperv-on-windows functionality
pub(crate) mod hyperv_windows;
pub(crate) mod hypervisor_handler;
#[cfg(kvm)]
/// Functionality to manipulate KVM-based virtual machines
pub mod kvm;
/// Metric definitions for Hypervisor module.
mod metrics;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process;
#[cfg(target_os = "windows")]
/// Hyperlight Surrogate Process
pub(crate) mod surrogate_process_manager;
/// WindowsHypervisorPlatform utilities
#[cfg(target_os = "windows")]
pub(crate) mod windows_hypervisor_platform;
/// Safe wrappers around windows types like `PSTR`
#[cfg(target_os = "windows")]
pub(crate) mod wrappers;

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use self::handlers::{
    MemAccessHandlerCaller, MemAccessHandlerWrapper, OutBHandlerCaller, OutBHandlerWrapper,
};
use crate::hypervisor::hypervisor_handler::HypervisorHandler;
use crate::mem::ptr::RawPtr;

#[cfg(all(target_os = "linux", not(mshv), not(kvm)))]
compile_error!("Hyperlight requires either the `mshv` or `kvm` feature to be enabled on Linux");

pub(crate) const CR4_PAE: u64 = 1 << 5;
pub(crate) const CR4_OSFXSR: u64 = 1 << 9;
pub(crate) const CR4_OSXMMEXCPT: u64 = 1 << 10;
pub(crate) const CR0_PE: u64 = 1;
pub(crate) const CR0_MP: u64 = 1 << 1;
pub(crate) const CR0_ET: u64 = 1 << 4;
pub(crate) const CR0_NE: u64 = 1 << 5;
pub(crate) const CR0_WP: u64 = 1 << 16;
pub(crate) const CR0_AM: u64 = 1 << 18;
pub(crate) const CR0_PG: u64 = 1 << 31;
pub(crate) const EFER_LME: u64 = 1 << 8;
pub(crate) const EFER_LMA: u64 = 1 << 10;
pub(crate) const EFER_SCE: u64 = 1;
pub(crate) const EFER_NX: u64 = 1 << 11;

/// These are the generic exit reasons that we can handle from a Hypervisor the Hypervisors run method is responsible for mapping from
/// the hypervisor specific exit reasons to these generic ones
pub enum HyperlightExit {
    /// The vCPU has halted
    Halt(),
    /// The vCPU has issued a write to the given port with the given value
    IoOut(u16, Vec<u8>, u64, u64),
    /// The vCPU has attempted to read or write from an unmapped address
    Mmio(u64),
    /// The vCPU tried to access memory but was missing the required permissions
    AccessViolation(u64, MemoryRegionFlags, MemoryRegionFlags),
    /// The vCPU execution has been cancelled
    Cancelled(),
    /// The vCPU has exited for a reason that is not handled by Hyperlight
    Unknown(String),
    /// The operation should be retried, for example this can happen on Linux where a call to run the CPU can return EAGAIN
    Retry(),
}

/// A common set of hypervisor functionality
///
/// Note: a lot of these structures take in an `Option<HypervisorHandler>`.
/// This is because, if we are coming from the C API, we don't have a HypervisorHandler and have
/// to account for the fact the Hypervisor was set up beforehand.
pub(crate) trait Hypervisor: Debug + Sync + Send {
    /// Initialise the internally stored vCPU with the given PEB address and
    /// random number seed, then run it until a HLT instruction.
    #[allow(clippy::too_many_arguments)]
    fn initialise(
        &mut self,
        peb_addr: RawPtr,
        seed: u64,
        page_size: u32,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
        hv_handler: Option<HypervisorHandler>,
    ) -> Result<()>;

    /// Dispatch a call from the host to the guest using the given pointer
    /// to the dispatch function _in the guest's address space_.
    ///
    /// Do this by setting the instruction pointer to `dispatch_func_addr`
    /// and then running the execution loop until a halt instruction.
    ///
    /// Returns `Ok` if the call succeeded, and an `Err` if it failed
    fn dispatch_call_from_host(
        &mut self,
        dispatch_func_addr: RawPtr,
        outb_handle_fn: OutBHandlerWrapper,
        mem_access_fn: MemAccessHandlerWrapper,
        hv_handler: Option<HypervisorHandler>,
    ) -> Result<()>;

    /// Handle an IO exit from the internally stored vCPU.
    fn handle_io(
        &mut self,
        port: u16,
        data: Vec<u8>,
        rip: u64,
        instruction_length: u64,
        outb_handle_fn: OutBHandlerWrapper,
    ) -> Result<()>;

    /// Run the vCPU
    fn run(&mut self) -> Result<HyperlightExit>;

    /// Returns a Some(HyperlightExit::AccessViolation(..)) if the given gpa doesn't have
    /// access its corresponding region. Returns None otherwise, or if the region is not found.
    fn get_memory_access_violation(
        &self,
        gpa: usize,
        mem_regions: &[MemoryRegion],
        access_info: MemoryRegionFlags,
    ) -> Option<HyperlightExit> {
        // find the region containing the given gpa
        let region = mem_regions
            .iter()
            .find(|region| region.guest_region.contains(&gpa));

        if let Some(region) = region {
            if !region.flags.contains(access_info)
                || region.flags.contains(MemoryRegionFlags::STACK_GUARD)
            {
                return Some(HyperlightExit::AccessViolation(
                    gpa as u64,
                    access_info,
                    region.flags,
                ));
            }
        }
        None
    }

    /// Get the logging level to pass to the guest entrypoint
    fn get_max_log_level(&self) -> u32 {
        log::max_level() as u32
    }

    /// get a mutable trait object from self
    fn as_mut_hypervisor(&mut self) -> &mut dyn Hypervisor;

    /// Get the partition handle for WHP
    #[cfg(target_os = "windows")]
    fn get_partition_handle(&self) -> windows::Win32::System::Hypervisor::WHV_PARTITION_HANDLE;

    /// Dump memory to a file on crash
    #[cfg(all(debug_assertions, feature = "dump_on_crash", target_os = "linux"))]
    fn dump_on_crash(&self, mem_regions: Vec<MemoryRegion>) {
        let memory_size = mem_regions
            .iter()
            .map(|region| region.guest_region.end - region.guest_region.start)
            .sum();
        if let Err(e) = unsafe {
            self.write_dump_file(
                mem_regions.clone(),
                mem_regions[0].host_region.start as *const u8,
                memory_size,
            )
        } {
            println!("Error dumping memory: {}", e);
        }
    }

    /// A function that takes an address and a size and writes the memory at that address to a file in the temp/tmp directory
    ///  # Safety
    /// This function is unsafe because it is writing memory to a file, make sure that the address is valid and that the size is correct
    /// This function is only available when the `dump_on_crash` feature is enabled and running in debug mode
    #[cfg(all(feature = "dump_on_crash", debug_assertions))]
    unsafe fn write_dump_file(
        &self,
        regions: Vec<MemoryRegion>,
        address: *const u8,
        size: usize,
    ) -> Result<()> {
        use std::io::Write;

        use tempfile::NamedTempFile;

        if address.is_null() || size == 0 {
            return Err(new_error!("Invalid address or size"));
        }

        let hv_details = format!("{:#?}", self);
        let regions_details = format!("{:#?}", regions);

        // Create a temporary file
        let mut file = NamedTempFile::with_prefix("mem")?;
        let temp_path = file.path().to_path_buf();

        file.write_all(hv_details.as_bytes())?;
        file.write_all(b"\n")?;
        file.write_all(regions_details.as_bytes())?;
        file.write_all(b"\n")?;

        // SAFETY: Ensure the address and size are valid and accessible
        unsafe {
            let slice = std::slice::from_raw_parts(address, size);
            file.write_all(slice)?;
            file.flush()?;
        }
        let persist_path = temp_path.with_extension("dmp");
        file.persist(&persist_path)
            .map_err(|e| new_error!("Failed to persist file: {:?}", e))?;

        print!("Memory dumped to file: {:?}", persist_path);
        log::error!("Memory dumped to file: {:?}", persist_path);

        Ok(())
    }
}

/// A virtual CPU that can be run until an exit occurs
pub struct VirtualCPU {}

impl VirtualCPU {
    /// Run the given hypervisor until a halt instruction is reached
    #[instrument(err(Debug), skip_all, parent = Span::current(), level = "Trace")]
    pub fn run(
        hv: &mut dyn Hypervisor,
        hv_handler: Option<HypervisorHandler>,
        outb_handle_fn: Arc<Mutex<dyn OutBHandlerCaller>>,
        mem_access_fn: Arc<Mutex<dyn MemAccessHandlerCaller>>,
    ) -> Result<()> {
        loop {
            match hv.run()? {
                HyperlightExit::Halt() => {
                    break;
                }
                HyperlightExit::IoOut(port, data, rip, instruction_length) => {
                    hv.handle_io(port, data, rip, instruction_length, outb_handle_fn.clone())?
                }

                HyperlightExit::Mmio(addr) => {
                    mem_access_fn
                        .clone()
                        .try_lock()
                        .map_err(|e| new_error!("Error locking at {}:{}: {}", file!(), line!(), e))?
                        .call()?;
                    log_then_return!("MMIO access address {:#x}", addr);
                }
                HyperlightExit::AccessViolation(addr, tried, region_permission) => {
                    if region_permission.intersects(MemoryRegionFlags::STACK_GUARD) {
                        return Err(HyperlightError::StackOverflow());
                    }
                    log_then_return!(HyperlightError::MemoryAccessViolation(
                        addr,
                        tried,
                        region_permission
                    ));
                }
                HyperlightExit::Cancelled() => {
                    // Shutdown is returned when the host has cancelled execution
                    // After termination, the main thread will re-initialize the VM
                    if let Some(hvh) = hv_handler {
                        // If hvh is None, then we are running from the C API, which doesn't use
                        // the HypervisorHandler
                        hvh.set_running(false);
                        #[cfg(target_os = "linux")]
                        hvh.set_run_cancelled(true);
                    }
                    int_counter_inc!(&NumberOfCancelledGuestExecutions);
                    log_then_return!(ExecutionCanceledByHost());
                }
                HyperlightExit::Unknown(reason) => {
                    log_then_return!("Unexpected VM Exit {:?}", reason);
                }
                HyperlightExit::Retry() => continue,
            }
        }

        Ok(())
    }
}

#[cfg(all(test, any(target_os = "windows", kvm)))]
pub(crate) mod tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use hyperlight_testing::dummy_guest_as_string;

    use super::handlers::{MemAccessHandlerWrapper, OutBHandlerWrapper};
    use crate::hypervisor::hypervisor_handler::{
        HvHandlerConfig, HypervisorHandler, HypervisorHandlerAction,
    };
    use crate::mem::ptr::RawPtr;
    use crate::sandbox::uninitialized::GuestBinary;
    use crate::sandbox::{SandboxConfiguration, UninitializedSandbox};
    use crate::{new_error, Result};

    pub(crate) fn test_initialise(
        outb_hdl: OutBHandlerWrapper,
        mem_access_hdl: MemAccessHandlerWrapper,
    ) -> Result<()> {
        let filename = dummy_guest_as_string().map_err(|e| new_error!("{}", e))?;
        if !Path::new(&filename).exists() {
            return Err(new_error!(
                "test_initialise: file {} does not exist",
                filename
            ));
        }

        let sandbox =
            UninitializedSandbox::new(GuestBinary::FilePath(filename.clone()), None, None, None)?;
        let (hshm, gshm) = sandbox.mgr.build();
        drop(hshm);

        let hv_handler_config = HvHandlerConfig {
            outb_handler: outb_hdl,
            mem_access_handler: mem_access_hdl,
            seed: 1234567890,
            page_size: 4096,
            peb_addr: RawPtr::from(0x230000),
            dispatch_function_addr: Arc::new(Mutex::new(None)),
            max_init_time: Duration::from_millis(
                SandboxConfiguration::DEFAULT_MAX_INITIALIZATION_TIME as u64,
            ),
            max_exec_time: Duration::from_millis(
                SandboxConfiguration::DEFAULT_MAX_EXECUTION_TIME as u64,
            ),
            max_wait_for_cancellation: Duration::from_millis(
                SandboxConfiguration::DEFAULT_MAX_WAIT_FOR_CANCELLATION as u64,
            ),
        };

        let mut hv_handler = HypervisorHandler::new(hv_handler_config);

        // call initialise on the hypervisor implementation with specific values
        // for PEB (process environment block) address, seed and page size.
        //
        // these values are not actually used, they're just checked inside
        // the dummy guest, and if they don't match these values, the dummy
        // guest issues a write to an invalid memory address, which in turn
        // fails this test.
        //
        // in this test, we're not actually testing whether a guest can issue
        // memory operations, call functions, etc... - we're just testing
        // whether we can configure the shared memory region, load a binary
        // into it, and run the CPU to completion (e.g., a HLT interrupt)

        hv_handler.start_hypervisor_handler(gshm)?;

        hv_handler.execute_hypervisor_handler_action(HypervisorHandlerAction::Initialise)
    }
}
