use core::slice;
use std::io::Write;
use std::{fs, time::Duration};

use kvm_ioctls::{Kvm, VcpuExit};

const GUEST_MEM_SIZE: u64 = 51_200_000;
const CODE_START: u16 = 0x1000;

fn main() {
    let kvm = Kvm::new().unwrap();

    let (vm, guest_mem_start) = create_vm(&kvm);

    load_binary(guest_mem_start);

    let vcpu = vm.create_vcpu(0).unwrap();
    init_vcpu(&vcpu);

    run(&vcpu);
}

fn create_vm(kvm: &kvm_ioctls::Kvm) -> (kvm_ioctls::VmFd, *mut u8) {
    let vm = kvm.create_vm().unwrap();
    let guest_mem_start: *mut u8 = unsafe {
        libc::mmap(
            std::ptr::null_mut::<libc::c_void>(),
            GUEST_MEM_SIZE as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
            -1,
            0,
        ) as *mut u8
    };
    let mem = kvm_bindings::kvm_userspace_memory_region {
        userspace_addr: guest_mem_start as u64,
        memory_size: GUEST_MEM_SIZE,
        ..Default::default()
    };
    unsafe { vm.set_user_memory_region(mem).unwrap() };
    return (vm, guest_mem_start);
}

fn load_binary(guest_mem_start: *mut u8) {
    let binary = fs::read("test.bin").unwrap();
    unsafe {
        let mut guest_mem = slice::from_raw_parts_mut(guest_mem_start, GUEST_MEM_SIZE as usize);
        guest_mem.write(&binary).unwrap();
    }
}

fn init_vcpu(vcpu: &kvm_ioctls::VcpuFd) {
    let mut sregs = vcpu.get_sregs().unwrap();
    sregs.cs.selector = CODE_START;
    sregs.cs.base = CODE_START as u64 * 16;
    sregs.ss.selector = CODE_START;
    sregs.ss.base = CODE_START as u64 * 16;
    sregs.ds.selector = CODE_START;
    sregs.ds.base = CODE_START as u64 * 16;
    sregs.es.selector = CODE_START;
    sregs.es.base = CODE_START as u64 * 16;
    sregs.fs.selector = CODE_START;
    sregs.fs.base = CODE_START as u64 * 16;
    sregs.gs.selector = CODE_START;
    vcpu.set_sregs(&sregs).unwrap();

    let mut regs = vcpu.get_regs().unwrap();
    regs.rflags = 0x0000000000000002u64;
    regs.rip = 0;
    regs.rsp = 0xffffffff;
    regs.rbp = 0;
    vcpu.set_regs(&regs).unwrap();
}

fn run(vcpu: &kvm_ioctls::VcpuFd) {
    loop {
        match vcpu.run().expect("run failed") {
            VcpuExit::IoOut(port, data) => {
                println!(
                    "Received an I/O out exit. Address: {port}. Data: {:#x}",
                    data[0],
                );
                std::thread::sleep(Duration::from_secs(1))
            }
            r => panic!("Unexpected exit reason: {:?}", r),
        }
    }
}
