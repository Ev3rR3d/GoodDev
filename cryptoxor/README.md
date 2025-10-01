## Descrição

- Toda execução de malware tem como pré-requisito a execução de um shellcode. Com as técnicas mais modernas, é necessário que executemos este shellcode em memóra.
- Para isto, devemos optar por ofuscá-lo, e desofuscar em tempo de execução
- Temos algumas formas de fazer isto, mas aqui usaremos o `criptoxor.py`
- As opções que temos para isto é: Beacon do Cobalt ou PE_to_shellcode

___

## Shellcode

### Cobalt Strike

- Adicionar uma imagem (qualquer imagem) com o nome imagem.jpg no servidor do Cobalt Strike

![](../Attach/Pasted_image_20240618131159.png)

![](../Attach/Pasted_image_20240618131254.png)

- Baixar um shellcode do Cobalt Strike no formato Stageless Payload, com o Output no formato C

![](../Attach/Pasted_image_20240618131912.png)

- Salvar no local indicado no `cryptoxor.py` com o nome `sc.c`

![](../Attach/Pasted_image_20240618132333.png)

### PE to Shellcode

- Para usar o PE_to_shellcode, é preciso clonar o repositório em um máquina Windows e seguir as instruções de compilação. Também é possível pegar o arquivo Release. Link: [PE_to_shellcode](https://github.com/hasherezade/pe_to_shellcode)
- Agora, transfira o PE (executável) para a máquina windows e rode `pe2shc.exe <executavel> <output>`. Aqui farei o exemplo com o mimikatz.

![](../Attach/Captura_de_tela_2024-06-18_134026.png)
![](../Attach/Captura_de_tela_2024-06-18_134206.png)

- Agora transfira o arquivo gerado para a máquina kali e mova para `sc.c`

![](../Attach/Pasted_image_20240618134516.png)

___

### Criptoxor

- Basicamente, para executar o `criptoxor.py` é preciso ter o arquivo `sc.c` na pasta e a `imagem.jpg` hospedada na C2
- Para executar, basta fazer: 

```bash
cargo build --release 

./cryptoxor/target/release/cryptoxor
```

- Se estiver feito corretamente, o output será este:

  ![](../Attach/Pasted_image_20240618134917.png)

- Agora seu shellcode está pronto para ser utilizado nos malwares `Vermilion` e `Crimson`

