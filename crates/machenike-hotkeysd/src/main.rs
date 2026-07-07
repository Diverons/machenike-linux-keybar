use std::fs::{self, File};
use std::io::Read;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const KBDCTL: &str = "/usr/local/bin/machenike-kbdctl";
const STATE_FILE: &str = "/var/lib/machenike-kbdctl/state";
const CONFIG_FILE: &str = "/etc/machenike/hotkeysd.conf";

const EV_KEY: u16 = 0x01;

const KEY_1: u16 = 2;
const KEY_2: u16 = 3;
const KEY_3: u16 = 4;
const KEY_4: u16 = 5;
const KEY_5: u16 = 6;
const KEY_6: u16 = 7;
const KEY_7: u16 = 8;
const KEY_8: u16 = 9;
const KEY_9: u16 = 10;
const KEY_0: u16 = 11;

const KEY_MINUS: u16 = 12;
const KEY_EQUAL: u16 = 13;

const KEY_Q: u16 = 16;
const KEY_W: u16 = 17;
const KEY_E: u16 = 18;
const KEY_R: u16 = 19;
const KEY_T: u16 = 20;
const KEY_Y: u16 = 21;
const KEY_U: u16 = 22;
const KEY_I: u16 = 23;
const KEY_O: u16 = 24;
const KEY_P: u16 = 25;

const KEY_A: u16 = 30;
const KEY_S: u16 = 31;
const KEY_D: u16 = 32;
const KEY_F: u16 = 33;
const KEY_G: u16 = 34;
const KEY_H: u16 = 35;
const KEY_J: u16 = 36;
const KEY_K: u16 = 37;
const KEY_L: u16 = 38;

const KEY_Z: u16 = 44;
const KEY_X: u16 = 45;
const KEY_C: u16 = 46;
const KEY_V: u16 = 47;
const KEY_B: u16 = 48;
const KEY_N: u16 = 49;
const KEY_M: u16 = 50;

const KEY_SLASH: u16 = 53;
const KEY_KPASTERISK: u16 = 55;
const KEY_SPACE: u16 = 57;
const KEY_KPMINUS: u16 = 74;
const KEY_KPPLUS: u16 = 78;
const KEY_KPSLASH: u16 = 98;

const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_LEFTALT: u16 = 56;
const KEY_LEFTMETA: u16 = 125;

const KEY_RIGHTCTRL: u16 = 97;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_RIGHTALT: u16 = 100;
const KEY_RIGHTMETA: u16 = 126;

#[derive(Debug, Clone)]
struct KeyCombo {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
    key_code: u16,
}

#[derive(Debug, Clone)]
struct Config {
    rgb_hold_delay_ms: u64,
    rgb_step_delay_ms: u64,
    rgb_hue_step: u16,

    key_color: KeyCombo,
    key_toggle: KeyCombo,
    key_brightness_down: KeyCombo,
    key_brightness_up: KeyCombo,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rgb_hold_delay_ms: 300,
            rgb_step_delay_ms: 18,
            rgb_hue_step: 5,

            key_color: parse_combo("ctrl+alt+slash").unwrap(),

            // Main keyboard "*" is usually Shift+8.
            // The alias matcher below also accepts KP*.
            key_toggle: parse_combo("ctrl+alt+8").unwrap(),

            key_brightness_down: parse_combo("ctrl+alt+minus").unwrap(),

            // Main keyboard "+" is usually Shift+=.
            // The alias matcher below also accepts KP+.
            key_brightness_up: parse_combo("ctrl+alt+equal").unwrap(),
        }
    }
}

#[derive(Debug, Default)]
struct Keys {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,

    color_key_down: bool,
    color_cycle_running: bool,
}

