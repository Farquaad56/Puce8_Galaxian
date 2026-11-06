//! Débogueur : fenêtre CPU (registres, désassemblage, pas à pas, points d'arrêt) et fenêtre
//! Mémoire à onglets (bus 64 Ko, ROM programme, RAM, VRAM, RAM objets, E/S, PROM, ROM graphique).
//! Toutes les lectures passent par `Board::peek` : aucun effet de bord sur l'émulation.

use eframe::egui::{self, text::LayoutJob, text::TextFormat, Color32, FontId};
use galaxian_core::{Machine, RomSet};

/// Nombre d'images pendant lesquelles un octet modifié reste surligné.
const HIGHLIGHT_FRAMES: u64 = 60;
/// Lignes de désassemblage affichées.
const DIS_LINES: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemTab {
    Bus,
    Program,
    Ram,
    Vram,
    ObjRam,
    Io,
    Prom,
    Gfx,
}

impl MemTab {
    const ALL: [MemTab; 8] = [
        MemTab::Bus,
        MemTab::Program,
        MemTab::Ram,
        MemTab::Vram,
        MemTab::ObjRam,
        MemTab::Io,
        MemTab::Prom,
        MemTab::Gfx,
    ];

    fn label(self) -> &'static str {
        match self {
            MemTab::Bus => "Bus 64 Ko",
            MemTab::Program => "ROM 0000-3FFF",
            MemTab::Ram => "RAM 4000-47FF",
            MemTab::Vram => "VRAM 5000-57FF",
            MemTab::ObjRam => "Objets 5800-5FFF",
            MemTab::Io => "E/S & latches",
            MemTab::Prom => "PROM couleurs",
            MemTab::Gfx => "ROM graphique",
        }
    }
}

/// Action ponctuelle demandée à l'application.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    None,
    StepInsn,
    StepFrame,
}

