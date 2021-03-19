#![feature(rustc_attrs, bool_to_option, array_chunks, asm, panic_info_message)]
#![no_std]
#![no_main]

use core::mem::MaybeUninit;
use core::mem::size_of_val;

/// The `print!()` macro!
macro_rules! print {
    ($($arg:tt)*) => {
        let _ = core::fmt::write(&mut $crate::Printer,
            format_args!($($arg)*));
    }
}

/// Make a very lightweight result type
type Result<T> = core::result::Result<T, ()>;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    print!("!!! PANIC !!!\n");

    // Print location information
    if let Some(location) = info.location() {
        print!("{}:{}:{}\n", location.file(), location.line(),
            location.column());
    }

    // Print the panic message
    if let Some(message) = info.message() {
        print!("{}\n", message);
    }

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
    /// Bind to an IP and port and accept the first connection and return the
    /// socket to the client
    pub fn bind_and_accept(ip: [u8; 4], port: u16) -> Result<Self> {
        // Create a TCP socket
        let socket = File::tcp_socket()?;
        
        // Create the address
        let mut addr = SockaddrIn {
            family: 0x108,
            port:   port.to_be(),
            addr:   u32::from_ne_bytes(ip),
            unused: MaybeUninit::uninit(),
        };
        
        // bind()
        let res = unsafe {
            core::mem::transmute::<
                usize,
                extern fn(i32, *const SockaddrIn, usize) -> i32
            >(0x001e7900 + 1)(socket.0, &addr, size_of_val(&addr))
        };
        if res == -1 { return Err(()); }

        // listen()
        let res = unsafe {
            core::mem::transmute::<
                usize,
                extern fn(i32, i32) -> i32
            >(0x001e7b08 + 1)(socket.0, 5)
        };
        if res == -1 { return Err(()); }

        // accept()
        let mut addrlen = size_of_val(&addr);
        let res = unsafe {
            core::mem::transmute::<
                usize,
                extern fn(i32, *mut SockaddrIn, *mut usize) -> i32
            >(0x001e7fba + 1)(socket.0, &mut addr, &mut addrlen)
        };
        if res == -1 { return Err(()); }

        // Wrap up the newly received file descriptor for the client in our
        // `File` wrapper
        let client = unsafe { File(res) };

        Ok(Self(client))
    }

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
    pub fn recv(&self, mut buf: impl AsMut<[u8]>) -> Result<usize> {
        let buf = buf.as_mut();

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

/// A basic structure to implement `Write` on such that we can use it for the
/// print macro
pub struct Printer;

impl core::fmt::Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            if let Some(sock) = &DEBUG_SOCKET {
                let _ = sock.send(s);
            }
        }

        Ok(())
    }
}

static mut DEBUG_SOCKET: Option<Socket> = None;

fn send_gdb(socket: &Socket, msg: &str) -> Result<()> {
    let checksum = msg.as_bytes().iter()
        .fold(0u8, |acc, &x| acc.wrapping_add(x));
    print!("${}#{:02x}", msg, checksum);
    Ok(())
}

/// Rust entry point
fn main() -> Result<()> {
    const HEXLUT: &[u8] = b"0123456789abcdef";

    unsafe {
        DEBUG_SOCKET = Some(Socket::bind_and_accept([0, 0, 0, 0], 1235)?);
    }

    let socket = unsafe { DEBUG_SOCKET.as_ref().unwrap() };

    let mut buf = [0u8; 128];

    loop {
        // Read the GDB message
        let bread = socket.recv(&mut buf)?;

        // Convert the message to a string
        let msg = core::str::from_utf8(&buf[..bread]).map_err(|_| ())?;

        // Discard acks
        if msg == "+" {
            continue;
        }

        if let Some(command) = msg.splitn(2, "$").nth(1)
                .and_then(|x| x.rsplitn(2, "#").nth(1)) {
            print!("+");

            match &command[..1] {
                "?" => send_gdb(socket, "S05"),
                "g" => send_gdb(socket, "00000000"),
                "p" => send_gdb(socket, "37133713"),
                "m" => {
                    let addr = command[1..].split(",").nth(0).unwrap();
                    let len  = command[1..].split(",").nth(1).unwrap();
                    let addr = u32::from_str_radix(addr, 16).unwrap();
                    let len  = u32::from_str_radix(len,  16).unwrap();

                    print!("$");
                    let mut checksum = 0u8;
                    for addr in addr..addr + len {
                        let val = unsafe {
                            core::ptr::read_volatile(addr as *const u8)
                        } as usize;
                        print!("{:02x}", val);

                        checksum = checksum
                            .wrapping_add(HEXLUT[(val >> 4) & 0xf]);
                        checksum = checksum
                            .wrapping_add(HEXLUT[(val >> 0) & 0xf]);
                    }
                    print!("#{:02x}", checksum);

                    Ok(())
                }
                _   => send_gdb(socket, ""),
            };
        }
    }

    Ok(())
}