fn key_name_to_code(name: &str) -> Option<u16> {
    match name {
        "1" => Some(KEY_1),
        "2" => Some(KEY_2),
        "3" => Some(KEY_3),
        "4" => Some(KEY_4),
        "5" => Some(KEY_5),
        "6" => Some(KEY_6),
        "7" => Some(KEY_7),
        "8" => Some(KEY_8),
        "9" => Some(KEY_9),
        "0" => Some(KEY_0),

        "q" => Some(KEY_Q),
        "w" => Some(KEY_W),
        "e" => Some(KEY_E),
        "r" => Some(KEY_R),
        "t" => Some(KEY_T),
        "y" => Some(KEY_Y),
        "u" => Some(KEY_U),
        "i" => Some(KEY_I),
        "o" => Some(KEY_O),
        "p" => Some(KEY_P),

        "a" => Some(KEY_A),
        "s" => Some(KEY_S),
        "d" => Some(KEY_D),
        "f" => Some(KEY_F),
        "g" => Some(KEY_G),
        "h" => Some(KEY_H),
        "j" => Some(KEY_J),
        "k" => Some(KEY_K),
        "l" => Some(KEY_L),

        "z" => Some(KEY_Z),
        "x" => Some(KEY_X),
        "c" => Some(KEY_C),
        "v" => Some(KEY_V),
        "b" => Some(KEY_B),
        "n" => Some(KEY_N),
        "m" => Some(KEY_M),

        "slash" | "/" => Some(KEY_SLASH),
        "minus" | "-" => Some(KEY_MINUS),
        "equal" | "plus" | "+" => Some(KEY_EQUAL),
        "space" => Some(KEY_SPACE),

        "kpslash" => Some(KEY_KPSLASH),
        "kpasterisk" | "asterisk" | "star" | "*" => Some(KEY_KPASTERISK),
        "kpminus" => Some(KEY_KPMINUS),
        "kpplus" => Some(KEY_KPPLUS),

        _ => {
            if let Some(raw) = name.strip_prefix("code:") {
                return raw.parse::<u16>().ok();
            }

            None
        }
    }
}

fn code_to_label(code: u16) -> String {
    match code {
        KEY_1 => "1".to_string(),
        KEY_2 => "2".to_string(),
        KEY_3 => "3".to_string(),
        KEY_4 => "4".to_string(),
        KEY_5 => "5".to_string(),
        KEY_6 => "6".to_string(),
        KEY_7 => "7".to_string(),
        KEY_8 => "8".to_string(),
        KEY_9 => "9".to_string(),
        KEY_0 => "0".to_string(),

        KEY_Q => "q".to_string(),
        KEY_W => "w".to_string(),
        KEY_E => "e".to_string(),
        KEY_R => "r".to_string(),
        KEY_T => "t".to_string(),
        KEY_Y => "y".to_string(),
        KEY_U => "u".to_string(),
        KEY_I => "i".to_string(),
        KEY_O => "o".to_string(),
        KEY_P => "p".to_string(),

        KEY_A => "a".to_string(),
        KEY_S => "s".to_string(),
        KEY_D => "d".to_string(),
        KEY_F => "f".to_string(),
        KEY_G => "g".to_string(),
        KEY_H => "h".to_string(),
        KEY_J => "j".to_string(),
        KEY_K => "k".to_string(),
        KEY_L => "l".to_string(),

        KEY_Z => "z".to_string(),
        KEY_X => "x".to_string(),
        KEY_C => "c".to_string(),
        KEY_V => "v".to_string(),
        KEY_B => "b".to_string(),
        KEY_N => "n".to_string(),
        KEY_M => "m".to_string(),

        KEY_SLASH => "slash".to_string(),
        KEY_MINUS => "minus".to_string(),
        KEY_EQUAL => "equal".to_string(),
        KEY_SPACE => "space".to_string(),

        KEY_KPSLASH => "kpslash".to_string(),
        KEY_KPASTERISK => "kpasterisk".to_string(),
        KEY_KPMINUS => "kpminus".to_string(),
        KEY_KPPLUS => "kpplus".to_string(),

        _ => format!("code:{code}"),
    }
}

fn parse_combo(input: &str) -> Option<KeyCombo> {
    let mut combo = KeyCombo {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
        key_code: 0,
    };

    for part in input.trim().to_lowercase().split('+') {
        let part = part.trim();

        match part {
            "ctrl" | "control" => combo.ctrl = true,
            "alt" => combo.alt = true,
            "shift" => combo.shift = true,
            "meta" | "super" | "win" => combo.meta = true,
            "" => {}
            key => {
                let code = key_name_to_code(key)?;
                combo.key_code = code;
            }
        }
    }

    if combo.key_code == 0 {
        return None;
    }

    Some(combo)
}

