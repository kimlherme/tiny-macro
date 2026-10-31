use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as VBox, Button, DropDown, FileChooserAction,
    FileChooserNative, Label, Orientation, ResponseType, ToggleButton,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Event {
    micros: u64,
    kind: u16,
    code: u16,
    value: i32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Macro {
    version: u32,
    events: Vec<Event>,
}
impl Macro {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.events.len() > 1_000_000 {
            return Err("Versão ou tamanho inválido".into());
        }
        let mut last = 0;
        for e in &self.events {
            if e.micros < last
                || e.micros > 86_400_000_000
                || !matches!(e.kind, 0..=2)
                || (e.kind == 0 && (e.code != 0 || e.value != 0))
                || (e.kind == 1
                    && (e.code > 767 || !(0..=2).contains(&e.value) || matches!(e.code, 70 | 119)))
                || (e.kind == 2 && e.code > 15)
            {
                return Err("Evento inválido".into());
            }
            last = e.micros;
        }
        Ok(())
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Input {
    time: libc::timeval,
    kind: u16,
    code: u16,
    value: i32,
}
fn read(f: &mut File) -> io::Result<Option<Input>> {
    let mut e = Input::default();
    let b = unsafe {
        std::slice::from_raw_parts_mut((&mut e as *mut Input).cast(), std::mem::size_of::<Input>())
    };
    match f.read(b) {
        Ok(n) if n == b.len() => Ok(Some(e)),
        Ok(0) => Ok(None),
        Ok(_) => Err(io::ErrorKind::UnexpectedEof.into()),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(e) => Err(e),
    }
}
fn inputs() -> io::Result<Vec<File>> {
    let mut v = Vec::new();
    for e in fs::read_dir("/dev/input")? {
        let p = e?.path();
        if p.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        {
            if let Ok(f) = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(p)
            {
                v.push(f)
            }
        }
    }
    if v.is_empty() {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Nenhum dispositivo de entrada acessível",
        ))
    } else {
        Ok(v)
    }
}
fn halt(e: &Input) -> bool {
    e.kind == 1 && e.value == 1 && matches!(e.code, 70 | 119)
}
fn capture(
    mut files: Vec<File>,
    stop: Arc<AtomicBool>,
    keys: [u16; 3],
) -> Result<Vec<Event>, String> {
    let start = Instant::now();
    let mut out = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        for f in &mut files {
            loop {
                match read(f) {
                    Ok(Some(e)) => {
                        if halt(&e) || (e.kind == 1 && e.value == 1 && e.code == keys[1]) {
                            stop.store(true, Ordering::Relaxed);
                            break;
                        }
                        if (e.kind == 1 && !matches!(e.code, 70 | 119) && !keys.contains(&e.code))
                            || e.kind == 2
                            || (e.kind == 0 && e.code == 0 && e.value == 0)
                        {
                            out.push(Event {
                                micros: start.elapsed().as_micros() as u64,
                                kind: e.kind,
                                code: e.code,
                                value: e.value,
                            });
                            if out.len() >= 1_000_000 {
                                stop.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
    Ok(out)
}
#[repr(C)]
struct Id {
    bus: u16,
    vendor: u16,
    product: u16,
    version: u16,
}
#[repr(C)]
struct Setup {
    id: Id,
    name: [u8; 80],
    ff: u32,
}
struct Device(File);
impl Device {
    fn new() -> io::Result<Self> {
        let f = OpenOptions::new().write(true).open("/dev/uinput")?;
        let fd = f.as_raw_fd();
        let set = |req: libc::c_ulong, val: libc::c_int| {
            if unsafe { libc::ioctl(fd, req, val) } < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        };
        for t in [1, 2] {
            set(0x40045564, t)?
        }
        for k in 0..=767 {
            set(0x40045565, k)?
        }
        for k in 0..=15 {
            set(0x40045566, k)?
        }
        let mut s = Setup {
            id: Id {
                bus: 3,
                vendor: 1,
                product: 1,
                version: 1,
            },
            name: [0; 80],
            ff: 0,
        };
        s.name[..16].copy_from_slice(b"Tiny Macro Linux");
        if unsafe { libc::ioctl(fd, 0x405c5503, &s) } < 0 {
            return Err(io::Error::last_os_error());
        }
        set(0x5501, 0)?;
        Ok(Self(f))
    }
    fn send(&mut self, e: &Event) -> io::Result<()> {
        let v = Input {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            kind: e.kind,
            code: e.code,
            value: e.value,
        };
        let b = unsafe {
            std::slice::from_raw_parts((&v as *const Input).cast(), std::mem::size_of::<Input>())
        };
        self.0.write_all(b)
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            libc::ioctl(self.0.as_raw_fd(), 0x5502);
        }
    }
}
fn playback(
    events: Vec<Event>,
    mut files: Vec<File>,
    stop: Arc<AtomicBool>,
) -> Result<Vec<Event>, String> {
    let mut dev = Device::new().map_err(|e| format!("/dev/uinput: {e}"))?;
    thread::sleep(Duration::from_millis(200));
    let start = Instant::now();
    for e in events {
        while start.elapsed().as_micros() < (e.micros as u128) {
            for f in &mut files {
                loop {
                    match read(f) {
                        Ok(Some(v)) if halt(&v) => {
                            stop.store(true, Ordering::Relaxed);
                            break;
                        }
                        Ok(Some(_)) => {}
                        Ok(None) => break,
                        Err(e) => return Err(e.to_string()),
                    }
                }
            }
            if stop.load(Ordering::Relaxed) {
                return Ok(vec![]);
            }
            thread::sleep(Duration::from_millis(2));
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }
        dev.send(&e).map_err(|e| e.to_string())?;
    }
    Ok(vec![])
}
fn config_path() -> std::path::PathBuf {
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    root.join("tiny-macro-linux").join("atalhos.json")
}
fn load_keys() -> [u16; 3] {
    let allowed = [64, 65, 66, 67, 68, 87, 88];
    fs::read(config_path())
        .ok()
        .and_then(|v| serde_json::from_slice::<[u16; 3]>(&v).ok())
        .filter(|k| {
            k.iter().all(|x| allowed.contains(x)) && k[0] != k[1] && k[0] != k[2] && k[1] != k[2]
        })
        .unwrap_or([64, 65, 66])
}
fn save_keys(keys: [u16; 3]) -> io::Result<()> {
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, serde_json::to_vec(&keys).map_err(io::Error::other)?)
}
struct State {
    events: Vec<Event>,
    stop: Option<Arc<AtomicBool>>,
    recording: bool,
    keys: [u16; 3],
}
fn main() {
    let app = Application::builder()
        .application_id("local.tinymacro.Linux")
        .build();
    app.connect_activate(|app| {
        let win = ApplicationWindow::builder()
            .application(app)
            .title("Tiny Macro Linux")
            .default_width(480)
            .default_height(130)
            .build();
        let root = VBox::new(Orientation::Vertical, 8);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);
        let heading = Label::new(Some("TINY MACRO  ·  LINUX"));
        heading.add_css_class("title-4");
        heading.set_xalign(0.0);
        root.append(&heading);
        let label = Label::new(Some("Pronto para gravar"));
        label.set_xalign(0.0);
        label.set_wrap(true);
        root.append(&label);
        let row = VBox::new(Orientation::Horizontal, 6);
        let record = Button::with_label("Gravar");
        let play = Button::with_label("Reproduzir");
        let stop = Button::with_label("Parar");
        let open = Button::with_label("Abrir");
        let save = Button::with_label("Salvar");
        let config = Button::with_label("⚙ Atalhos");
        record.add_css_class("suggested-action");
        stop.add_css_class("destructive-action");
        for b in [&record, &play, &stop, &open, &save, &config] {
            row.append(b)
        }
        root.append(&row);
        let overlay = ToggleButton::with_label("Fixar sobre janelas");
        overlay.set_tooltip_text(Some(
            "Janela flutuante em todos os espaços de trabalho no Hyprland",
        ));
        root.append(&overlay);
        let hint = Label::new(Some("Pause ou Scroll Lock para parar"));
        hint.add_css_class("dim-label");
        hint.set_xalign(0.0);
        root.append(&hint);
        win.set_child(Some(&root));
        let state = Rc::new(RefCell::new(State {
            events: vec![],
            stop: None,
            recording: false,
            keys: load_keys(),
        }));
        {
            let label = label.clone();
            overlay.connect_toggled(move |button| {
                let selector = "class:^(local[.]tinymacro[.]Linux)$";
                let run = |expression: &str| {
                    std::process::Command::new("hyprctl")
                        .args(["dispatch", expression])
                        .output()
                        .map(|o| {
                            o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "ok"
                        })
                        .unwrap_or(false)
                };
                if button.is_active() {
                    if !run(&format!("hl.dsp.window.float({{window=\"{selector}\", action=\"on\"}})"))
                        || !run(&format!("hl.dsp.window.pin({{window=\"{selector}\", action=\"on\"}})")) {
                        label.set_text("Não foi possível fixar a janela no Hyprland.");
                        return;
                    }
                    let _ = run(&format!("hl.dsp.window.resize({{window=\"{selector}\", x=520, y=184, relative=false}})"));
                    let _ = run(&format!("hl.dsp.window.alter_zorder({{window=\"{selector}\", mode=\"top\"}})"));
                    label.set_text("Janela flutuante fixada nos espaços de trabalho.");
                } else if !run(&format!("hl.dsp.window.pin({{window=\"{selector}\", action=\"off\"}})")) {
                    label.set_text("Não foi possível desafixar a janela.");
                } else {
                    label.set_text("Janela desafixada.");
                }
            });
        }
        {
            let state = state.clone();
            let win = win.clone();
            let label = label.clone();
            config.connect_clicked(move |_| {
                if state.borrow().stop.is_some() { label.set_text("Pare a operação antes de configurar atalhos."); return; }
                let dialog = gtk::Window::builder().title("Configurar atalhos").transient_for(&win).modal(true).default_width(340).build();
                let layout = VBox::new(Orientation::Vertical, 10);
                layout.set_margin_top(16); layout.set_margin_bottom(16); layout.set_margin_start(16); layout.set_margin_end(16);
                let names = ["F6", "F7", "F8", "F9", "F10", "F11", "F12"];
                let codes = [64u16, 65, 66, 67, 68, 87, 88];
                let current = state.borrow().keys;
                let mut choices = Vec::new();
                for (i, title) in ["Iniciar gravação", "Parar gravação", "Iniciar reprodução"].iter().enumerate() {
                    let text = Label::new(Some(title)); text.set_xalign(0.0); layout.append(&text);
                    let choice = DropDown::from_strings(&names);
                    choice.set_selected(codes.iter().position(|v| *v == current[i]).unwrap_or(i) as u32);
                    layout.append(&choice); choices.push(choice);
                }
                let message = Label::new(Some("Pause e Scroll Lock continuam como parada de emergência.")); message.set_wrap(true); layout.append(&message);
                let save = Button::with_label("Salvar atalhos"); save.add_css_class("suggested-action"); layout.append(&save);
                let state = state.clone(); let dialog_copy = dialog.clone(); let label = label.clone();
                save.connect_clicked(move |_| {
                    let keys = [codes[choices[0].selected() as usize], codes[choices[1].selected() as usize], codes[choices[2].selected() as usize]];
                    if keys[0] == keys[1] || keys[0] == keys[2] || keys[1] == keys[2] { message.set_text("Escolha uma tecla diferente para cada ação."); return; }
                    if let Err(e) = save_keys(keys) { message.set_text(&format!("Erro ao salvar: {e}")); return; }
                    state.borrow_mut().keys = keys; label.set_text("Atalhos salvos."); dialog_copy.close();
                });
                dialog.set_child(Some(&layout)); dialog.present();
            });
        }
        let (hot_tx, hot_rx) = mpsc::channel::<u16>();
        thread::spawn(move || {
            let mut files = match inputs() { Ok(files) => files, Err(_) => return };
            loop {
                for f in &mut files { loop { match read(f) { Ok(Some(e)) if e.kind == 1 && e.value == 1 => { if hot_tx.send(e.code).is_err() { return; } }, Ok(Some(_)) => {}, _ => break } } }
                thread::sleep(Duration::from_millis(10));
            }
        });
        {
            let state = state.clone(); let record = record.clone(); let play = play.clone(); let stop = stop.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(20), move || {
                while let Ok(code) = hot_rx.try_recv() {
                    let s = state.borrow();
                    if s.stop.is_some() { if s.recording && code == s.keys[1] { drop(s); stop.emit_clicked(); } }
                    else if code == s.keys[0] { drop(s); record.emit_clicked(); }
                    else if code == s.keys[2] { drop(s); play.emit_clicked(); }
                }
                gtk::glib::ControlFlow::Continue
            });
        }
        let (tx, rx) = mpsc::channel::<Result<Vec<Event>, String>>();
        for (button, rec) in [(&record, true), (&play, false)] {
            let s = state.clone();
            let l = label.clone();
            let tx = tx.clone();
            button.connect_clicked(move |_| {
                if s.borrow().stop.is_some() {
                    return;
                }
                if !rec && s.borrow().events.is_empty() {
                    l.set_text("Nenhuma macro carregada.");
                    return;
                }
                match inputs() {
                    Err(e) => l.set_text(&format!("Entrada indisponível: {e}")),
                    Ok(files) => {
                        let flag = Arc::new(AtomicBool::new(false));
                        let mut state = s.borrow_mut();
                        state.stop = Some(flag.clone());
                        state.recording = rec;
                        let events = state.events.clone();
                        let keys = state.keys;
                        let tx = tx.clone();
                        l.set_text(if rec {
                            "Gravando…"
                        } else {
                            "Reproduzindo…"
                        });
                        thread::spawn(move || {
                            let r = if rec {
                                capture(files, flag, keys)
                            } else {
                                playback(events, files, flag)
                            };
                            let _ = tx.send(r);
                        });
                    }
                }
            });
        }
        {
            let s = state.clone();
            stop.connect_clicked(move |_| {
                if let Some(f) = &s.borrow().stop {
                    f.store(true, Ordering::Relaxed)
                }
            });
        }
        for (button, action) in [
            (&open, FileChooserAction::Open),
            (&save, FileChooserAction::Save),
        ] {
            let s = state.clone();
            let l = label.clone();
            let w = win.clone();
            button.connect_clicked(move |_| {
                if s.borrow().stop.is_some() {
                    l.set_text("Pare a macro primeiro.");
                    return;
                }
                let d = FileChooserNative::new(
                    Some("Macro"),
                    Some(&w),
                    action,
                    Some("Selecionar"),
                    Some("Cancelar"),
                );
                if action == FileChooserAction::Save {
                    d.set_current_name("macro.json")
                }
                let s = s.clone();
                let l = l.clone();
                d.connect_response(move |d, r| {
                    if r != ResponseType::Accept {
                        return;
                    }
                    let Some(path) = d.file().and_then(|f| f.path()) else {
                        return;
                    };
                    let result: Result<String, String> = (|| {
                        if action == FileChooserAction::Open {
                            let f = File::open(path).map_err(|e| e.to_string())?;
                            if f.metadata().map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
                                return Err("Arquivo grande demais".into());
                            }
                            let m: Macro = serde_json::from_reader(f).map_err(|e| e.to_string())?;
                            m.validate()?;
                            let n = m.events.len();
                            s.borrow_mut().events = m.events;
                            Ok(format!("Macro aberta: {n} eventos"))
                        } else {
                            let m = Macro {
                                version: 1,
                                events: s.borrow().events.clone(),
                            };
                            m.validate()?;
                            let f = File::create(path).map_err(|e| e.to_string())?;
                            serde_json::to_writer_pretty(f, &m).map_err(|e| e.to_string())?;
                            Ok("Macro salva.".into())
                        }
                    })();
                    l.set_text(&result.unwrap_or_else(|e| format!("Erro: {e}")));
                });
                d.show();
            });
        }
        {
            let s = state.clone();
            let l = label.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(50), move || {
                if let Ok(r) = rx.try_recv() {
                    let mut s = s.borrow_mut();
                    s.stop = None;
                    match r {
                        Ok(events) if s.recording => {
                            let n = events.len();
                            s.events = events;
                            l.set_text(&format!("Gravação concluída: {n} eventos."))
                        }
                        Ok(_) => l.set_text("Reprodução encerrada."),
                        Err(e) => l.set_text(&e),
                    }
                }
                gtk::glib::ControlFlow::Continue
            });
        }
        {
            let s = state.clone();
            win.connect_close_request(move |_| {
                if let Some(f) = &s.borrow().stop {
                    f.store(true, Ordering::Relaxed)
                }
                gtk::glib::Propagation::Proceed
            });
        }
        win.present();
    });
    app.run();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validate() {
        let mut m = Macro {
            version: 1,
            events: vec![Event {
                micros: 1,
                kind: 1,
                code: 30,
                value: 1,
            }],
        };
        assert!(m.validate().is_ok());
        m.events[0].code = 119;
        assert!(m.validate().is_err());
    }
    #[test]
    fn roundtrip() {
        let m = Macro {
            version: 1,
            events: vec![],
        };
        assert!(
            serde_json::from_str::<Macro>(&serde_json::to_string(&m).unwrap())
                .unwrap()
                .validate()
                .is_ok()
        );
    }
}