pub struct Debugger {
    pub show_cpu: bool,
    pub show_mem: bool,
    pub paused: bool,
    cmd: Cmd,
    breakpoints: Vec<u16>,
    bp_text: String,
    last_break: Option<u16>,
    follow_pc: bool,
    dis_text: String,
    dis_addr: u16,
    tab: MemTab,
    goto_text: String,
    scroll_to: Option<usize>,
    snap: Vec<u8>,
    changed: Vec<u64>,
    snap_tab: Option<MemTab>,
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {
            show_cpu: false,
            show_mem: false,
            paused: false,
            cmd: Cmd::None,
            breakpoints: Vec::new(),
            bp_text: String::new(),
            last_break: None,
            follow_pc: true,
            dis_text: String::new(),
            dis_addr: 0,
            tab: MemTab::Bus,
            goto_text: String::new(),
            scroll_to: None,
            snap: Vec::new(),
            changed: Vec::new(),
            snap_tab: None,
        }
    }

    pub fn take_cmd(&mut self) -> Cmd {
        std::mem::replace(&mut self.cmd, Cmd::None)
    }

    pub fn request(&mut self, cmd: Cmd) {
        if self.paused {
            self.cmd = cmd;
        }
    }

    pub fn breakpoints(&self) -> &[u16] {
        &self.breakpoints
    }

    /// Appelé par l'application quand l'exécution s'arrête sur un point d'arrêt.
    pub fn hit_breakpoint(&mut self, pc: u16) {
        self.paused = true;
        self.last_break = Some(pc);
        self.show_cpu = true;
    }

    fn toggle_breakpoint(&mut self, addr: u16) {
        if let Some(i) = self.breakpoints.iter().position(|&a| a == addr) {
            self.breakpoints.remove(i);
        } else {
            self.breakpoints.push(addr);
            self.breakpoints.sort_unstable();
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, machine: &Machine, rom: &RomSet, frame: u64) {
        if self.show_cpu {
            self.cpu_window(ctx, machine);
        }
        if self.show_mem {
            self.mem_window(ctx, machine, rom, frame);
        }
    }

    // ------------------------------------------------------------------ CPU

    fn cpu_window(&mut self, ctx: &egui::Context, machine: &Machine) {
        let mut open = self.show_cpu;
        egui::Window::new("Débogueur CPU")
            .open(&mut open)
            .default_width(520.0)
            .show(ctx, |ui| {
                self.cpu_controls(ui);
                ui.separator();
                registers(ui, machine);
                ui.separator();
                self.disassembly(ui, machine);
                ui.separator();
                self.breakpoint_list(ui);
                ui.separator();
                stack(ui, machine);
            });
        self.show_cpu = open;
    }

    fn cpu_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let label = if self.paused {
                "▶ Reprendre"
            } else {
                "⏸ Pause"
            };
            if ui.button(label).clicked() {
                self.paused = !self.paused;
                self.last_break = None;
            }
            ui.add_enabled_ui(self.paused, |ui| {
                if ui.button("Instruction (F10)").clicked() {
                    self.cmd = Cmd::StepInsn;
                }
                if ui.button("Image (F11)").clicked() {
                    self.cmd = Cmd::StepFrame;
                }
            });
        });
        if let Some(pc) = self.last_break {
            ui.colored_label(
                Color32::LIGHT_RED,
                format!("Point d'arrêt atteint en {pc:04X}"),
            );
        }
    }

    fn disassembly(&mut self, ui: &mut egui::Ui, machine: &Machine) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.follow_pc, "Suivre PC");
            ui.label("Adresse");
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.dis_text)
                    .desired_width(56.0)
                    .font(egui::TextStyle::Monospace),
            );
            if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.button("Aller").clicked()
            {
                if let Some(a) = parse_hex16(&self.dis_text) {
                    self.dis_addr = a;
                    self.follow_pc = false;
                }
            }
        });
        let pc = machine.cpu.regs.pc;
        let mut addr = if self.follow_pc { pc } else { self.dis_addr };
        let mono = FontId::monospace(13.0);
        let mut toggled = None;
        for _ in 0..DIS_LINES {
            let (text, len) = machine.disassemble_at(addr);
            let bytes: String = (0..len)
                .map(|k| {
                    format!(
                        "{:02X} ",
                        machine.board.peek(addr.wrapping_add(u16::from(k)))
                    )
                })
                .collect();
            let is_pc = addr == pc;
            let has_bp = self.breakpoints.contains(&addr);
            let mut job = LayoutJob::default();
            let color = if is_pc {
                Color32::YELLOW
            } else {
                Color32::LIGHT_GRAY
            };
            let bg = if is_pc {
                Color32::from_rgb(60, 60, 0)
            } else {
                Color32::TRANSPARENT
            };
            let mark = format!(
                "{}{} ",
                if has_bp { "●" } else { " " },
                if is_pc { "▶" } else { " " }
            );
            let fmt = |c: Color32| TextFormat {
                font_id: mono.clone(),
                color: c,
                background: bg,
                ..Default::default()
            };
            job.append(&mark, 0.0, fmt(Color32::LIGHT_RED));
            job.append(&format!("{addr:04X}  "), 0.0, fmt(color));
            job.append(&format!("{bytes:<12}"), 0.0, fmt(Color32::GRAY));
            job.append(&text, 0.0, fmt(color));
            let r = ui.add(egui::Label::new(job).sense(egui::Sense::click()));
            if r.clicked() {
                toggled = Some(addr);
            }
            addr = addr.wrapping_add(u16::from(len.max(1)));
        }
        if let Some(a) = toggled {
            self.toggle_breakpoint(a);
        }
        ui.small("Clic sur une ligne : point d'arrêt (●).");
    }

    fn breakpoint_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Points d'arrêt (PC)");
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.bp_text)
                    .desired_width(56.0)
                    .font(egui::TextStyle::Monospace),
            );
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Ajouter").clicked() || enter {
                if let Some(a) = parse_hex16(&self.bp_text) {
                    if !self.breakpoints.contains(&a) {
                        self.toggle_breakpoint(a);
                    }
                    self.bp_text.clear();
                }
            }
        });
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            for &a in &self.breakpoints {
                if ui.button(format!("{a:04X} ✕")).clicked() {
                    remove = Some(a);
                }
            }
            if self.breakpoints.is_empty() {
                ui.weak("aucun");
            }
        });
        if let Some(a) = remove {
            self.toggle_breakpoint(a);
        }
    }

    // ------------------------------------------------------------------ mémoire

    /// Octets affichés pour un onglet : (adresse de base, contenu).
    fn region(tab: MemTab, machine: &Machine, rom: &RomSet) -> (u32, Vec<u8>) {
        let b = &machine.board;
        let peek_range = |start: u32, len: u32| -> Vec<u8> {
            (start..start + len).map(|a| b.peek(a as u16)).collect()
        };
        match tab {
            MemTab::Bus => (0, peek_range(0, 0x10000)),
            MemTab::Program => (0, peek_range(0, 0x4000)),
            MemTab::Ram => (0x4000, peek_range(0x4000, 0x400)),
            MemTab::Vram => (0x5000, peek_range(0x5000, 0x400)),
            MemTab::ObjRam => (0x5800, peek_range(0x5800, 0x100)),
            MemTab::Io => (0x6000, Vec::new()),
            MemTab::Prom => (0, rom.prom.to_vec()),
            MemTab::Gfx => (0, rom.gfx.clone()),
        }
    }

    fn mem_window(&mut self, ctx: &egui::Context, machine: &Machine, rom: &RomSet, frame: u64) {
        let mut open = self.show_mem;
        egui::Window::new("Mémoire")
            .open(&mut open)
            .default_size([620.0, 520.0])
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for t in MemTab::ALL {
                        if ui.selectable_label(self.tab == t, t.label()).clicked() {
                            self.tab = t;
                            self.scroll_to = Some(0);
                        }
                    }
                });
                ui.separator();
                if self.tab == MemTab::Io {
                    io_view(ui, machine);
                    return;
                }
                let (base, data) = Self::region(self.tab, machine, rom);
                self.track(&data, frame);
                ui.horizontal(|ui| {
                    ui.label("Aller à");
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.goto_text)
                            .desired_width(56.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("OK").clicked() || enter {
                        if let Some(a) = parse_hex16(&self.goto_text) {
                            let off = (u32::from(a)).saturating_sub(base) as usize;
                            self.scroll_to = Some(off / 16);
                        }
                    }
                    ui.weak(format!("{} octets, base {base:04X}", data.len()));
                });
                if self.tab == MemTab::ObjRam {
                    objram_decode(ui, &data);
                    ui.separator();
                }
                let scroll = self.scroll_to.take();
                hex_view(ui, base, &data, &self.changed, frame, scroll);
            });
        self.show_mem = open;
    }

    /// Suit les modifications octet par octet pour le surlignage.
    fn track(&mut self, data: &[u8], frame: u64) {
        if self.snap_tab != Some(self.tab) || self.snap.len() != data.len() {
            self.snap = data.to_vec();
            self.changed = vec![0; data.len()];
            self.snap_tab = Some(self.tab);
            return;
        }
        for (i, (&new, old)) in data.iter().zip(self.snap.iter_mut()).enumerate() {
            if new != *old {
                *old = new;
                self.changed[i] = frame.max(1);
            }
        }
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Debugger::new()
    }
}