fn combo_to_string(combo: &KeyCombo) -> String {
    let mut parts = Vec::new();

    if combo.ctrl {
        parts.push("ctrl".to_string());
    }
    if combo.alt {
        parts.push("alt".to_string());
    }
    if combo.shift {
        parts.push("shift".to_string());
    }
    if combo.meta {
        parts.push("meta".to_string());
    }

    parts.push(code_to_label(combo.key_code));
    parts.join("+")
}

fn combo_display(combo: &KeyCombo) -> String {
    let mut out = combo_to_string(combo);

    out = out.replace("ctrl", "Ctrl");
    out = out.replace("alt", "Alt");
    out = out.replace("shift", "Shift");
    out = out.replace("meta", "Meta");

    out = out.replace("slash", "/");
    out = out.replace("minus", "-");
    out = out.replace("equal", "+");
    out = out.replace("space", "Space");

    out = out.replace("kpasterisk", "KP*");
    out = out.replace("kpslash", "KP/");
    out = out.replace("kpminus", "KP-");
    out = out.replace("kpplus", "KP+");

    out = out.replace("Ctrl+Alt+8", "Ctrl+Alt+*");

    out
}

fn read_config() -> Config {
    let mut config = Config::default();

    let Ok(content) = fs::read_to_string(CONFIG_FILE) else {
        return config;
    };

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "rgb_hold_delay_ms" => {
                if let Ok(v) = value.parse::<u64>() {
                    config.rgb_hold_delay_ms = v.clamp(0, 5000);
                }
            }

            "rgb_step_delay_ms" => {
                if let Ok(v) = value.parse::<u64>() {
                    config.rgb_step_delay_ms = v.clamp(5, 1000);
                }
            }

            "rgb_hue_step" => {
                if let Ok(v) = value.parse::<u16>() {
                    config.rgb_hue_step = v.clamp(1, 120);
                }
            }

            "key_color" => {
                if let Some(v) = parse_combo(value) {
                    config.key_color = v;
                }
            }

            "key_toggle" => {
                if let Some(v) = parse_combo(value) {
                    config.key_toggle = v;
                }
            }

            "key_brightness_down" => {
                if let Some(v) = parse_combo(value) {
                    config.key_brightness_down = v;
                }
            }

            "key_brightness_up" => {
                if let Some(v) = parse_combo(value) {
                    config.key_brightness_up = v;
                }
            }

            _ => {}
        }
    }

    config
}

fn key_code_matches(actual: u16, expected: u16) -> bool {
    if actual == expected {
        return true;
    }

    match expected {
        // "/" can be main slash or numpad slash.
        KEY_SLASH => actual == KEY_KPSLASH,

        // "-" can be main minus or numpad minus.
        KEY_MINUS => actual == KEY_KPMINUS,

        // "+" on main keyboard is usually Shift+=,
        // but numpad plus is a separate key.
        KEY_EQUAL => actual == KEY_KPPLUS,

        // "*" on main keyboard is usually Shift+8,
        // but numpad star is a separate key.
        KEY_8 => actual == KEY_KPASTERISK,

        // Reverse aliases.
        KEY_KPSLASH => actual == KEY_SLASH,
        KEY_KPMINUS => actual == KEY_MINUS,
        KEY_KPPLUS => actual == KEY_EQUAL,
        KEY_KPASTERISK => actual == KEY_8,

        _ => false,
    }
}

fn combo_matches(keys: &Keys, code: u16, combo: &KeyCombo) -> bool {
    key_code_matches(code, combo.key_code)
        && (!combo.ctrl || keys.ctrl)
        && (!combo.alt || keys.alt)
        && (!combo.shift || keys.shift)
        && (!combo.meta || keys.meta)
}

