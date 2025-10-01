## Descrição
- Malware baseado em APC Injection
- Decoda um shellcode em xor (gerado pelo `criptoxor`) e executa em memória com QueueUserAPC

___

## Hospedagem

- Hospedar o shellcode (e a imagem, que é a mesma utilizada no `criptoxor`) na C2
- Para gerar o `redsc` é só copiar o arquivo `loader` com este nome. O arquivo loader é gerado pelo `criptoxor`

![](../Attach/Pasted_image_20240618140100.png)

___

## Compilação

- Se o malware `Crimson` não estiver compilado, rodar:
  
![](../Attach/Pasted_image_20240618143831.png)

- É comum que dê alguns alertas, mas não tem problema, porque compilará corretamente com a seguinte mensagem:

![](../Attach/Pasted_image_20240618140315.png)

___
## Execução

- Com o malware compilado, basta mover para a máquina alvo e executar

![](../Attach/Captura_de_tela_2024-06-18_144216.png)

![](../Attach/Captura_de_tela_2024-06-18_144312.png)

![](../Attach/Pasted_image_20240618144439.png)