// ---------------------------------------------------------------------- vues

fn parse_hex16(s: &str) -> Option<u16> {
    let t = s
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .trim_start_matches('$');
    u16::from_str_radix(t, 16).ok()
}

fn mono(text: String) -> egui::RichText {
    egui::RichText::new(text).monospace()
}

fn flags_text(f: u8) -> String {
    const NAMES: [char; 8] = ['S', 'Z', 'Y', 'H', 'X', 'P', 'N', 'C'];
    NAMES
        .iter()
        .enumerate()
        .map(|(i, &n)| if f & (0x80 >> i) != 0 { n } else { '-' })
        .collect()
}

fn registers(ui: &mut egui::Ui, machine: &Machine) {
    let r = &machine.cpu.regs;
    let b = &machine.board;
    let pair = |h: u8, l: u8| format!("{h:02X}{l:02X}");
    egui::Grid::new("regs").num_columns(4).show(ui, |ui| {
        ui.label(mono(format!("AF  {}", pair(r.a, r.f))));
        ui.label(mono(format!("BC  {}", pair(r.b, r.c))));
        ui.label(mono(format!("DE  {}", pair(r.d, r.e))));
        ui.label(mono(format!("HL  {}", pair(r.h, r.l))));
        ui.end_row();
        ui.label(mono(format!("AF' {}", pair(r.a2, r.f2))));
        ui.label(mono(format!("BC' {}", pair(r.b2, r.c2))));
        ui.label(mono(format!("DE' {}", pair(r.d2, r.e2))));
        ui.label(mono(format!("HL' {}", pair(r.h2, r.l2))));
        ui.end_row();
        ui.label(mono(format!("IX  {:04X}", r.ix)));
        ui.label(mono(format!("IY  {:04X}", r.iy)));
        ui.label(mono(format!("SP  {:04X}", r.sp)));
        ui.label(mono(format!("PC  {:04X}", r.pc)));
        ui.end_row();
        ui.label(mono(format!("I   {:02X}   R {:02X}", r.i, r.r)));
        ui.label(mono(format!("IM  {}", r.im)));
        ui.label(mono(format!(
            "IFF {}{}",
            u8::from(r.iff1),
            u8::from(r.iff2)
        )));
        ui.label(mono(format!("WZ  {:04X}", r.wz)));
        ui.end_row();
    });
    ui.label(mono(format!(
        "Drapeaux {}   {}",
        flags_text(r.f),
        if r.halted { "HALT" } else { "" }
    )));
    ui.label(mono(format!(
        "Image {}  ligne {:3}  col {:3}  T/image {:5}  T total {}",
        b.frames_completed,
        b.vpos(),
        b.hpos(),
        b.frame_t(),
        b.tstate
    )));
    ui.label(mono(format!(
        "NMI autorisée {}  levée {}  watchdog {}",
        u8::from(b.nmi_enabled()),
        u8::from(b.nmi_pending()),
        b.watchdog_count()
    )));
}