fn run_kbdctl(args: &[&str]) {
    let result = Command::new(KBDCTL).args(args).status();

    match result {
        Ok(status) if status.success() => {
            println!("machenike-hotkeysd: {}", args.join(" "));
        }
        Ok(status) => {
            eprintln!(
                "machenike-hotkeysd: {KBDCTL} {} exited with {status}",
                args.join(" ")
            );
        }
        Err(error) => {
            eprintln!("machenike-hotkeysd: failed to run {KBDCTL}: {error}");
        }
    }
}

fn hsv_to_rgb(hue: u16) -> (u8, u8, u8) {
    let h = hue % 360;
    let region = h / 60;
    let remainder = h % 60;

    let p = 0u8;
    let q = (255u16 - (255u16 * remainder / 60)) as u8;
    let t = (255u16 * remainder / 60) as u8;
    let v = 255u8;

    match region {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

fn read_current_rgb() -> Option<(u8, u8, u8)> {
    let content = fs::read_to_string(STATE_FILE).ok()?;
    let parts: Vec<&str> = content.split_whitespace().collect();

    if parts.len() < 3 {
        return None;
    }

    Some((
        parts[0].parse::<u8>().ok()?,
        parts[1].parse::<u8>().ok()?,
        parts[2].parse::<u8>().ok()?,
    ))
}

fn rgb_to_hue(r: u8, g: u8, b: u8) -> u16 {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    if delta == 0.0 {
        return 0;
    }

    let mut hue = if max == r {
        60.0 * ((g - b) / delta)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    if hue < 0.0 {
        hue += 360.0;
    }

    hue.round() as u16
}

fn current_hue_or_default() -> u16 {
    read_current_rgb()
        .map(|(r, g, b)| rgb_to_hue(r, g, b))
        .unwrap_or(0)
}

fn start_rgb_cycle_if_held(keys: Arc<Mutex<Keys>>, config: Config) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(config.rgb_hold_delay_ms));

        {
            let mut keys = keys.lock().unwrap();

            if !keys.color_key_down {
                return;
            }

            keys.color_cycle_running = true;
        }

        let mut hue = current_hue_or_default();

        loop {
            {
                let keys = keys.lock().unwrap();

                if !keys.color_key_down {
                    break;
                }
            }

            let (r, g, b) = hsv_to_rgb(hue);

            run_kbdctl(&["rgb", &r.to_string(), &g.to_string(), &b.to_string()]);

            hue = (hue + config.rgb_hue_step) % 360;

            thread::sleep(Duration::from_millis(config.rgb_step_delay_ms));
        }

        let mut keys = keys.lock().unwrap();
        keys.color_cycle_running = false;
    });
}

fn handle_color_key(keys: &Arc<Mutex<Keys>>, config: &Config, value: i32) {
    match value {
        // Press.
        1 => {
            let should_start_thread = {
                let mut keys_locked = keys.lock().unwrap();

                if keys_locked.color_key_down {
                    false
                } else {
                    keys_locked.color_key_down = true;
                    keys_locked.color_cycle_running = false;
                    true
                }
            };

            if should_start_thread {
                start_rgb_cycle_if_held(Arc::clone(keys), config.clone());
            }
        }

        // Release.
        0 => {
            let should_do_single_next = {
                let mut keys_locked = keys.lock().unwrap();

                let was_short_press =
                    keys_locked.color_key_down && !keys_locked.color_cycle_running;

                keys_locked.color_key_down = false;

                was_short_press
            };

            if should_do_single_next {
                run_kbdctl(&["next"]);
            }
        }

        // Repeat. Smooth RGB cycle has its own timer.
        2 => {}

        _ => {}
    }
}

fn update_modifiers(keys: &mut Keys, code: u16, value: i32) -> bool {
    match code {
        KEY_LEFTCTRL | KEY_RIGHTCTRL => {
            keys.ctrl = value != 0;
            true
        }
        KEY_LEFTALT | KEY_RIGHTALT => {
            keys.alt = value != 0;
            true
        }
        KEY_LEFTSHIFT | KEY_RIGHTSHIFT => {
            keys.shift = value != 0;
            true
        }
        KEY_LEFTMETA | KEY_RIGHTMETA => {
            keys.meta = value != 0;
            true
        }
        _ => false,
    }
}

