# Tiny Macro Linux

Um gravador de ações de teclado e mouse para Linux, inspirado no TinyTask. Grave uma sequência, reproduza e salve suas macros no computador.

> **Projeto em desenvolvimento:** gravação e reprodução básicas, incluindo cliques e rolagem, foram testadas no Hyprland. A janela abriu no KWin Wayland, mas a gravação e a reprodução ainda não foram validadas no KDE.

## Instalação fácil — Omarchy e Arch Linux

O instalador apresenta telas interativas no terminal, compila o aplicativo e cria o atalho no menu. Ele oferece instalar as dependências e configurar permissões separadamente.

```bash
sudo pacman -S --needed git
git clone https://github.com/kimlherme/tiny-macro.git
cd tiny-macro
bash install.sh
```

Execute como usuário normal: o instalador solicitará `sudo` quando necessário. Se `whiptail` estiver instalado, as perguntas aparecem em janelas no terminal; caso contrário, aparecem como perguntas de texto. Para compilar, é necessário acesso às dependências Rust.

**Permissões:** se você aceitar a configuração, o instalador adicionará sua conta ao grupo `input` e criará regras para `/dev/uinput`. Isso dá aos programas executados pela sua conta acesso amplo para ler e simular entrada, podendo expor senhas. Encerre a sessão e entre novamente depois de aceitar. Você pode recusar: o aplicativo será instalado, mas a gravação e a reprodução podem não funcionar sem permissões já configuradas.

Depois da instalação, procure **Tiny Macro Linux** no launcher. O executável fica em `${XDG_DATA_HOME:-$HOME/.local/share}/tiny-macro-linux/` e o atalho em `${XDG_DATA_HOME:-$HOME/.local/share}/applications/`.

## Como usar

1. Clique em **Gravar** ou pressione **F6**.
2. Faça a sequência desejada.
3. Clique em **Parar** ou pressione **F7**, **Pause** ou **Scroll Lock**.
4. Clique em **Reproduzir** ou pressione **F8**. A reprodução começa imediatamente: prepare a janela de destino antes.
5. Use **Salvar** para guardar a macro em JSON e **Abrir** para carregá-la depois.

Em **⚙ Atalhos**, escolha teclas entre F6 e F12 para iniciar e parar a gravação e iniciar a reprodução. Cada ação deve usar uma tecla diferente. Pause e Scroll Lock continuam disponíveis como parada de emergência. Os atalhos também podem ser recebidos pelo aplicativo que estiver em foco.

No Hyprland, **Fixar sobre janelas** torna a janela flutuante e a fixa nos espaços de trabalho. A sobreposição em jogos de tela cheia não foi confirmada; esse botão ainda não foi adaptado para KDE.

## Privacidade e limitações

- As gravações ficam locais, mas podem conter senhas e tudo que você digitou. O arquivo JSON **não é criptografado**.
- A reprodução ocorre uma vez, na velocidade gravada. Alteração de velocidade, repetição, contagem regressiva e exportação para executável ainda não estão disponíveis.
- Arquivos `.rec` do TinyTask original ainda não são compatíveis.
- Movimento relativo, botões, teclado e rolagem são suportados; toque, mesa digitalizadora e movimento absoluto ainda não são.
- Dispositivos conectados após o início da operação não são acompanhados. Mudanças de foco, resolução, layout ou monitor podem alterar o resultado.
- A gravação pode incluir cliques na interface. Não reproduza macros de origem desconhecida.
- A instalação descrita acima concede permissões ao usuário; o aplicativo não as configura sozinho.

## Desinstalar

Remova o aplicativo instalado e o atalho:

```bash
rm -f "${XDG_DATA_HOME:-$HOME/.local/share}/applications/tiny-macro-linux.desktop"
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/tiny-macro-linux"
```

Se você criou as regras de permissão deste tutorial e não precisa delas para outros programas, remova-as e retire seu usuário do grupo `input`:

```bash
sudo rm -f /etc/modules-load.d/tiny-macro-uinput.conf /etc/udev/rules.d/70-tiny-macro-uinput.rules
sudo gpasswd -d "$USER" input
sudo udevadm control --reload-rules
```

Encerre a sessão e entre novamente; reinicie o computador para garantir que as mudanças de acesso sejam aplicadas. Remover o usuário do grupo pode afetar outros programas que dependam dele.

As configurações dos atalhos ficam em `${XDG_CONFIG_HOME:-$HOME/.config}/tiny-macro-linux/atalhos.json`. Exclua a pasta `tiny-macro-linux` desse diretório se também quiser apagar as configurações.