fn stack(ui: &mut egui::Ui, machine: &Machine) {
    let sp = machine.cpu.regs.sp;
    ui.label("Pile");
    let mut line = String::new();
    for k in 0..8u16 {
        let a = sp.wrapping_add(k * 2);
        let lo = machine.board.peek(a);
        let hi = machine.board.peek(a.wrapping_add(1));
        line.push_str(&format!("{a:04X}:{hi:02X}{lo:02X}  "));
        if k % 4 == 3 {
            ui.label(mono(std::mem::take(&mut line)));
        }
    }
}

fn io_view(ui: &mut egui::Ui, machine: &Machine) {
    let b = &machine.board;
    let bits = |v: u8| format!("{v:08b}");
    let on = |v: bool| if v { "1" } else { "0" };
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Entrées");
        for (addr, name) in [(0x6000u16, "IN0"), (0x6800, "IN1"), (0x7000, "IN2 (DIP)")] {
            let v = b.peek(addr);
            ui.label(mono(format!("{addr:04X} {name:<10} {v:02X}  {}", bits(v))));
        }
        ui.separator();
        ui.heading("Latches 6000-6007 (écriture)");
        ui.label(mono(format!(
            "6000 lampe start1 {}   6001 lampe start2 {}",
            on(b.start_lamp[0]),
            on(b.start_lamp[1])
        )));
        ui.label(mono(format!(
            "6002 verrou pièce {}   6003 compteur pièce {}",
            on(b.coin_lockout_raw),
            on(b.coin_counter)
        )));
        ui.label(mono(format!(
            "6004-6007 DAC fond (LFO) {}{}{}{}  = {:X}",
            on(b.lfo[3]),
            on(b.lfo[2]),
            on(b.lfo[1]),
            on(b.lfo[0]),
            b.audio.regs.lfo
        )));
        ui.separator();
        ui.heading("Latch son 6800-6807 (écriture)");
        let names = ["FS1", "FS2", "FS3", "HIT", "—", "FIRE", "VOL1", "VOL2"];
        for (i, n) in names.iter().enumerate() {
            ui.label(mono(format!(
                "{:04X} {n:<5} {}",
                0x6800 + i,
                on(b.sound_latch[i])
            )));
        }
        ui.label(mono(format!("7800 pitch {:02X}", b.pitch)));
        ui.separator();
        ui.heading("Latch 7000-7007 (écriture)");
        ui.label(mono(format!("7001 NMI autorisée {}", on(b.nmi_enabled()))));
        ui.label(mono(format!(
            "7004 étoiles {}   7006 flip X {}   7007 flip Y {}",
            on(b.video.stars_enabled),
            on(b.video.flip_x),
            on(b.video.flip_y)
        )));
        ui.label(mono(format!(
            "origine des étoiles {} (image {})",
            b.video.star_origin, b.video.star_origin_frame
        )));
        ui.label(mono(format!("watchdog déclenché {}", on(b.watchdog_fired))));
    });
}