fn handle_key(keys: &Arc<Mutex<Keys>>, config: &Config, code: u16, value: i32) {
    {
        let mut keys_locked = keys.lock().unwrap();

        if update_modifiers(&mut keys_locked, code, value) {
            // Stop RGB-scroll when any modifier from the combo is released.
            if value == 0 {
                keys_locked.color_key_down = false;
                keys_locked.color_cycle_running = false;
            }

            return;
        }
    }

    if value != 0 && value != 1 && value != 2 {
        return;
    }

    let keys_snapshot = keys.lock().unwrap();

    let color_match = combo_matches(&keys_snapshot, code, &config.key_color);
    let toggle_match = combo_matches(&keys_snapshot, code, &config.key_toggle);
    let down_match = combo_matches(&keys_snapshot, code, &config.key_brightness_down);
    let up_match = combo_matches(&keys_snapshot, code, &config.key_brightness_up);

    drop(keys_snapshot);

    if color_match {
        handle_color_key(keys, config, value);
        return;
    }

    if toggle_match && value == 1 {
        run_kbdctl(&["toggle"]);
        return;
    }

    if down_match && (value == 1 || value == 2) {
        run_kbdctl(&["brightness-down"]);
        return;
    }

    if up_match && (value == 1 || value == 2) {
        run_kbdctl(&["brightness-up"]);
    }
}

fn read_input_event(file: &mut File) -> std::io::Result<(u16, u16, i32)> {
    let mut buf = [0u8; 24];
    file.read_exact(&mut buf)?;

    Ok((
        u16::from_ne_bytes([buf[16], buf[17]]),
        u16::from_ne_bytes([buf[18], buf[19]]),
        i32::from_ne_bytes([buf[20], buf[21], buf[22], buf[23]]),
    ))
}

fn watch_device(path: String, keys: Arc<Mutex<Keys>>, config: Config) {
    let Ok(mut file) = File::open(&path) else {
        return;
    };

    println!("machenike-hotkeysd: watching {path}");

    loop {
        match read_input_event(&mut file) {
            Ok((event_type, code, value)) => {
                if event_type == EV_KEY {
                    handle_key(&keys, &config, code, value);
                }
            }
            Err(error) => {
                eprintln!("machenike-hotkeysd: stopped watching {path}: {error}");
                return;
            }
        }
    }
}

fn main() {
    if unsafe { libc_geteuid() } != 0 {
        eprintln!("machenike-hotkeysd must run as root");
        std::process::exit(1);
    }

    let config = read_config();

    println!("machenike-hotkeysd: loaded config");
    println!("  color: {}", combo_display(&config.key_color));
    println!("  toggle: {}", combo_display(&config.key_toggle));
    println!("  brightness down: {}", combo_display(&config.key_brightness_down));
    println!("  brightness up: {}", combo_display(&config.key_brightness_up));
    println!("  rgb_hold_delay_ms: {}", config.rgb_hold_delay_ms);
    println!("  rgb_step_delay_ms: {}", config.rgb_step_delay_ms);
    println!("  rgb_hue_step: {}", config.rgb_hue_step);

    let keys = Arc::new(Mutex::new(Keys::default()));

    let entries = fs::read_dir("/dev/input").unwrap_or_else(|error| {
        eprintln!("failed to read /dev/input: {error}");
        std::process::exit(1);
    });

    let mut handles = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !name.starts_with("event") {
            continue;
        }

        let path = path.to_string_lossy().to_string();
        let keys = Arc::clone(&keys);
        let config = config.clone();

        handles.push(thread::spawn(move || {
            watch_device(path, keys, config);
        }));
    }

    if handles.is_empty() {
        eprintln!("machenike-hotkeysd: no /dev/input/event* devices found");
        std::process::exit(1);
    }

    for handle in handles {
        let _ = handle.join();
    }
}

#[cfg(target_os = "linux")]
unsafe fn libc_geteuid() -> u32 {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }

    geteuid()
}
