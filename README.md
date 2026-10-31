# Tiny Macro Linux

Aplicativo local para gravar e reproduzir eventos de teclado e mouse em Linux/Hyprland. Não inicia gravação automaticamente e não transmite dados. A macro permanece em memória até ser salva como JSON pelo usuário. Pressione **Pause** ou **Scroll Lock** ou clique em **Parar** para encerrar a gravação ou reprodução.

## Compilar e executar

Requer Rust, bibliotecas de desenvolvimento GTK 4 e Linux com evdev e uinput:

```sh
cargo run --release
```

Clique em **Gravar**, execute as ações e pare; use **Reproduzir**, **Salvar** ou **Abrir**. A reprodução usa temporização relativa original e é executada uma vez. Arquivos são locais e podem conter tudo que foi digitado: proteja-os como informação sensível.

## Permissões e limites

A gravação e o atalho global precisam de leitura dos dispositivos `/dev/input/event*`; reprodução também precisa de escrita em `/dev/uinput`. Esses acessos permitem observar e simular entrada de toda a sessão e devem ser concedidos explicitamente pelo administrador apenas a usuários confiáveis. O programa não altera permissões, não usa sudo nem instala regras de udev. Sem acesso ele mostra o erro. Pode ser necessário carregar o módulo `uinput` e configurar acesso específico por udev/logind; as medidas exatas dependem da distribuição. Reinicie o app após mudar permissões.

O programa captura eventos de dispositivos que estão acessíveis quando começa; não acompanha dispositivos conectados depois. Apenas eventos de teclado, botões, movimento e rolagem relativos são suportados: toque, mesa digitalizadora e posicionamento absoluto não são reproduzidos. O aplicativo não bloqueia a propagação das teclas de parada. Atalhos globais configuráveis usam F6 a F12: por padrão F6 inicia a gravação, F7 para a gravação e F8 inicia a reprodução; Pause e Scroll Lock continuam disponíveis para parar. Configure pelo botão Atalhos. Os atalhos escolhidos também são entregues ao aplicativo em foco. A captura pode incluir cliques na própria interface. Mudanças de janela, resolução, foco, layout e escala podem alterar o resultado. Não execute macros de arquivos que não conhece: elas podem produzir comandos e cliques na sua sessão.

O formato JSON é validado na abertura (versão, limites, ordem e tipos de eventos); não é criptografado. Teste em um ambiente controlado antes de usá-lo em tarefas reais.