fn objram_decode(ui: &mut egui::Ui, d: &[u8]) {
    egui::CollapsingHeader::new("Décodage des objets")
        .default_open(false)
        .show(ui, |ui| {
            ui.label(mono(
                "5800-583F colonnes : (défilement, attribut) ×32".into(),
            ));
            let cols: String = (0..32)
                .map(|c| format!("{:02X}/{:02X} ", d[c * 2], d[c * 2 + 1]))
                .collect::<Vec<_>>()
                .chunks(8)
                .map(|c| c.concat())
                .collect::<Vec<_>>()
                .join("\n");
            ui.label(mono(cols));
            ui.label(mono(
                "5840-585F sprites : Y, code(+flips), couleur, X".into(),
            ));
            for s in 0..8 {
                let o = 0x40 + s * 4;
                ui.label(mono(format!(
                    "#{s}  Y {:02X}  code {:02X}{}{}  couleur {:X}  X {:02X}",
                    d[o],
                    d[o + 1] & 0x3f,
                    if d[o + 1] & 0x80 != 0 { " flipY" } else { "" },
                    if d[o + 1] & 0x40 != 0 { " flipX" } else { "" },
                    d[o + 2] & 7,
                    d[o + 3]
                )));
            }
            ui.label(mono("5860-587F projectiles (octets bruts)".into()));
            for s in 0..8 {
                let o = 0x60 + s * 4;
                ui.label(mono(format!(
                    "#{s}  {:02X} {:02X} {:02X} {:02X}",
                    d[o],
                    d[o + 1],
                    d[o + 2],
                    d[o + 3]
                )));
            }
        });
}

fn hex_view(
    ui: &mut egui::Ui,
    base: u32,
    data: &[u8],
    changed: &[u64],
    frame: u64,
    scroll_to: Option<usize>,
) {
    let font = FontId::monospace(13.0);
    let row_h = ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
    let rows = data.len().div_ceil(16);
    let mut area = egui::ScrollArea::vertical()
        .id_salt("hexview")
        .auto_shrink([false, false]);
    if let Some(r) = scroll_to {
        area = area.vertical_scroll_offset(r as f32 * row_h);
    }
    area.show_rows(ui, row_h, rows, |ui, range| {
        for row in range {
            let start = row * 16;
            let end = (start + 16).min(data.len());
            let mut job = LayoutJob::default();
            let fmt = |c: Color32| TextFormat {
                font_id: font.clone(),
                color: c,
                ..Default::default()
            };
            job.append(
                &format!("{:04X}  ", base as usize + start),
                0.0,
                fmt(Color32::from_rgb(120, 170, 255)),
            );
            for (k, &byte) in data[start..end].iter().enumerate() {
                let i = start + k;
                let stamp = changed.get(i).copied().unwrap_or(0);
                let hot = stamp != 0 && frame.saturating_sub(stamp) < HIGHLIGHT_FRAMES;
                let color = if hot {
                    Color32::from_rgb(255, 140, 60)
                } else if byte == 0 {
                    Color32::from_gray(110)
                } else {
                    Color32::LIGHT_GRAY
                };
                job.append(&format!("{byte:02X} "), 0.0, fmt(color));
                if i % 8 == 7 {
                    job.append(" ", 0.0, fmt(color));
                }
            }
            let pad = (16 - (end - start)) * 3;
            job.append(&" ".repeat(pad + 1), 0.0, fmt(Color32::GRAY));
            let ascii: String = data[start..end]
                .iter()
                .map(|&b| {
                    if (0x20..0x7f).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            job.append(&ascii, 0.0, fmt(Color32::from_rgb(140, 200, 140)));
            ui.label(job);
        }
    });
}
