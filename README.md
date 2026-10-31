# Tiny Macro Linux

Um gravador de ações de teclado e mouse para Linux, inspirado no TinyTask. Grave uma sequência, reproduza e salve suas macros no computador.

> **Projeto em desenvolvimento:** gravação e reprodução básicas, incluindo cliques e rolagem, foram testadas no Hyprland. A janela abriu no KWin Wayland, mas a gravação e a reprodução ainda não foram validadas no KDE.

## Instalação fácil — Omarchy e Arch Linux

Ainda não há instalador gráfico. Estes passos compilam o aplicativo no seu computador e adicionam um atalho ao menu.

### 1. Instale as dependências

Abra um terminal e execute:

```bash
sudo pacman -S --needed git rust gtk4 pkgconf
```

### 2. Baixe e compile

```bash
git clone https://github.com/kimlherme/tiny-macro.git
cd tiny-macro
cargo build --release --locked
```

### 3. Configure o acesso ao teclado e ao mouse

O aplicativo precisa ler `/dev/input/event*` para gravar e escrever em `/dev/uinput` para reproduzir. **O grupo `input` dá acesso amplo aos dispositivos de entrada**, permitindo que programas executados pelo seu usuário leiam teclas e simulem entrada. Faça esta configuração somente em uma conta confiável.

```bash
sudo usermod -aG input "$USER"
printf '%s\n' uinput | sudo tee /etc/modules-load.d/tiny-macro-uinput.conf
printf '%s\n' 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/70-tiny-macro-uinput.rules
sudo modprobe uinput
sudo udevadm control --reload-rules
sudo udevadm trigger --action=change /dev/uinput
```

**Encerre a sessão e entre novamente** para que a participação no grupo seja aplicada. Depois, confira:

```bash
id -nG
ls -l /dev/uinput /dev/input/event*
```

O grupo `input` deve aparecer na primeira linha, e `/dev/uinput` deve permitir escrita para esse grupo. Se não aparecer, reinicie o computador e confira novamente. Em outros ambientes ou sistemas de permissões, pode ser necessário consultar a documentação da distribuição.

### 4. Abra o aplicativo

Dentro da pasta clonada:

```bash
./target/release/tiny-macro-linux
```

Para adicionar ao menu de aplicativos, execute **dentro da pasta do projeto**:

```bash
mkdir -p "$HOME/.local/share/applications"
printf '[Desktop Entry]\nType=Application\nName=Tiny Macro Linux\nComment=Gravar e reproduzir teclado e mouse\nExec=%s/target/release/tiny-macro-linux\nIcon=input-keyboard\nTerminal=false\nCategories=Utility;\nKeywords=macro;tinytask;teclado;mouse;\n' "$PWD" > "$HOME/.local/share/applications/tiny-macro-linux.desktop"
```

Procure por **Tiny Macro Linux** no launcher. Se não aparecer de imediato, feche e reabra a busca. O atalho usa o caminho atual do projeto; se mover a pasta, execute o comando novamente dentro do novo local. Para caminhos com espaços, use uma pasta sem espaços para esta versão do tutorial.

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

Apague a pasta clonada e remova o atalho, se criado:

```bash
rm -f "$HOME/.local/share/applications/tiny-macro-linux.desktop"
```

Se você criou as regras de permissão deste tutorial e não precisa delas para outros programas, remova-as e retire seu usuário do grupo `input`:

```bash
sudo rm -f /etc/modules-load.d/tiny-macro-uinput.conf /etc/udev/rules.d/70-tiny-macro-uinput.rules
sudo gpasswd -d "$USER" input
sudo udevadm control --reload-rules
```

Encerre a sessão e entre novamente; reinicie o computador para garantir que as mudanças de acesso sejam aplicadas. Remover o usuário do grupo pode afetar outros programas que dependam dele.

As configurações dos atalhos ficam em `${XDG_CONFIG_HOME:-$HOME/.config}/tiny-macro-linux/atalhos.json`. Exclua a pasta `tiny-macro-linux` desse diretório se também quiser apagar as configurações.
